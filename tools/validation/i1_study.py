"""Reproduce Stage 08 from the maintained O2 runner's returned, hashed Rust traces."""

import json
import math
import subprocess
from dataclasses import replace
from itertools import combinations, product
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

import numpy as np
from acoustic_identity import FIXTURES, audio, capture, distances
from acoustic_observation import db, partials, rms, tracking
from guide_qualification import digest, trajectory
from i1_clock import OUT, Confirm, Frame, Gesture, Launch, controls, due_stamps, replay
from i1_safety import correction, historical_run, smooth_targets, transfer
from o2_audio import physical_key
from o2_bench import OUT as O2
from o2_coverage import RATE, STATIONARY, keys
from o2_qualification import strongest_hz, v17_row
from o2_records import producer_clock
from o2_response import reference, require
from scipy import signal
from scipy.io import wavfile

ALT = np.tile([-5.0, 5.0], 6)
VECTORS = {
    "opposed": ALT,
    "upper": np.full(12, 5.0),
    "lower": np.full(12, -5.0),
    "inversion": np.r_[-5.0, 5.0, np.zeros(10)],
}


def write(name, obj):
    tmp = OUT / (name + ".tmp")
    tmp.write_text(json.dumps(obj, indent=2, allow_nan=False) + "\n")
    tmp.replace(OUT / name)


def steady_grid():
    grid = {(fs, f, v, r, "opposed") for fs, f, v, r in STATIONARY}
    grid.add((48000, 440, 0.06, 1.8, "opposed"))
    grid.update(
        (48000, 440, v, r, "opposed") for v, r in product((0.06, 0.16), (1.2, 2.4))
    )
    grid.update(
        (48000, f, 0.1, 1.8, name)
        for f, name in product((82.41, 196, 440), ("upper", "lower", "inversion"))
    )
    grid.update(
        (48000, 196, 0.1, 1.8, name)
        for name in ("weak-H1", "missing-six", "dip-six", "bright", "dark")
    )
    return sorted(grid)


def key(case):
    return "/".join(map(str, case))


def carrier(fs, f, v=0.1, r=1.8, kind="steady"):
    name = physical_key(fs, f, v, r, kind)
    path = O2 / "traces" / (name + ".f64le")
    parity = json.loads((O2 / "parity.json").read_text())[name]
    require(digest(path) == parity["sha256"], "Rust O2 trace " + name)
    return np.memmap(path, mode="r", dtype="<f8").reshape(-1, 30)


def source(fs, f, name):
    t = np.arange(fs) / fs
    amp = 1 / np.arange(1, 13, dtype=float)
    if name == "weak-H1":
        amp *= 0.08
        amp[:2] = [0.01, 0.2]
    else:
        amp *= 0.12
        if name in ("missing-six", "dip-six"):
            amp[5] *= 0 if name == "missing-six" else 0.35
        else:
            amp[4:] *= 2.5 if name == "bright" else 0.15
    return sum(a * np.cos(2 * np.pi * k * f * t) for k, a in enumerate(amp, 1))


def capture_log(x, fs, f):
    """A3-confirmed pitch injected from the first frame. Launch before first color."""
    hop = int(fs // 8000) * math.floor(0.005 * (fs / (fs // 8000)) + 0.5)
    raw = np.r_[x[: round(0.30 * fs)], np.zeros(hop * 3)]
    c = math.exp(-2 * math.pi * 30 / fs)
    hp = signal.lfilter([c, -c], [1, -c], raw)
    a = -math.expm1(-1 / (0.004 * fs))
    amplitude = np.sqrt(signal.lfilter([a], [1, a - 1], hp**2))
    frames = {
        t: Frame(f, amplitude=amplitude[t], supported=t < round(0.30 * fs))
        for t in range(hop - 1, len(raw), hop)
    }
    return replay(
        raw,
        fs,
        frames,
        {
            0: (Gesture.PICK,),
            2 * hop - 1: (Confirm((0, 1), len(raw)), Launch((0, 1), 1, 0.0)),
        },
        identity_stamps=due_stamps(len(raw), fs),
    )


def updates_for(x, fs, f):
    rows = capture_log(x, fs, f)
    return [(r["tick"], np.array(r["held"])) for r in rows if r["consumed"]]


def grade_wet(row):
    for gate, flag in row["passes"].items():
        require(flag is None or flag is True, gate)
    require(row["spectral"] is not None, "ineligible wet")


def inspect_row(raw, wet, fs, f, setting):
    q = raw[:, 25:28]
    m5 = raw[:, 11]
    ref, _ = producer_clock(m5, q[:, 0], fs, f, (setting,))
    off = replace(ref, samples=wet[:, 0])
    rows = []
    for i, d in enumerate((0, 0.5, 1)):
        row = v17_row(
            q[:, i],
            q[:, 0],
            raw[:, 22],
            raw[:, 24],
            m5,
            raw[:, 21],
            raw[:, 3],
            fs,
            f,
            d,
            operation="corrected",
            clock=ref,
            settings=(setting,),
            corrected_off=off,
            corrected=replace(ref, samples=wet[:, i], depth=d),
        )
        # Independent RMS from the actual final and reference taps.
        require(
            abs(row["final_over_M5"] - db(rms(wet[-fs:, i]) / rms(m5[-fs:]))) < 1e-10,
            "M5 denominator",
        )
        if i == 1:
            res = db(rms(wet[-fs:, 1] - wet[-fs:, 0]) / rms(wet[-fs:, 0]))
            require(
                abs(row["factory_difference_over_off"] - res) < 1e-10,
                "final-wet residual",
            )
        rows.append(row)
    return rows


def summary(log, scale):
    return {
        "windows": len(log),
        "veto_fraction": sum(x["veto"] for x in log) / len(log),
        "constraint_fraction": sum(x["scale"] < 1 for x in log) / len(log),
        "neutral_sample_fraction": float(np.mean(scale == 0)),
        "upstream_or_uncertified": sum(not x["neutral_ok"] for x in log),
        "realized_veto_failures": sum(not x["realized_ok"] for x in log),
        "settled_scale": float(scale[-1]),
    }


def harmonics(x, fs, f):
    return abs(partials(x[-fs:], fs, f))


def spectrum(h):
    return np.maximum(-50.0, db(h / max(h)))


def cues(h):
    v = spectrum(h)
    return [
        v[5] - (v[4] + v[6]) / 2,
        db(np.linalg.norm(h[4:10]) / np.linalg.norm(h[:4])),
    ]


def study_case(raw, case):
    fs, f, _v, _r, name = case
    if name in VECTORS:
        gains = np.expm1(np.log(10) * VECTORS[name] / 20)
    else:
        updates = updates_for(source(fs, f, name), fs, f)
        gains = smooth_targets(len(raw), fs, updates)
    out, alpha, log = historical_run(raw[:, 25:28].T, raw[:, 11], fs, f, gains)
    final = gains if gains.ndim == 1 else gains[-1]
    spec = inspect_row(raw, out, fs, f, key(case))
    frequencies = np.arange(65.0, 4000.25, 0.25)
    sweep = []
    for d in (0, 0.5, 1):
        h = transfer(fs, f, frequencies, alpha[-1] * final) * reference(
            fs, frequencies, d
        )
        i = np.argmax(abs(h))
        sweep.append([frequencies[i], float(db(abs(h[i])))])
    neutrals = inspect_row(raw, raw[:, 25:28], fs, f, key(case) + "/neutral")
    return {
        "neutral_depths": neutrals,
        "depths": spec,
        "policy": summary(log, alpha),
        "sweep": sweep,
        "requested_dB": (20 * np.log10(1 + final)).tolist(),
        "accepted_dB": (20 * np.log10(1 + alpha[-1] * final)).tolist(),
    }


def swept_output(accepted):
    from o2_bench import replay as electrical_replay
    from o2_qualification import sweep_peaks

    fs = 48000
    t = np.arange(3 * fs) / fs
    x = np.r_[
        0.1 * signal.chirp(t, 65, 3, 4000, method="logarithmic"), np.zeros(fs // 2)
    ]
    hz = np.fft.rfftfreq(len(x), 1 / fs)
    den = np.fft.rfft(x)
    use = (hz >= 65) & (hz <= 4000) & (abs(den) / len(x) > 1e-12)
    results = {}
    for depth in (0, 0.5, 1):
        q = electrical_replay(fs, x, depth, O2 / "render")[:, 3]
        for name in ("weak-H1", "upper", "lower", "inversion", "opposed"):
            gains = np.expm1(
                np.log(10)
                * np.array(accepted[key((fs, 196, 0.1, 1.8, name))]["accepted_dB"])
                / 20
            )
            measured = (
                np.fft.rfft(q + correction(q, fs, 196, gains))[use]
                * math.sqrt(2)
                / (0.85 * den[use])
            )
            peaks = sweep_peaks(hz[use], abs(measured))
            ref = transfer(fs, 196, hz[use], gains) * reference(fs, hz[use], depth)
            error = float(max(abs(measured - ref)))
            peak = max(peaks, key=lambda p: p["dB"])
            require(peak["dB"] <= 6 and error <= 1e-9, "measured frozen sweep")
            results[f"{name}/{depth}"] = {"peak": peak, "complex_error": error}
    return results


def gain_corners():
    from o2_qualification import pluck

    result = {}
    fs = 48000
    direct = pluck(fs, 440, 3 * fs)
    for v, r in product((0.06, 0.16), (1.2, 2.4)):
        raw = carrier(fs, 440, v, r)
        for name in ("neutral", "upper", "lower", "inversion"):
            if name == "neutral":
                out = raw[:, 25:28]
            else:
                gains = np.expm1(np.log(10) * VECTORS[name] / 20)
                out, _, _ = historical_run(raw[:, 25:28].T, raw[:, 11], fs, 440, gains)
            for i, d in enumerate((0, 0.5, 1)):
                wet_peak = float(max(abs(out[:, i])))
                for g, c, trim in product((0.08, 1.4), (0, 1), (0, 2)):
                    pre = trim * ((1 - c) * direct + c * g * out[:, i])
                    peak = float(np.max(abs(pre)))
                    result[key((v, r, name, d, g, c, trim))] = {
                        "peak": peak,
                        "unity_wet_peak": wet_peak,
                        "demand_dB": max(0.0, float(db(peak / 0.98))),
                    }
    return result


def complex_tests():
    rows = {}
    for fs, f in product((32000, 48000, 192000), (82.41, 196, 1318.51)):
        impulse = np.r_[1.0, np.zeros(fs - 1)]
        coeff = 10 ** (ALT / 20) - 1
        h = np.fft.rfft(impulse + correction(impulse, fs, f, coeff))
        ref = transfer(fs, f, np.fft.rfftfreq(fs, 1 / fs), coeff)
        error = float(max(abs(h - ref)))
        require(error <= 1e-9, "overlapping impulse response")
        rows[key((fs, f))] = error
    return rows


def inversion():
    fs = 48000
    raw = carrier(fs, 196)
    g, count = capture(source(fs, 196, "weak-H1"), fs, 196, 0.30)
    q = raw[:, 25]
    wet = q + correction(q, fs, 196, 10 ** (g / 20) - 1)
    f0 = strongest_hz(raw[-fs:, 11], fs)
    ratios = [db(h[1] / h[0]) for h in (harmonics(y, fs, f0) for y in (q, wet))]
    peak = strongest_hz(wet[-fs:], fs)
    require(
        count == 51
        and np.allclose(
            [*ratios, *g[:2]],
            [-1.32479, 9.23725, -4.99148, 4.99148],
            rtol=0,
            atol=0.002,
        )
        and abs(peak - 391.84) < 0.05,
        "original inversion",
    )
    return {"updates": count, "ratios": ratios, "gains": g.tolist(), "peak": peak}


def listening(clips):
    from o2_qualification import archive_entry

    root = OUT / "listening"
    gains = {}
    for name, (fs, x, anchor) in clips.items():
        # Only a scalar differs. DI keeps its native gain and sample rate.
        scale = 0.1 / rms(x[anchor])
        for group, g in (("engineering", 1.0), ("matched", scale)):
            p = root / group / (name + ".wav")
            p.parent.mkdir(exist_ok=True, parents=True)
            wavfile.write(p, fs, (g * x).astype("f4"))
        gains[name] = scale
    with ZipFile(OUT / "listening.zip", "w", ZIP_DEFLATED) as z:
        for name in clips:
            for group in ("engineering", "matched"):
                rel = Path(group) / (name + ".wav")
                alias = rel
                if group == "matched" and name in (
                    "techs-P1-60-held",
                    "techs-P2-60-held",
                ):
                    alias = Path(group) / (
                        "held-B.wav" if "P1" in name else "held-A.wav"
                    )
                archive_entry(z, alias, (root / rel).read_bytes())
        archive_entry(z, "listening-gains.json", json.dumps(gains, indent=2))
        archive_entry(z, "ATTRIBUTION.md", (FIXTURES / "ATTRIBUTION.md").read_bytes())
        archive_entry(
            z,
            "LISTEN.txt",
            "Compare the two G3 DI attacks to the held tones at fixed playback gain. Which source matches each held tone, and by which cue? Compare the matched neutral tone. Hear the bend and the monitor fallback. Record route, timestamp, cue and indistinguishable outcomes. Engineering files retain unity gain. No listening verdict recorded.\n",
        )
    write("listening-key.json", {"held-A": "techs-P2-60", "held-B": "techs-P1-60"})
    return gains


def real_sources(clips):
    from i1_repeatability import representation

    fs = 48000
    raw = carrier(fs, 196)
    q = raw[:, 25]
    m5 = raw[:, 11]
    f0 = strongest_hz(m5[-fs:], fs)
    records = {}
    spectra = {}
    for player, passage in product((1, 2), (60, 132, 204)):
        name = f"techs-P{player}-{passage}"
        rate, x = audio(FIXTURES / (name + ".wav"))
        onset = np.flatnonzero(abs(x) > 0.01)[0]
        x = x[onset:]
        updates = updates_for(x, rate, 196)
        require(bool(updates), "missing source capture " + name)
        times = [(round(t * fs / rate), g) for t, g in updates]
        coeff = smooth_targets(len(q), fs, times)
        wet, alpha, log = historical_run(raw[:, 25:28].T, m5, fs, 196, coeff)
        unsafe = q + correction(q, fs, 196, coeff)
        di = x[round(0.08 * rate) : round(0.23 * rate)]
        pitchgrid = np.arange(193, 199.01, 0.025)
        pitch = pitchgrid[
            np.argmax([abs(partials(di, rate, hz, 1)[0]) for hz in pitchgrid])
        ]
        h = abs(partials(di, rate, pitch))
        spectra[name] = {
            "di": h,
            "unsafe": harmonics(unsafe, fs, f0),
            "neutral": harmonics(q, fs, f0),
            "safe": harmonics(wet[:, 0], fs, f0),
            "requested": updates[-1][1],
            "accepted": 20 * np.log10(1 + alpha[-1] * coeff[-1]),
        }
        historical = x * 0.08 / rms(x[round(0.06 * rate) : round(0.22 * rate)])
        oldgain, count = capture(historical, rate, 196, 0.30)
        oldwet = q + correction(q, fs, 196, 10 ** (oldgain / 20) - 1)
        spectra[name]["old_wet"] = harmonics(oldwet, fs, 196)
        spectra[name]["old_di"] = abs(
            partials(historical[round(0.08 * rate) : round(0.23 * rate)], rate, 196)
        )
        records[name] = {
            "historical_updates": count,
            "source_features": {
                k: representation(spectra[name][k])
                for k in ("di", "safe", "unsafe", "neutral")
            },
            "pitch": pitch,
            "onset": int(onset),
            "policy": summary(log, alpha),
            "valid_publications": len(updates) - 1,
            "bands_dB": [
                db(np.linalg.norm(h[a:b]) / np.linalg.norm(h))
                for a, b in ((0, 2), (2, 4), (4, 8), (8, 12))
            ],
            "depths": inspect_row(raw, wet, fs, 196, name),
        }
        if passage == 60:
            clips[name + "-DI"] = (
                rate,
                x[: round(0.4 * rate)],
                slice(round(0.06 * rate), round(0.22 * rate)),
            )
            clips[name + "-held"] = (fs, wet[-fs:, 0], slice(None))
            if player == 1:
                actions = capture_log(x, rate, 196)
                # Join the consumed capture events to the actual per-sample safety state.
                for action in actions:
                    n = action["tick"]
                    finished = [r for r in log if r["end"] + 17 <= n]
                    action.update(
                        applied_scale=float(alpha[n]),
                        veto=finished[-1]["veto"] if finished else True,
                        wet_FS=float(wet[n, 0]),
                        carrier_FS=float(q[n]),
                    )
                write("source-clock.json", actions)
    clips["neutral-held"] = (fs, q[-fs:], slice(None))
    pairs = {}
    for a, b in combinations(spectra, 2):
        fields = {}
        for k in spectra[a]:
            ha, hb = spectra[a][k], spectra[b][k]
            if k in ("requested", "accepted"):
                fields[k] = rms(ha - hb)
            else:
                fields[k] = distances(ha, hb)
                fields[k + "_signed"] = (np.array(cues(ha)) - cues(hb)).tolist()
        pairs[a + "/" + b] = fields
    return {"takes": records, "pairs": pairs}


def dynamic(clips):
    fs = 48000
    data = {}
    for kind in (
        "bend",
        "vibrato",
        "retune",
        "gain-veto",
        "lift",
        "long-hold",
        "bow-controls",
    ):
        seconds = 12 if kind == "long-hold" else 3
        if kind in ("retune", "long-hold", "bow-controls"):
            path = OUT / (kind + ".f64le")
            n = seconds * fs
            freq = np.full(n, 196.0)
            if kind == "retune":
                freq[fs : fs + fs // 50] = np.linspace(196, 440, fs // 50)
                freq[fs + fs // 50 :] = 440
            ctl = np.column_stack((freq, np.full(n, 0.1), np.full(n, 0.18 / 1.65)))
            if kind == "bow-controls":
                # Original independent controller endpoints and linear prescribed ramps.
                ctl[:, 1] = np.interp(
                    np.arange(n) / fs,
                    [0, 0.8, 0.85, 1.6, 1.65, 3],
                    [0.06, 0.06, 0.16, 0.16, 0.10, 0.10],
                )
                ratio = np.interp(
                    np.arange(n) / fs,
                    [0, 1, 1.05, 2, 2.05, 3],
                    [1.2, 1.2, 2.4, 2.4, 1.8, 1.8],
                )
                ctl[:, 2] = ratio * ctl[:, 1] / 1.65
            with path.open("wb") as stream:
                cp = subprocess.run(
                    [O2 / "render", "--o2-guide", str(fs)],
                    input=ctl.astype("<f8").tobytes(),
                    stdout=stream,
                    stderr=subprocess.PIPE,
                    check=True,
                )
            require(
                list(map(int, cp.stderr.split())) == [n, 1, 0, 1, n, n], "guide clock"
            )
            raw = np.memmap(path, mode="r", dtype="<f8").reshape(n, 30)
        else:
            base = 82.41 if kind == "lift" else 196
            raw = carrier(fs, base, kind="steady" if kind == "gain-veto" else kind)
            freq = base * np.exp2(trajectory(np.arange(len(raw)) / fs, kind) / 1200)
        coeff = 10 ** (ALT / 20) - 1
        if kind == "gain-veto":
            coeff = smooth_targets(len(raw), fs, [(0, ALT), (fs, -ALT), (2 * fs, ALT)])
        forced = [(round(1.35 * fs), round(1.55 * fs))] if kind == "gain-veto" else []
        out, alpha, log = historical_run(
            raw[:, 25:28].T, raw[:, 11], fs, freq, coeff, forced
        )
        row = {
            "policy": summary(log, alpha),
            "peak": float(np.max(abs(out))),
            "delta_rms": rms(out[:, 0] - raw[:, 25]),
            "timeline": log,
            "sha256": digest(OUT / (kind + ".f64le"))
            if kind in ("long-hold", "retune", "bow-controls")
            else None,
        }
        if kind in ("bend", "vibrato"):
            row["tracking"] = tracking(out[:, 0], fs, 196, kind)
        if kind == "lift":
            # Baseline M6 scheduled release, ramp from unity for 100 ms then dispose.
            e = np.clip(1 - (np.arange(len(out)) - fs + 1) / round(0.1 * fs), 0, 1)
            release = e * out[:, 0]
            row["released_peak_after_zero"] = float(
                max(abs(release[fs + round(0.1 * fs) :]))
            )
            row["before_after"] = [
                rms(out[fs // 2 : fs, 0]),
                rms(out[6 * fs : 7 * fs, 0]),
            ]
        if kind in ("bend", "gain-veto"):
            clips[kind] = (fs, out[: 2 * fs, 0], slice(fs // 2, fs))
        if kind == "gain-veto":
            row["veto_recovered"] = bool(
                max(alpha[fs // 2 : fs]) > 0
                and min(alpha[round(1.45 * fs) : round(1.55 * fs)]) == 0
                and max(alpha[2 * fs :]) > 0
            )
        data[kind] = row
    return data


def grade(result):
    failures = []
    rows = result["stationary"]
    keys(rows, map(key, steady_grid()), "stationary")
    for name, entry in rows.items():
        require(len(entry["depths"]) == 3, "depth inventory")
        for i, r in enumerate(entry["depths"]):
            failures.extend(
                f"{name}/{i}/{g}" for g, val in r["passes"].items() if val is False
            )
        if max(p[1] for p in entry["sweep"]) > 6:
            failures.append(name + "/narrow peak")

    def find(f, v, r, vector="opposed"):
        return rows[key((48000, f, v, r, vector))]["depths"][0]

    neutral = lambda v, r: carrier(48000, 440, v, r)[-48000:, 25]
    from o2_qualification import fft_metrics

    pressure = lambda row: row["spectral"]["high_fraction"]
    p0, p1 = (find(440, 0.1, r) for r in (1.2, 2.4))
    pd = pressure(p1) - pressure(p0)
    neutral_pd = (
        fft_metrics(neutral(0.1, 2.4), 48000, p1["measured_f0"])["high_fraction"]
        - fft_metrics(neutral(0.1, 1.2), 48000, p0["measured_f0"])["high_fraction"]
    )
    speed = db(find(440, 0.16, 1.8)["rms_FS"] / find(440, 0.06, 1.8)["rms_FS"])
    neutral_speed = db(rms(neutral(0.16, 1.8)) / rms(neutral(0.06, 1.8)))
    identity = distances(
        *[
            np.array(find(196, 0.1, 1.8, v)["harmonics"]["peak_FS"])
            for v in ("bright", "dark")
        ]
    )
    if pd * neutral_pd <= 0 or abs(pd) < 0.5 * abs(neutral_pd):
        failures.append("pressure contrast")
    if speed < 0.75 * neutral_speed:
        failures.append("speed contrast")
    if identity < 1:
        failures.append("neutral-only or insufficient source transfer")
    spread = []
    for depth in range(3):
        rms_values = [
            rows[key((fs, 1318.51, 0.1, 1.8, "opposed"))]["depths"][depth]["rms_FS"]
            for fs in RATE
        ]
        spread.append(db(max(rms_values) / min(rms_values)))
    if max(spread) >= 3:
        failures.append("V20 final level spread")
    dyn = result["dynamic"]
    keys(
        dyn,
        ("bend", "vibrato", "retune", "gain-veto", "lift", "long-hold", "bow-controls"),
        "dynamic",
    )
    if (
        not dyn["gain-veto"]["veto_recovered"]
        or dyn["lift"]["before_after"][1] >= dyn["lift"]["before_after"][0]
    ):
        failures.append("dynamic recovery or decay")
    for name, row in result["real"]["takes"].items():
        for d in row["depths"]:
            try:
                grade_wet(d)
            except ValueError as e:
                failures.append(name + "/" + str(e))
    return failures, {
        "pressure": [neutral_pd, pd],
        "speed_dB": [neutral_speed, speed],
        "high_note_spread_dB": spread,
        "synthetic_distance_dB": identity,
    }


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    result = {
        "passed": False,
        "controlled_identity": "pending",
        "stationary": {},
        "head": subprocess.check_output(
            ["git", "log", "-1", "--format=%H", "--", "tools/validation"], text=True
        ).strip(),
    }
    write("i1-results.json", {"passed": False, "error": "qualification incomplete"})
    try:
        clock = controls()
        write("clock.json", clock)
        result["clock"] = {
            "sha256": digest(OUT / "clock.json"),
            "negatives": clock["negatives"],
        }
        result["complex"] = complex_tests()
        result["inversion"] = inversion()
        for case in steady_grid():
            fs, f, v, r, _name = case
            result["stationary"][key(case)] = study_case(carrier(fs, f, v, r), case)
        result["corners"] = gain_corners()
        result["measured_sweep"] = swept_output(result["stationary"])
        clips = {}
        result["real"] = real_sources(clips)
        result["dynamic"] = dynamic(clips)
        result["listening_gains"] = listening(clips)
        result["failures"], result["expression"] = grade(result)
        from i1_acceptance import audit_structure, negative_controls

        result["negatives"] = negative_controls(result)
        audit_structure(result)
        json.dumps(result, allow_nan=False)
        result["passed"] = not result["failures"]
    except Exception as err:
        result["error"] = str(err)
        raise
    finally:
        try:
            write("i1-results.json", result)
        except (TypeError, ValueError) as err:
            write(
                "i1-results.json",
                {
                    "passed": False,
                    "error": f"invalid evidence: {err}",
                    "completed_cases": list(result["stationary"]),
                },
            )
            raise
    print("Stage 08", result["expression"], result["failures"])
    require(result["passed"], "I1 engineering predicates failed")


if __name__ == "__main__":
    import sys

    if sys.argv[1:] == ["--historical-measurements"]:
        main()
    elif len(sys.argv) == 1:
        from i1_qualification import run as qualify

        qualify()
    else:
        raise SystemExit(
            "use --historical-measurements or run the complete qualification"
        )
