"""Stage 07 private Rust O2 replay. See STAGE_07_O2_IMPLEMENTATION.md."""

import json
import math
import os
from dataclasses import dataclass, replace
from hashlib import sha256
from itertools import product
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

import acoustic_observation as obs
import guide_qualification as guide
import numpy as np
import o1_qualification as o1
from acoustic_identity import resonant_bands
from guide_audit_build import build
from o2_audio import physical_key
from o2_audio import qualify as qualify_audio
from o2_audio import rows as audio_rows
from o2_bench import BINARY, DEPTHS, OUT, check_rate, replay, run, timing
from o2_coverage import UniqueRows, coverage_negatives, grade, grade_controls
from o2_response import (
    RATES_EXTENDED,
    ROOT2,
    apply_load,
    blend,
    check_transfer,
    coefficients,
    reference,
    require,
    verify_taps,
)
from scipy import signal
from scipy.io import wavfile

FIX = Path(__file__).with_name("fixtures")
FLOOR = obs.SPECTRAL_FLOOR
VECTORS = {
    "neutral": [0] * 12,
    "upper": [5] * 12,
    "lower": [-5] * 12,
    "inversion": [-5, 5] + [0] * 10,
}
HFBAND_FLAG = 0.01  # inquiry threshold, fraction of common-band power above 6 kHz


def write_json(name, value):
    (OUT / name).write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def trace(fs, f, speed=0.1, ratio=1.8, kind="steady"):
    name = physical_key(fs, f, speed, ratio, kind)
    path = OUT / "traces" / (name + ".f64le")
    return audio_rows(name), guide.digest(path)


def ratio_db(num, den):
    denom = obs.rms(den)
    return guide.db(obs.rms(num) / denom) if denom > FLOOR else None


def strongest(f, target, native_bin=1.0):
    return bool(
        f is not None
        and f > 0
        and (abs(f - target) <= 2 * native_bin or abs(1200 * np.log2(f / target)) <= 15)
    )


def strongest_hz(x, fs):
    if not np.isfinite(x).all() or x.std() <= FLOOR:
        return None
    n = 8 * len(x)
    amp = abs(np.fft.rfft((x - x.mean()) * np.hanning(len(x)), n))
    k = 1 + np.argmax(amp[1:])
    offset = 0.0
    if k < len(amp) - 1 and amp[k] >= amp[k - 1] and min(amp[k - 1 : k + 2]) > 0:
        left, center, right = np.log(amp[k - 1 : k + 2])
        curvature = left - 2 * center + right
        if curvature < 0:
            offset = 0.5 * (left - right) / curvature
    return float((k + offset) * fs / n)


def summary(x, fs, f):
    spec = obs.harmonic_report(x, fs, f) if len(x) >= 3 and f is not None else None
    return {
        "rms": obs.rms(x),
        "ac_rms": x.std(),
        "peak": float(np.max(abs(x))),
        "mean": float(x.mean()),
        "harmonic_FS": spec,
    }


def fft_metrics(x, fs, f, ceiling=None):
    # All callers supply a full second and voiced data.
    require(len(x) == fs and x.std() > FLOOR, "voiced FFT window")
    hz = np.arange(4 * fs + 1) / 8
    power = abs(np.fft.rfft((x - x.mean()) * np.hanning(fs), 8 * fs)) ** 2
    in_band = (hz > 0) & (hz <= (ceiling or fs / 2))
    k = np.rint(hz / f)
    harmonic = (
        (k >= 1) & (k * f < fs / 2) & (abs(hz - k * f) <= np.maximum(2, 0.001 * k * f))
    )
    total = power[in_band].sum()
    eta = power[in_band & harmonic].sum() / total
    hf = power[in_band & (hz >= 6000)].sum() / total
    return {
        "centroid_hz": (hz[in_band] @ power[in_band]) / total,
        "eta": eta,
        "residual": 1 - eta,
        "high_fraction": hf,
        "high_flag": bool(hf > HFBAND_FLAG),
    }


def pluck(fs, f, n):
    t = np.arange(n) / fs
    theta = 2 * np.pi * f * t
    x = (
        -np.expm1(-t / 0.001)
        * np.exp(-t / 0.28)
        * sum(
            np.sin(k * theta + 0.13 * k * k) / k
            for k in range(1, 9)
            if k * f < 0.45 * fs
        )
    )
    return 0.35 * x / max(abs(x))


@dataclass(frozen=True)
class WetTrace:
    samples: np.ndarray
    rate: int
    pitch: float
    start_tick: int
    # Identifies the exact physical epoch/control stream and all downstream settings.
    epoch: str
    settings: tuple
    depth: float

    source: str
    window: tuple
    end_tick: int
    count: int
    m5_hash: str
    off_hash: str

    def alignment(self):
        return (
            self.rate,
            self.pitch,
            self.start_tick,
            self.end_tick,
            self.count,
            self.epoch,
            self.source,
            self.settings,
            self.window,
            self.m5_hash,
            self.off_hash,
        )


def reference_trace(m5, off, fs, f, source, epoch, settings, start=0):
    def digest(x):
        return sha256(np.ascontiguousarray(x, dtype="<f8").tobytes()).hexdigest()

    end = start + len(m5)
    return WetTrace(
        off,
        fs,
        f,
        start,
        epoch,
        settings,
        0,
        source,
        (end - fs, end),
        end,
        len(m5),
        digest(m5),
        digest(off),
    )


def v17_row(
    q,
    neutral,
    h,
    loaded,
    m5,
    v,
    nut,
    fs,
    f,
    noise,
    *,
    operation,
    corrected=None,
    corrected_off=None,
    clock=None,
):
    tail = slice(-fs, None)

    def level(num, den):
        return ratio_db(num[tail], den[tail])

    if operation == "neutral":
        require(corrected is None and corrected_off is None, "unexpected colored wet")
        wet, off = q, neutral
    elif operation == "corrected":
        require(
            isinstance(clock, WetTrace)
            and isinstance(corrected, WetTrace)
            and isinstance(corrected_off, WetTrace),
            "paired final wet traces required",
        )
        ref = reference_trace(
            m5,
            neutral,
            fs,
            f,
            clock.source,
            clock.epoch,
            clock.settings,
            clock.start_tick,
        )
        require(
            ref.alignment()
            == clock.alignment()
            == corrected.alignment()
            == corrected_off.alignment()
            and corrected.depth == noise
            and corrected_off.depth == 0
            and corrected.samples.shape == corrected_off.samples.shape == m5.shape,
            "final wet alignment/settings/reference differ",
        )
        wet, off = corrected.samples, corrected_off.samples
    else:
        raise ValueError("unknown wet operation")
    require(
        all(x.shape == q.shape for x in (neutral, h, loaded, m5, v, nut, wet)),
        "unaligned V17 streams",
    )

    def voiced_pitch(x):
        return strongest_hz(x[tail], fs)

    measured_f = voiced_pitch(m5)
    peak_f = voiced_pitch(wet)
    anchor = strongest(measured_f, f, fs / len(m5[tail]))
    voiced = anchor and peak_f is not None
    spectral = fft_metrics(wet[tail], fs, measured_f) if voiced else None
    eta_pass = voiced and bool(spectral["eta"] >= 0.85)
    rms_db = level(wet, m5)
    local_residual = level(q - neutral, neutral)
    # Both wet arrays originate from the same epoch and coloration settings.
    if noise == 0.5:
        require(
            off is not None and off.shape == wet.shape,
            "aligned off-depth final wet required",
        )
    residual = level(wet - off, off) if noise == 0.5 else None
    color_db = level(wet, q)
    return {
        "failure_reason": None if anchor else "ineligible M5 bridge anchor",
        "peak_FS": float(max(abs(wet))),
        "rms_FS": obs.rms(wet[tail]),
        "same_source_difference_rms_FS": obs.rms((q - neutral)[tail]),
        "harmonics": obs.harmonic_report(wet[tail], fs, measured_f) if voiced else None,
        "spatial_over_N": level(v, nut),
        "load_over_blocked": level(loaded, 0.85 * h / ROOT2),
        "detail_over_neutral": level(q, loaded),
        "common_over_M5": level(ROOT2 * q, m5),
        "corrected_over_neutral": color_db,
        "final_over_M5": rms_db,
        "local_q_difference_over_off": local_residual,
        "factory_wet_difference_rms_FS": obs.rms((wet - off)[tail])
        if noise == 0.5
        else None,
        "factory_difference_over_off": residual if noise == 0.5 else None,
        "measured_f0": measured_f,
        "strongest_hz": peak_f,
        "spectral": spectral,
        "passes": {
            "bridge_anchor": bool(anchor),
            "level": rms_db is not None and abs(rms_db) <= 3,
            "coloration": color_db is not None and abs(color_db) <= 3,
            "fundamental": voiced and bool(strongest(peak_f, f)),
            "high_note_eta": eta_pass
            if fs == 32000 and f == 1318.51 and noise == 0
            else None,
            "factory_eta": eta_pass if noise == 0.5 else None,
            "factory_residual": (residual is not None and residual <= -28)
            if noise == 0.5
            else None,
        },
    }


def corrections(q, fs, f):
    bank = resonant_bands(q, fs, f, fs // (fs // 16000))
    return {
        name: q + (10 ** (np.asarray(v) / 20) - 1) @ bank for name, v in VECTORS.items()
    }


def corners():
    fs, f = 48000, 440
    direct = pluck(fs, f, 3 * fs)
    result = []
    for speed, r in product((0.06, 0.16), (1.2, 2.4)):
        raw, digest = trace(fs, f, speed, r)
        loaded = raw[:, 24]
        check_transfer(raw[:, 27] - loaded, 2 * (raw[:, 26] - loaded))
        require(
            obs.rms((raw[:, 26] - loaded)[-fs:]) > FLOOR,
            "absent carrier detail",
        )
        for noise in (0, 0.5, 1):
            q = raw[:, 25 + DEPTHS.index(noise)]
            for ident, wet in corrections(q, fs, f).items():
                for g, char, out in product((0.08, 1.4), (0.0, 1.0), (0.0, 2.0)):
                    pre = out * ((1 - char) * direct + char * g * wet)
                    peak = float(np.max(abs(pre)))
                    result.append(
                        {
                            "V": speed,
                            "R": r,
                            "noise": noise,
                            "vector": ident,
                            "G": g,
                            "character": char,
                            "internal_Out": out,
                            "peak": peak,
                            "demand_dB": max(0.0, guide.db(peak / 0.98)),
                            "trace": digest,
                        }
                    )
    return result


def archive_entry(pack, path, data):
    # Default ZipInfo time is fixed at 1980-01-01 for reproducible comparisons.
    pack.writestr(ZipInfo(str(path)), data, compress_type=ZIP_DEFLATED)


def listening(clips):
    # A small set of paired 1 s files plus a single 2 s lift.
    rng = np.random.default_rng(603)
    key = {}
    with ZipFile(OUT / "listening.zip", "w", ZIP_DEFLATED) as pack:
        for pair, sources in clips.items():
            names = list(rng.permutation(list(sources)))
            gains = [0.10 / obs.rms(sources[n][9600:38400]) for n in names]
            shared = min(
                1.0,
                0.90 / max(g * np.max(abs(sources[n])) for n, g in zip(names, gains)),
            )
            for i, (name, gain) in enumerate(zip(names, gains)):
                anon = f"{pair}-{chr(65 + i)}" if len(names) > 1 else pair
                gain *= shared
                for directory, label, scale in (
                    ("engineering", name, 1.0),
                    ("matched", anon, gain),
                ):
                    file = (
                        OUT
                        / directory
                        / (
                            pair + "-" + label + ".wav"
                            if directory == "engineering"
                            else label + ".wav"
                        )
                    )
                    file.parent.mkdir(exist_ok=True)
                    wavfile.write(file, 48000, (sources[name] * scale).astype("f4"))
                    archive_entry(pack, file.relative_to(OUT), file.read_bytes())
                key[anon] = {"source": name, "listening_gain": gain}
        ref = FIX / "acoustic/vaisvil-low-e-stroke.wav"
        archive_entry(pack, "reference/" + ref.name, ref.read_bytes())
        archive_entry(
            pack,
            "reference/ATTRIBUTION.md",
            (FIX / "acoustic/ATTRIBUTION.md").read_bytes(),
        )
        archive_entry(
            pack,
            "gains.json",
            json.dumps(
                {label: item["listening_gain"] for label, item in key.items()}, indent=2
            ),
        )
        archive_entry(
            pack,
            "LISTEN.txt",
            "Hear the physical low-E stroke, then matched A/B at low, middle and high pitch. Which resembles bowed electric guitar? Which buzzes less? Mark any whistle, resonance or objectionable harmonic with time. Is the 440 Hz detail pair distinguishable and useful? Is the sustained note and lift musically usable? Neither and indistinguishable are valid. Record listener, date, route and failed excerpt before opening the separate listening-key.json. Engineering WAVs preserve common gain. No effects.\n",
        )
    write_json("listening-key.json", key)
    return key


def acoustic(make_audio=True):
    rows, high, onset, clips, inputs = UniqueRows(), {}, {}, {}, {}
    witness = {}
    for fs, f, speed, r in (
        [(48000, f, 0.1, r) for f in (82.41, 196, 440) for r in (1.2, 1.8, 2.4)]
        + [(fs, 1318.51, 0.1, 1.8) for fs in guide.RATES]
        + [(48000, 440, 0.16, 1.8)]
    ):
        key = f"{fs}/{f}/{speed}/{r}"
        raw, inputs[key] = trace(fs, f, speed, r)
        v, m5 = raw[:, 21], raw[:, 11]
        h, loaded, off = raw[:, 22], raw[:, 24], raw[:, 25]
        check_transfer(raw[:, 27] - loaded, 2 * (raw[:, 26] - loaded))
        require(
            obs.rms((raw[:, 26] - loaded)[-fs:]) > FLOOR,
            "absent carrier detail",
        )
        for noise in (0, 0.5, 1):
            q = raw[:, 25 + DEPTHS.index(noise)]
            verify_taps(h, raw[:, 23], loaded, q, ROOT2 * q, noise)
            rows[f"{key}/{noise}"] = v17_row(
                q, off, h, loaded, m5, v, raw[:, 3], fs, f, noise, operation="neutral"
            )
        if f < 1000 and speed == 0.1 and r == 1.8:
            for ident, corrected in corrections(off, fs, f).items():
                if ident != "neutral":
                    clock = reference_trace(m5, off, fs, f, key, "launch-1", (ident,))
                    colored = replace(clock, samples=corrected)
                    rows[f"stress/{f}/{ident}"] = v17_row(
                        off,
                        off,
                        h,
                        loaded,
                        m5,
                        v,
                        raw[:, 3],
                        fs,
                        f,
                        0,
                        operation="corrected",
                        corrected=colored,
                        corrected_off=colored,
                        clock=clock,
                    )
        if speed == 0.16:
            peak = float(np.max(abs(off)))
            witness = {
                "common": ROOT2 * peak,
                "trimmed": peak,
                "implied": peak * 1.4 * 2,
                "demand_db": guide.db(peak * 1.4 * 2 / 0.98),
            }
            for a, b in zip(witness.values(), (1.005419, 0.710938, 1.990628, 6.155)):
                require(
                    abs(a - b) < (0.0005 if b == 6.155 else 1e-6), "Review 5 witness"
                )
        elif f < 1000 and r == 1.8:
            clips[f"{f:g}Hz"] = {"M5": m5[:fs], "O2": ROOT2 * off[:fs]}
            if f == 440:
                clips["detail"] = {
                    "off": ROOT2 * off[:fs],
                    "factory": ROOT2 * raw[:, 26][:fs],
                }
        if f == 1318.51:
            f0 = strongest_hz(m5[-fs:], fs)
            anchor_ok = strongest(f0, f)
            for name, x in [
                ("raw", v),
                ("blocked", h),
                ("loaded", loaded * ROOT2 / 0.85),
                ("factory", raw[:, 26] * ROOT2 / 0.85),
                ("full", raw[:, 27] * ROOT2 / 0.85),
            ]:
                if not anchor_ok:
                    high[f"{fs}/{name}"] = {
                        "failure_reason": "ineligible high-note bridge anchor",
                        "f0": f0,
                        "eta": None,
                    }
                    continue
                divisor = math.gcd(fs, 32000)
                y = signal.resample_poly(
                    x, 32000 // divisor, fs // divisor, window=("kaiser", 10)
                )
                y = y[48000:80000]  # interior, 1.5..2.5 s at common rate
                spec = fft_metrics(y, 32000, f0, 14000)
                harm = obs.harmonic_report(y, 32000, f0)
                count = math.floor(14000 / f0)
                high[f"{fs}/{name}"] = dict(
                    **spec,
                    f0=f0,
                    rms_m_s=obs.rms(y),
                    amplitudes_m_s=harm["peak_FS"][:count],
                    phase_deg=harm["phase_deg"][:count],
                )
    for key, data in high.items():
        ref = high["192000/" + key.split("/")[1]]
        if "failure_reason" in ref or "failure_reason" in data:
            data.update(
                centroid_minus_ref_hz=None,
                H1_to_H10_delta_dB=None,
                high_fraction_minus_ref=None,
            )
            continue
        data["centroid_minus_ref_hz"] = data["centroid_hz"] - ref["centroid_hz"]
        data["H1_to_H10_delta_dB"] = obs.relative_db(
            np.array(data["amplitudes_m_s"]),
            np.array(ref["amplitudes_m_s"]),
            np.array(data["amplitudes_m_s"]) > FLOOR,
        )
        data["high_fraction_minus_ref"] = data["high_fraction"] - ref["high_fraction"]
    for f in (82.41, 440):
        fs = 48000
        raw, inputs[f"lift/{f}"] = trace(fs, f, kind="lift")
        h, loaded, off = raw[:, 22], raw[:, 24], raw[:, 25]
        f0 = strongest_hz(raw[:fs, 11], fs)
        anchor_ok = strongest(f0, f)
        for noise in (0, 0.5, 1):
            q = raw[:, 25 + DEPTHS.index(noise)]
            for start, end in [
                (0, 1 / fs),
                (0, 0.01),
                (0, 0.05),
                (0.5, 1),
                (0.975, 1),
                (1, 1.025),
                (1.025, 1.05),
                (1.1, 1.15),
                (2, 2.05),
                (6, 7),
            ]:
                window = slice(round(start * fs), round(end * fs))
                d = summary(q[window], fs, f0 if anchor_ok else None)
                if not anchor_ok:
                    d["failure_reason"] = "ineligible lift bridge anchor"
                d.update(
                    detail_difference_rms=obs.rms((q - off)[window]),
                    load_minus_input_rms=obs.rms((loaded - 0.85 * h / ROOT2)[window]),
                )
                onset[f"{f}/{noise}/{start}-{end}"] = d
            if f == 82.41 and noise == 0.5:
                clips["lift"] = {"O2": ROOT2 * q[: 2 * fs]}
    return {
        "v17": rows,
        "high_band": high,
        "onset_release": onset,
        "witness": witness,
        "inputs": inputs,
        "listening": listening(clips) if make_audio else None,
    }


def sweep_peaks(grid, magnitude):
    # Each contiguous eligible band owns its endpoints. An edge can truncate
    # either half-power crossing, in which case width is unavailable.
    peaks = []
    for ix in np.split(
        np.arange(len(grid)),
        np.flatnonzero(np.diff(grid) > 1.5 * np.min(np.diff(grid))) + 1,
    ):
        y = magnitude[ix]
        maxima = list(signal.find_peaks(y)[0])
        if len(y) == 1 or y[0] >= y[1]:
            maxima.append(0)
        if len(y) > 1 and y[-1] >= y[-2]:
            maxima.append(len(y) - 1)
        for k in maxima:
            threshold = y[k] * 10 ** (-3 / 20)
            left = np.flatnonzero(y[:k] < threshold)
            right = np.flatnonzero(y[k + 1 :] < threshold)
            width = None
            if len(left) and len(right):
                i, j = left[-1], k + 1 + right[0]
                low = np.interp(threshold, y[i : i + 2], grid[ix[i : i + 2]])
                high = np.interp(
                    threshold, y[j - 1 : j + 1][::-1], grid[ix[j - 1 : j + 1]][::-1]
                )
                width = high - low
            peaks.append({"hz": grid[ix[k]], "dB": guide.db(y[k]), "width_hz": width})
    return peaks


def sweep(fs):
    # Electrical bench only. Same excitation in denominator, flat input spectrum
    # unnecessary. Include 0.5 s zero ring-out to close linear convolution.
    t = np.arange(3 * fs) / fs
    x = np.r_[
        0.1 * signal.chirp(t, 65, 3, 4000, method="logarithmic"), np.zeros(fs // 2)
    ]
    freq = np.fft.rfftfreq(len(x), 1 / fs)
    den = np.fft.rfft(x)
    valid = (freq >= 65) & (freq <= 4000) & (abs(den) / len(x) > FLOOR)
    rows = UniqueRows()
    for noise in (0, 0.5, 1):
        measured = (
            np.fft.rfft(replay(fs, x, noise)[:, 3])[valid] * ROOT2 / (0.85 * den[valid])
        )
        error = float(np.max(abs(measured - reference(fs, freq[valid], noise))))
        magnitude = abs(measured)
        peaks = sweep_peaks(freq[valid], magnitude)
        rows[str(noise)] = {
            "maximum_dB": max(p["dB"] for p in peaks),
            "maximum": max(peaks, key=lambda p: p["dB"]),
            "peaks_above_6dB": [p for p in peaks if p["dB"] > 6],
            "complex_error": error,
        }
    return rows


def negative_controls():
    rejected = {}

    def reject(name, fn):
        try:
            fn()
        except ValueError as exc:
            rejected[name] = str(exc)
        else:
            raise ValueError("escaped negative: " + name)

    fs = 48000
    coeff = coefficients(fs)
    x = np.r_[1.0, np.zeros(fs // 2 - 1)]
    h, loaded, _ = apply_load(x, coeff)
    hz = np.fft.rfftfreq(len(x), 1 / fs)
    ref = reference(fs, hz)

    def transfer_check(y):
        check_transfer(np.fft.rfft(y) * ROOT2 / 0.85, ref)

    # Design with the unwarped analog pole and with a fixed-48k digital cutoff at 32k.
    b, a = signal.bilinear(
        [(2 * np.pi * 3200) ** 2],
        [1, ROOT2 * 2 * np.pi * 3200, (2 * np.pi * 3200) ** 2],
        fs=fs,
    )
    reject("no prewarp", lambda: transfer_check(signal.lfilter(b, a, 0.85 * h / ROOT2)))
    reject(
        "wrong denominator sign",
        lambda: require(
            max(abs(np.roots([1, -coeff[4], -coeff[5]]))) < 1, "unstable poles"
        ),
    )
    reject(
        "unstable pole",
        lambda: check_transfer(
            signal.freqz([1], [1, -1.001], worN=[0.1])[1],
            reference(fs, np.array([0.1 * fs / (2 * np.pi)])),
        ),
    )
    reject(
        "single rate fixed cutoff",
        lambda: check_transfer(
            np.fft.rfft(loaded) * ROOT2 / 0.85,
            reference(32000, np.fft.rfftfreq(len(x), 1 / 32000)),
        ),
    )
    reject(
        "no DC blocker",
        lambda: transfer_check(
            signal.lfilter(coeff[1:4], [1, *coeff[4:]], x) * 0.85 / ROOT2
        ),
    )
    reject(
        "old reset state",
        lambda: transfer_check(apply_load(x, coeff, (0.0, 0.0, 0.1, -0.1))[1]),
    )
    a = 0.85 * h / ROOT2
    q = blend(h, loaded, 0.5)
    for name, taps in [
        ("gain after filter input", (h, h, loaded, q, ROOT2 * q, 0.5)),
        ("duplicate conversion", (h, a * 0.85, loaded, q, ROOT2 * q, 0.5)),
        ("duplicate trim", (h, a / ROOT2, loaded, q, ROOT2 * q, 0.5)),
        ("excess detail", (h, a, loaded, loaded + 0.1 * (a - loaded), ROOT2 * q, 0.5)),
        ("per-render normalization", (h, a, loaded, q, q / max(abs(q)), 0.5)),
    ]:
        reject(name, lambda taps=taps: verify_taps(*taps))

    def silent(y):
        require(np.count_nonzero(y) == 0, "silence feedthrough")

    z = np.zeros(256)
    zh, zl, _state = apply_load(z, coeff)
    for noise in (0, 0.5, 1):
        silent(blend(zh, zl, noise))
    reject("silent offset", lambda: silent(zl + 1e-16))

    # Use O1 reporter's actual relative/phase outputs, including a strong signal
    # with identically zero N. Physical magnitudes remain available.
    def check_null(stats):
        for field in (
            "harmonic_phase_deg",
            "envelope_dBr",
            "center_over_N_dB",
            "aperture_over_center_dB",
        ):
            require(all(x is None for x in stats[field]), "ineligible " + field)

    t = np.arange(fs) / fs
    raw = np.zeros((fs, 22))
    entry = {"rate": fs, "trajectory": "steady", "seconds": 1, "hz": 196}
    for scale in (0.0, 1e-22):
        raw[:, -1] = scale * np.sin(2 * np.pi * 196 * t)
        report = o1.summarize(raw, entry, 196)
        check_null(report)
        json.dumps(report, allow_nan=False)
    reject(
        "tail phase from numerical residue",
        lambda: check_null(dict(report, harmonic_phase_deg=[0.0] * 12)),
    )
    raw[:, 17] = np.sin(2 * np.pi * 196 * t)
    require(
        all(v is None for v in o1.summarize(raw, entry, 196)["center_over_N_dB"]),
        "zero harmonic denominator",
    )
    silent_tail = raw[:, -1] * 0
    missing = v17_row(
        silent_tail,
        silent_tail,
        silent_tail,
        silent_tail,
        silent_tail,
        silent_tail,
        silent_tail,
        fs,
        196,
        0.5,
        operation="neutral",
    )
    require(
        missing["final_over_M5"] is None
        and missing["spectral"] is None
        and not missing["passes"]["fundamental"],
        "unavailable silent metrics",
    )
    json.dumps(missing, allow_nan=False)
    reject(
        "zero RMS denominator",
        lambda: require(missing["passes"]["level"], "ineligible RMS denominator"),
    )
    phase = 2 * np.pi * 196 * t
    m5 = np.sin(phase)
    upper = 0.5 * m5 + 2 * np.sin(2 * phase)

    def grade_level(value):
        require(abs(value) <= 3, "original M5 level")

    def grade_pitch(row):
        require(row["passes"]["fundamental"], "strongest fundamental")

    changed = v17_row(2 * m5, m5, m5, m5, m5, m5, m5, fs, 196, 0, operation="neutral")
    reject(
        "wrong denominator self-reference",
        lambda: check_transfer(
            np.array([ratio_db(2 * m5, 2 * m5)]), np.array([changed["final_over_M5"]])
        ),
    )
    reject("double wet level", lambda: grade_level(changed["final_over_M5"]))
    reject(
        "dominant H2",
        lambda: grade_pitch(
            v17_row(upper, m5, m5, m5, m5, m5, m5, fs, 196, 0, operation="neutral")
        ),
    )
    require(guide.db(0) == guide.db(1e-20) == -240, "amplitude ratio floor")
    nyquist = 0.5 * m5 + 0.6 * (-1.0) ** np.arange(fs)
    nyq_row = v17_row(nyquist, m5, m5, m5, m5, m5, m5, fs, 196, 0, operation="neutral")
    require(nyq_row["strongest_hz"] == fs / 2, "Nyquist endpoint")
    reject("dominant Nyquist", lambda: grade_pitch(nyq_row))
    flat = np.sin(2 * np.pi * 440 * 2 ** (-15.04 / 1200) * t)
    anchor = np.sin(2 * np.pi * 440 * t)
    reject(
        "15.04 cents flat",
        lambda: grade_pitch(
            v17_row(
                flat,
                anchor,
                anchor,
                anchor,
                anchor,
                anchor,
                anchor,
                fs,
                440,
                0,
                operation="neutral",
            )
        ),
    )
    reject(
        "invalid bridge anchor",
        lambda: grade_pitch(
            v17_row(m5, m5, m5, m5, upper, m5, m5, fs, 196, 0, operation="neutral")
        ),
    )
    colored = m5 + 0.1 * np.sin(2 * np.pi * 6151 * t)
    off = reference_trace(m5, m5, fs, 196, "injected DI", "epoch-1", ("unity",))
    factory = replace(off, samples=colored, depth=0.5)

    def paired(off_trace, on=factory):
        return v17_row(
            m5,
            m5,
            m5,
            m5,
            m5,
            m5,
            m5,
            fs,
            196,
            0.5,
            operation="corrected",
            corrected=on,
            corrected_off=off_trace,
            clock=off,
        )

    row = paired(off)
    reject("off only", lambda: paired(off, None))
    reject("factory only", lambda: paired(None))
    for label, edit in [
        ("unaligned wet pair", {"start_tick": 1}),
        ("unmatched wet settings", {"settings": ("other gain",)}),
        ("wrong epoch", {"epoch": "epoch-2"}),
        ("wrong reference", {"m5_hash": off.m5_hash[::-1]}),
        ("wrong measurement window", {"window": (1, fs)}),
        ("joint shift", {"start_tick": 1, "end_tick": fs + 1, "window": (1, fs + 1)}),
    ]:
        reject(
            label,
            lambda edit=edit: paired(replace(off, **edit), replace(factory, **edit)),
        )
    require(
        abs(row["factory_difference_over_off"] + 20) < 1e-9
        and row["local_q_difference_over_off"] == -240,
        "colored wet control",
    )
    reject(
        "colored final wet",
        lambda: require(row["passes"]["factory_residual"], "final wet residual -20 dB"),
    )
    edge = sweep_peaks(np.arange(3.0), np.array([3.0, 1.0, 0.5]))[0]
    require(
        edge["hz"] == 0 and edge["width_hz"] is None and edge["dB"] > 6,
        "sweep endpoint",
    )
    return rejected


def tool_hashes():
    paths = [
        *sorted(Path(__file__).parent.glob("o2_*.py")),
        *sorted(Path(__file__).parent.glob("o2_*.rs")),
        Path(__file__).with_name("guide_render.rs"),
        Path(__file__).with_name("guide_epoch.rs"),
        Path(__file__).with_name("guide_audit_build.py"),
        Path(__file__).with_name("acoustic_identity.py"),
        Path(obs.__file__),
        Path(o1.__file__),
        Path(guide.__file__),
        *(
            Path("rust/dsp/src/instrument") / (name + ".rs")
            for name in ("o2", "waveguide", "friction", "ring")
        ),
        FIX / "o1_sha256.tsv",
        FIX / "o2_sha256.tsv",
        FIX / "o2_corner_peaks.tsv",
        FIX / "acoustic/SHA256SUMS",
    ]
    return {p.name: guide.digest(p) for p in paths}


def qualify(result):
    build(BINARY, o2=True)
    result["replay"] = qualify_audio()
    result.update(electrical=UniqueRows(), lifecycle=UniqueRows())
    for fs in RATES_EXTENDED:
        result["electrical"][str(fs)] = check_rate(fs)
    for fs in (32000, 48000, 192000):
        for line in run("lifecycle", fs).splitlines():
            name, *values = line.split()
            result["lifecycle"][f"{fs}/{name}"] = list(map(int, values))
    result["local_timing"] = timing()
    pluck_frames = pluck(48000, 196, 144000)
    nominal = np.pad(pluck_frames.astype("<f4"), (4800, 62400))
    result["nominal_mono_input"] = {
        "fs": 48000,
        "frames": len(nominal),
        "eligible_frames": len(pluck_frames),
        "peak_FS": float(max(abs(nominal))),
        "sha256_f32le": sha256(nominal.tobytes()).hexdigest(),
    }
    data = acoustic()
    result.update(data)
    result.update(
        corners=corners(),
        sweep=sweep(48000),
        negatives=negative_controls(),
        injected_vectors_dB=VECTORS,
    )
    result["failed_predicates"] = {
        k: [g for g, v in row["passes"].items() if v is False]
        for k, row in data["v17"].items()
        if False in row["passes"].values()
    }
    result["all_tested_vectors_passed"] = not result["failed_predicates"]
    result["stage"] = "acceptance tooling awaiting independent Review 8"
    result["head"] = guide.git("rev-parse", "HEAD")
    result["tool_hashes"] = tool_hashes()
    result["renderer_sha256"] = guide.digest(BINARY)
    grade(result)
    result["negatives"].update(coverage_negatives(result))
    grade_controls(result["negatives"])
    result["neutral_candidate_passed"] = True
    print("Retained downstream failures", result["failed_predicates"])


def main():
    OUT.mkdir(exist_ok=True, parents=True)
    result = {"neutral_candidate_passed": False}
    write_json(
        "results.json",
        {"neutral_candidate_passed": False, "error": "qualification incomplete"},
    )
    try:
        qualify(result)
        json.dumps(result, allow_nan=False)
    except Exception as exc:
        result["error"] = f"{type(exc).__name__}: {exc}"
        result["neutral_candidate_passed"] = False
        from o2_coverage import REQUIRED

        result["missing_sections"] = sorted(REQUIRED - result.keys())
        raise
    finally:
        # Preserve partial evidence, including the location of invalid numbers.
        def clean(value):
            if isinstance(value, dict):
                return {k: clean(v) for k, v in value.items()}
            if isinstance(value, (list, tuple)):
                return [clean(v) for v in value]
            if isinstance(value, np.generic):
                return clean(value.item())
            if isinstance(value, float) and not math.isfinite(value):
                return None
            return value

        path = OUT / "results.tmp"
        path.write_text(json.dumps(clean(result), allow_nan=False, indent=2) + "\n")
        os.replace(path, OUT / "results.json")


if __name__ == "__main__":
    main()
