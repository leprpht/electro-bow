"""Independent offline A1 and direct-window difference measurements. Run from repository root."""

import csv
import hashlib
import struct
import subprocess
from functools import cache
from math import ceil, floor
from pathlib import Path

import numpy as np
import soundfile as sf
from numpy.lib.stride_tricks import sliding_window_view
from scipy.signal import lfilter, upfirdn

OUT = Path("build/spec-signoff")
AUDIO = Path("ThirdParty/q/test/audio_files")
RATES = (32000, 44100, 48000, 88200, 96000, 176400, 192000)
NOTES = (65.41, 82.41, 110, 196, 329.63, 440, 880, 1318.51)
SPECTRA = {
    "sine": [1.0],
    "six": [1 / k for k in range(1, 7)],
    "pluck": [1 / k for k in range(1, 9)],
    "weak": [0.05, 1, 0.6, 0.35],
    "second": [0.2, 1, 0.1, 0.25],
    "bright": [1.0] * 6,
}


def noise(n, rms):
    v = 1
    values = np.empty(n)
    for i in range(n):
        v ^= (v << 13) & 0xFFFFFFFF
        v ^= v >> 17
        v ^= (v << 5) & 0xFFFFFFFF
        values[i] = 2 * v / 0xFFFFFFFF - 1
    return values * (rms / np.sqrt(np.mean(values**2)))


def tone(fs, f, shape, peak, seconds):
    t = np.arange(round(fs * seconds)) / fs
    if np.ndim(f):
        phase = 2 * np.pi * np.r_[0.0, np.cumsum(f[:-1])] / fs
    else:
        phase = 2 * np.pi * f * t
    x = np.zeros(len(t))
    for k, weight in enumerate(SPECTRA[shape], 1):
        angle = 0.13 * k * k
        if shape == "bright":
            angle = 0.0
        if shape == "second":
            angle = 0.7 * k
        x += weight * np.sin(k * phase + angle)
    if shape == "pluck":
        x *= -np.expm1(-t / 0.001) * np.exp(-t / 0.28)
    return x * (peak / np.max(abs(x)))


@cache
def fir(dec):
    j = np.arange(32 * dec + 1)
    taps = np.blackman(len(j)) * np.sinc(0.68 * (j - 16 * dec) / dec)
    return taps / taps.sum()


def differences(z, ends, w, k):
    spans = sliding_window_view(z, w)
    lags = np.arange(k + 1)
    rows = np.empty((len(ends), k + 1))
    for i in range(0, len(ends), 16):
        indices = ends[i : i + 16, None] - w + 1 - lags
        window = spans[indices]
        rows[i : i + 16] = np.sum((window[:, :1] - window) ** 2, axis=2)
    return rows


def check_recurrence(z, w, k, h):
    # Independent samplewise A2 recurrence, rotating exact rebase before decisions.
    history = np.r_[np.zeros(w + k), z]
    lag = np.arange(k + 1)
    running = np.zeros(k + 1)
    cursor = 1
    max_error = 0.0
    for n in range(len(z)):
        end = n + w + k
        running += (history[end] - history[end - lag]) ** 2 - (
            history[end - w] - history[end - w - lag]
        ) ** 2
        if (n + 1) % h == 0:
            exact = differences(history, np.array([end]), w, k)[0]
            max_error = max(max_error, np.max(abs(exact - running)))
            running[cursor] = exact[cursor]
            cursor = 1 + cursor % k
    assert max_error < 1e-8
    return max_error


def run_case(
    proc,
    writer,
    group,
    fs,
    x,
    f=None,
    shape="",
    peak="",
    window=(0.0, None),
    level=0.001,
    check=False,
    preroll=True,
):
    pre = round(0.1 * fs) if preroll else 0
    signal = np.r_[np.zeros(pre), x]
    digest = hashlib.sha256(signal.astype("<f8").tobytes()).hexdigest()
    dec = fs // 8000
    fa = fs / dec
    w, h = ceil(fa / 65), floor(0.005 * fa + 0.5)
    k, low = w + 1, max(2, floor(fa / 1320))
    hp = np.exp(-2 * np.pi * 30 / fs)
    raw = lfilter([hp, -hp], [1, -hp], signal)
    # Prefix one zero so decimated indices D,2D,... correspond to D-1,2D-1,...
    z = upfirdn(fir(dec), np.r_[0.0, raw], down=dec)[1 : len(raw) // dec + 1]
    alpha = -np.expm1(-1 / (0.004 * fs))
    fast = np.sqrt(lfilter([alpha], [1, alpha - 1], raw**2))
    ends = np.arange(h - 1, len(z), h)
    host = (ends + 1) * dec - 1
    score_from = pre + round(window[0] * fs)
    score_until = len(signal) if window[1] is None else pre + round(window[1] * fs)
    valid = (ends + 1 >= w + k) & (host >= score_from) & (host < score_until)
    ends, host = ends[valid], host[valid]
    above = fast[host] >= level
    rows = differences(z, ends, w, k)
    if group == "counter" or (
        group == "harmonics" and shape == "second" and peak == 0.35
    ):
        diff = rows[0]
        cm = np.r_[1, diff[1:] * np.arange(1, k + 1) / np.cumsum(diff[1:])]
        for lag in (6, 13) if group == "counter" else (36, 73):
            a, b, c = cm[lag - 1 : lag + 2]
            curvature = a - 2 * b + c
            u = np.clip(0.5 * (a - c) / curvature, -0.5, 0.5)
            depth = np.clip(b + 0.5 * u * (c - a + u * curvature), 0, 1)
            print("trough", fs, f, ends[0], host[0], lag, b, depth)
    if check:
        np.testing.assert_allclose(
            z, lfilter(fir(dec), [1], raw)[dec - 1 :: dec], atol=1e-13, rtol=0
        )
        print("direct FIR and recurrence", group, fs, check_recurrence(z, w, k, h))
    proc.stdin.write(struct.pack("<III", len(ends), low, k))
    proc.stdin.write(rows.astype("<f8", copy=False).tobytes())
    proc.stdin.flush()
    result = np.frombuffer(proc.stdout.read(72 * len(rows)), "<f8").reshape(-1, 3, 3)
    for index, method in enumerate(("integer", "fitted", "deepest")):
        for mode, column in (("start", 0), ("follow", 1)):
            freq = result[:, index, column] * fa
            usable = above & (freq >= 0.995 * 65) & (freq <= 1.005 * 1320)
            freq = np.clip(freq[usable], 65, 1320)
            times = host[usable] - pre
            data = {
                "group": group,
                "fs": fs,
                "shape": shape,
                "peak": peak,
                "f": f if np.isscalar(f) else "",
                "method": method,
                "mode": mode,
                "sha256": digest,
                "decimation": dec,
                "analysis_hz": fa,
                "W": w,
                "K": k,
                "H": h,
                "stimulus_start": pre,
                "score_from": score_from,
                "score_until": score_until,
                "window": len(rows),
                "eligible": int(sum(above)),
                "accepted": len(freq),
                "rejected": np.count_nonzero(above) - len(freq),
            }
            if len(freq):
                data["median_hz"] = np.median(freq)
                data["first_ms"] = times[0] * 1000 / fs
            if f is not None and len(freq):
                truth = f if np.isscalar(f) else f[times]
                error = 1200 * np.log2(freq / truth)
                data.update(
                    median=np.median(abs(error)),
                    p95=np.percentile(abs(error), 95),
                    octave=int(np.sum(abs(error) >= 600)),
                    rms=np.sqrt(np.mean(error**2)),
                )
                if not np.isscalar(f):
                    measured = 1200 * np.log2(freq)
                    alignments = []
                    for ms in range(46):
                        reference = 1200 * np.log2(
                            f[np.maximum(0, times - round(ms * fs / 1000))]
                        )
                        correlation = np.corrcoef(measured, reference)[0, 1]
                        residual = np.sqrt(np.mean((measured - reference) ** 2))
                        alignments.append((correlation, residual))
                    lag = int(np.argmax([correlation for correlation, _ in alignments]))
                    data.update(lag_ms=lag, aligned_rms=alignments[lag][1])
                    if shape.startswith("vib"):
                        # Estimate modulation frequency/depth independently of the known 5 Hz.
                        seconds = times / fs
                        fits = []
                        for rate in np.arange(3, 8.01, 0.01):
                            phase = 2 * np.pi * seconds * rate
                            basis = np.column_stack(
                                (np.ones(len(seconds)), np.sin(phase), np.cos(phase))
                            )
                            coef = np.linalg.lstsq(basis, measured, rcond=None)[0]
                            fits.append(
                                (
                                    np.linalg.norm(basis @ coef - measured),
                                    rate,
                                    np.hypot(*coef[1:]),
                                )
                            )
                        _, rate, depth = min(fits)
                        data.update(modulation_hz=rate, depth_cents=depth)
            writer.writerow(data)


def study(proc, writer):
    for fs in RATES:
        for f in NOTES:
            for shape in ("sine", "six", "pluck"):
                for peak in (0.03, 0.35):
                    start = 0.04 if shape == "pluck" else 1.0
                    run_case(
                        proc,
                        writer,
                        "named",
                        fs,
                        tone(fs, f, shape, peak, 2),
                        f,
                        shape,
                        peak,
                        (start, None),
                    )
    for fs, f in ((48000, 1230.77), (34300, 1318.51)):
        run_case(
            proc,
            writer,
            "counter",
            fs,
            tone(fs, f, "six", 0.35, 2),
            f,
            "six",
            0.35,
            (1, None),
            check=True,
        )
    for fs in (*RATES, 34300, 39999, 44150):
        fa = fs / (fs // 8000)
        # 65 and 1320 included explicitly, four fractional offsets for every legal integer part.
        notes = {65.0, 1320.0} | {
            fa / (m + u)
            for m in range(floor(fa / 1320), ceil(fa / 65) + 1)
            for u in (0, 0.25, 0.5, 0.75)
            if 65 <= fa / (m + u) <= 1320
        }
        for f in sorted(notes):
            for shape in (
                ("six", "second", "weak", "bright") if fs == 48000 else ("six",)
            ):
                run_case(
                    proc,
                    writer,
                    "sweep",
                    fs,
                    tone(fs, f, shape, 0.35, 0.12),
                    f,
                    shape,
                    0.35,
                    (0.08, None),
                )
    fs = 48000
    for peak in (0.003, 0.03, 0.35):
        for shape in ("weak", "second", "bright"):
            run_case(
                proc,
                writer,
                "harmonics",
                fs,
                tone(fs, 110, shape, peak, 2),
                110,
                shape,
                peak,
                (1, None),
            )
    for with_noise in (False, True):
        x = tone(fs, 82.41, "pluck", 0.35, 2)
        if with_noise:
            x[:384] += noise(384, 0.1)
        run_case(
            proc,
            writer,
            "onset",
            fs,
            x,
            82.41,
            "noisy" if with_noise else "clean",
            0.35,
            (0.033, None),
        )
    t = np.arange(3 * fs) / fs
    for shape, f in (
        ("bend", 196 * 2 ** (np.maximum(0, 1 - abs(t - 1.2) / 0.2) / 6)),
        ("vib196", 196 * 2 ** (35 * np.sin(2 * np.pi * 5 * t) / 1200)),
        ("vib440", 440 * 2 ** (35 * np.sin(2 * np.pi * 5 * t) / 1200)),
    ):
        run_case(
            proc,
            writer,
            "trajectory",
            fs,
            tone(fs, f, "six", 0.35, 3),
            f,
            shape,
            0.35,
            (1, 1.45 if shape == "bend" else 3),
        )
    for db in (-65, -30):
        run_case(
            proc,
            writer,
            "unpitched",
            fs,
            noise(10 * fs, 10 ** (db / 20)),
            shape=f"noise{db}",
            level=0,
            check=(db == -30),
        )
    for hz in (50, 60):
        x = (
            np.sin(2 * np.pi * hz * np.arange(10 * fs) / fs)
            * np.sqrt(2)
            * 10 ** (-50 / 20)
        )
        run_case(proc, writer, "unpitched", fs, x, shape=f"hum{hz}", level=0)
    run_case(proc, writer, "unpitched", fs, np.zeros(10 * fs), shape="silence", level=0)
    x = np.zeros(fs)
    x[:96] = noise(96, 0.2)
    run_case(proc, writer, "unpitched", fs, x, shape="2ms-click", level=0)
    for name in (
        "1a-Low-E",
        "4a-G",
        "6a-High-E",
        "6c-High-E-24th",
        "Bend-Slide G",
        "GStaccato",
        "Hammer-Pull High E",
    ):
        x, fs = sf.read(AUDIO / (name + ".wav"), dtype="float64")
        assert x.ndim == 1
        run_case(proc, writer, "DI", fs, x, shape=name, level=0.004, preroll=False)


if __name__ == "__main__":
    fields = [
        "group",
        "fs",
        "f",
        "shape",
        "peak",
        "method",
        "mode",
        "sha256",
        "decimation",
        "analysis_hz",
        "W",
        "K",
        "H",
        "stimulus_start",
        "score_from",
        "score_until",
        "window",
        "eligible",
        "accepted",
        "rejected",
        "median_hz",
        "first_ms",
        "median",
        "p95",
        "octave",
        "rms",
        "lag_ms",
        "aligned_rms",
        "modulation_hz",
        "depth_cents",
    ]
    with subprocess.Popen(
        [OUT / "pitch_candidates"], stdin=subprocess.PIPE, stdout=subprocess.PIPE
    ) as proc:
        try:
            with (OUT / "pitch.csv").open("w", newline="") as file:
                writer = csv.DictWriter(file, fieldnames=fields)
                writer.writeheader()
                study(proc, writer)
        finally:
            proc.stdin.close()
        if proc.wait():
            raise RuntimeError("candidate comparison failed")
