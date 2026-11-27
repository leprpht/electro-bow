"""Independent convolution oracle and electrical measurements of the private Rust lane."""

import json
import math
import subprocess
from pathlib import Path

import numpy as np
from o2_coverage import UniqueRows
from o2_response import ROOT2, check_transfer, reference, require
from scipy import optimize, signal

OUT = Path("build/stage-07")
BINARY = OUT / "render"
DEPTHS = (0, 0.5, 1)
PROBES = (0, 20, 82.41, 196, 440, 1000, 3200, 6000, 12000)


def run(mode, fs, *args):
    return subprocess.check_output(
        [BINARY, "--o2-" + mode, str(fs), *map(str, args)], text=True
    )


def replay(fs, v, d, binary=BINARY):
    payload = np.column_stack((v, np.broadcast_to(d, v.shape))).astype("<f8")
    result = subprocess.run(
        [binary, "--o2-taps", str(fs)],
        input=payload.tobytes(),
        stdout=subprocess.PIPE,
        check=True,
    )
    return np.frombuffer(result.stdout, "<f8").reshape(len(v), 8)


def oracle(fs, v):
    # Chronological first differences, then an exponential integrator.
    c = math.exp(-2 * math.pi * 20 / fs)
    h = signal.lfilter([c], [1, -c], np.diff(v, prepend=0.0))
    a = 0.85 * h / ROOT2
    # Map one conjugate analog pole and normalize the double zero at -1 for
    # unity DC. Convolve with the closed-form real impulse of the pole pair.
    p = np.exp(3j * np.pi / 4) * np.tan(np.pi * 3200 / fs)
    p = (1 + p) / (1 - p)
    n = np.arange(1, 1 + min(len(a), math.ceil(math.log(1e-22) / math.log(abs(p)))))
    kernel = np.convolve((p**n).imag / p.imag, [1, 2, 1]) * (abs(1 - p) ** 2 / 4)
    loaded = signal.oaconvolve(a, kernel)[: len(v)]
    return h, a, loaded


def compare(fs, name, v, d, actual, states, expected=None):
    h, a, l = oracle(fs, v) if expected is None else expected
    wanted = np.column_stack((h, a, l, (1 - 0.025 * d) * l + (0.025 * d) * a))
    errors = abs(actual - wanted)
    bad = np.argwhere(~(errors <= 1e-9))
    if len(bad):
        i, j = bad[0]
        old = states[i - 1] if i else [0.0] * 4
        depth = np.broadcast_to(d, v.shape)[i]
        raise ValueError(
            f"rate={fs} fixture={name} tick={i} v={v[i]} detail={depth} "
            f"previous={old} tap={'halq'[j]} expected={wanted[i, j]} actual={actual[i, j]}"
        )
    return errors.max(axis=0).tolist()


def check_rate(fs):
    coeff = json.loads(run("coeff", fs))
    c, b0, b1, b2, a1, a2 = coeff
    poles = abs(np.roots([1, a1, a2]))
    require(np.isfinite(coeff).all() and 0 < c < 1 and max(poles) < 1, "poles")
    require(b1 == 2 * b0 and b2 == b0, "load Nyquist zeros")
    n = 2 ** math.ceil(math.log2(0.5 * fs))
    v = np.zeros(n)
    v[0] = 1
    freq = np.fft.rfftfreq(n, 1 / fs)
    times = np.arange(n) / fs
    response, differential, steps = UniqueRows(), UniqueRows(), UniqueRows()

    def lp(f):
        w = np.exp(-2j * np.pi * f / fs)
        return (b0 + b1 * w + b2 * w * w) / (1 + a1 * w + a2 * w * w)

    # Recover load gain from the two actual Rust impulse taps.
    unit = replay(fs, v, 0)

    def measured_load(f):
        phasors = np.exp(-2j * np.pi * f * times)
        return abs((phasors @ unit[:, 2]) / (phasors @ unit[:, 1]))

    cutoff = optimize.brentq(lambda f: measured_load(f) - 1 / ROOT2, 1000, 6000)
    require(abs(cutoff - 3200) < 1e-6 and abs(lp(0) - 1) <= 1e-9, "load cutoff/DC")
    for depth in DEPTHS:
        taps = unit if depth == 0 else replay(fs, v, depth)
        differential[f"impulse/{depth}"] = compare(
            fs, "impulse", v, depth, taps[:, :4], taps[:, 4:]
        )
        q = taps[:, 3] * ROOT2 / 0.85
        error = check_transfer(np.fft.rfft(q), reference(fs, freq, depth))
        tail = np.max(abs(taps[-1]))
        require(tail < 1e-10, "impulse decay")
        points = []
        for hz in (*PROBES, fs / 2):
            spectrum = np.exp(-2j * np.pi * hz * times) * q
            measured = spectrum.sum()
            check_transfer(measured, reference(fs, hz, depth))
            eligible = hz != 0 and (depth != 0 or hz != fs / 2)
            delay = delta = phase = None
            if eligible:
                delay = float(((times @ spectrum) / measured).real)
                ref_delay = -np.angle(
                    reference(fs, hz + 0.001, depth) / reference(fs, hz - 0.001, depth)
                ) / (0.002 * 2 * np.pi)
                delta = abs(delay - ref_delay)
                require(delta < 1e-8, "eligible delay")
                phase = np.angle(measured, deg=True)
            points.append(
                {
                    "hz": hz,
                    "real": measured.real,
                    "imag": measured.imag,
                    "load_magnitude": abs(lp(hz)),
                    "phase_deg": phase,
                    "delay_s": delay,
                    "delay_error_s": delta,
                }
            )
        response[str(depth)] = {
            "max_complex_error": error,
            "terminal": tail,
            "points": points,
        }
        values = list(map(float, run("step", fs, depth).split()))
        steps[str(depth)] = {
            "frames": int(values[0]),
            "first": values[1],
            "peak": values[2],
            "max_after_one_second": values[3],
            "terminal": values[4:],
        }
        t = np.arange(4096) / fs
        for label, x in [
            ("constant", np.full(len(t), -0.375)),
            ("step", np.where(t > 0.01, 1.0, 0.0)),
            ("sine", np.sin(2 * np.pi * 196 * t)),
            (
                "multitone",
                np.cos(2 * np.pi * 1318.51 * t) + 0.45 * np.sin(2 * np.pi * 6151 * t),
            ),
        ]:
            taps = replay(fs, x, depth)
            differential[f"{label}/{depth}"] = compare(
                fs, label, x, depth, taps[:, :4], taps[:, 4:]
            )
    return {
        "coefficients": coeff,
        "poles": poles.tolist(),
        "cutoff_hz": cutoff,
        "response": response,
        "differential": differential,
        "long_step": steps,
        "impulse_frames": n,
    }


def timing():
    from statistics import median

    widths = list(map(int, run("size", 48000).split()))
    require(widths == [48, 32], "Rust coefficient/lane size")
    batches = {}
    for fs in (32000, 48000, 192000):
        raw = run("timing", fs)
        (OUT / f"timing-{fs}.tsv").write_text(raw)
        groups = {
            k: [] for k in ("silence", "near-denormal", "o2", "guide-o1", "guide-o1-o2")
        }
        for line in raw.splitlines():
            label, batch, ns, h = line.split()
            require(
                int(batch) == len(groups[label]) + 1 and float(ns) > 0, "timing batch"
            )
            groups[label].append(float(ns))
            if label == "near-denormal":
                require(
                    math.isfinite(float(h)) and abs(float(h)) < np.finfo(float).tiny,
                    "subnormal workload",
                )
        require(all(len(values) == 5 for values in groups.values()), "timing repeats")
        ratio = median(groups["near-denormal"]) / median(groups["silence"])
        require(ratio <= 2, "near-denormal runtime relative to silence")
        batches[str(fs)] = {"ns_per_tick": groups, "near_denormal_ratio": ratio}
    return {"coefficient_bytes": widths[0], "lane_bytes": widths[1], "rates": batches}
