"""Offline O2 matrix. Run from repo root after o1_qualification.py (Stage 06 report)."""

import json
import math
import subprocess
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
from o2_response import (
    RATES_EXTENDED,
    ROOT2,
    apply_load,
    blend,
    check_rate,
    check_transfer,
    coefficients,
    reference,
    require,
    verify_taps,
)
from scipy import signal
from scipy.io import wavfile

OUT = Path("build/stage-06")
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
    seconds = 7 if kind == "lift" else 3
    name = f"{kind}_{fs}_{f:g}_R{ratio:g}_V{speed:g}_{seconds}s.f64le"
    hashes = dict(line.split()[::-1] for line in o1.HASHES.read_text().splitlines())
    if name in hashes:
        path = o1.OUT / name
    else:
        path = OUT / name
        controls = np.tile([f, speed, ratio * speed / 1.65], (seconds * fs, 1)).astype(
            "<f8"
        )
        if kind == "lift":
            controls[fs:, 2] = 0
        with path.open("wb") as dest:
            run = subprocess.run(
                [OUT / "observed-render", str(fs), "--o1"],
                input=controls.tobytes(),
                stdout=dest,
                stderr=subprocess.PIPE,
                check=True,
            )
        require(list(map(int, run.stderr.split())) == [len(controls), 1, 0, 1], name)
        hashes = dict(
            line.split()[::-1]
            for line in (FIX / "o2_sha256.tsv").read_text().splitlines()
        )
    digest = guide.digest(path)
    require(digest == hashes[name], name)
    rows = np.memmap(path, mode="r", dtype="<f8").reshape(seconds * fs, 22)
    return rows, digest


def ratio_db(num, den):
    denom = obs.rms(den)
    return guide.db(obs.rms(num) / denom) if denom > FLOOR else None


def strongest(f, target):
    return abs(f - target) <= max(2.0, target * (2 ** (15 / 1200) - 1))


def summary(x, fs, f):
    spec = obs.harmonic_report(x, fs, f) if len(x) >= 3 else None
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


def v17_row(q, neutral, h, loaded, m5, v, nut, fs, f, noise, corrected=None):
    # Analyze a whole three-second physical render; final one-second window throughout.
    tail = slice(-fs, None)

    def level(num, den):
        return ratio_db(num[tail], den[tail])

    wet = q if corrected is None else corrected

    def voiced_pitch(x):
        return guide.spectrum(x[tail], fs, f)["hz"] if x[tail].std() > FLOOR else None

    measured_f = voiced_pitch(m5)
    peak_f = voiced_pitch(wet)
    voiced = measured_f is not None and peak_f is not None
    spectral = fft_metrics(wet[tail], fs, measured_f) if voiced else None
    eta_pass = voiced and bool(spectral["eta"] >= 0.85)
    rms_db = level(wet, m5)
    residual = level(q - neutral, neutral)
    color_db = level(wet, q)
    return {
        "peak_FS": float(max(abs(wet))),
        "rms_FS": obs.rms(wet[tail]),
        "same_source_difference_rms_FS": obs.rms((q - neutral)[tail]),
        "harmonics": obs.harmonic_report(wet[tail], fs, measured_f) if voiced else None,
        "spatial_over_N": level(v, nut),
        "load_over_blocked": level(loaded, 0.85 * h / ROOT2),
        "detail_over_neutral": level(q, neutral),
        "common_over_M5": level(ROOT2 * q, m5),
        "corrected_over_neutral": color_db,
        "final_over_M5": rms_db,
        "factory_difference_over_off": residual if noise == 0.5 else None,
        "measured_f0": measured_f,
        "strongest_hz": peak_f,
        "spectral": spectral,
        "passes": {
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
        h, loaded, _ = apply_load(raw[:, -1], coefficients(fs))
        check_transfer(
            blend(h, loaded, 1) - loaded, 2 * (blend(h, loaded, 0.5) - loaded)
        )
        require(
            obs.rms((blend(h, loaded, 0.5) - loaded)[-fs:]) > FLOOR,
            "absent carrier detail",
        )
        for noise in (0, 0.5, 1):
            q = blend(h, loaded, noise)
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


def acoustic():
    rows, high, onset, clips, inputs = {}, {}, {}, {}, {}
    witness = {}
    for fs, f, speed, r in (
        [(48000, f, 0.1, r) for f in (82.41, 196, 440) for r in (1.2, 1.8, 2.4)]
        + [(fs, 1318.51, 0.1, 1.8) for fs in (32000, 48000, 96000, 192000)]
        + [(48000, 440, 0.16, 1.8)]
    ):
        key = f"{fs}/{f}/{speed}/{r}"
        raw, inputs[key] = trace(fs, f, speed, r)
        v, m5 = raw[:, -1], raw[:, 11]
        h, loaded, _ = apply_load(v, coefficients(fs))
        check_transfer(
            blend(h, loaded, 1) - loaded, 2 * (blend(h, loaded, 0.5) - loaded)
        )
        require(
            obs.rms((blend(h, loaded, 0.5) - loaded)[-fs:]) > FLOOR,
            "absent carrier detail",
        )
        for noise in (0, 0.5, 1):
            q = blend(h, loaded, noise)
            verify_taps(h, 0.85 * h / ROOT2, loaded, q, ROOT2 * q, noise)
            rows[f"{key}/{noise}"] = v17_row(
                q, loaded, h, loaded, m5, v, raw[:, 3], fs, f, noise
            )
        if f < 1000 and speed == 0.1 and r == 1.8:
            for ident, corrected in corrections(loaded, fs, f).items():
                if ident != "neutral":
                    rows[f"stress/{f}/{ident}"] = v17_row(
                        loaded, loaded, h, loaded, m5, v, raw[:, 3], fs, f, 0, corrected
                    )
        if speed == 0.16:
            peak = float(np.max(abs(loaded)))
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
            clips[f"{f:g}Hz"] = {"M5": m5[:fs], "O2": ROOT2 * loaded[:fs]}
            if f == 440:
                clips["detail"] = {
                    "off": ROOT2 * loaded[:fs],
                    "factory": ROOT2 * blend(h, loaded, 0.5)[:fs],
                }
        if f == 1318.51:
            f0 = guide.spectrum(m5[-fs:], fs, f)["hz"]
            for name, x in [
                ("raw", v),
                ("blocked", h),
                ("loaded", loaded * ROOT2 / 0.85),
                ("factory", blend(h, loaded, 0.5) * ROOT2 / 0.85),
                ("full", blend(h, loaded, 1) * ROOT2 / 0.85),
            ]:
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
        h, loaded, _ = apply_load(raw[:, -1], coefficients(fs))
        f0 = guide.spectrum(raw[:fs, 11], fs, f)["hz"]
        for noise in (0, 0.5, 1):
            q = blend(h, loaded, noise)
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
                d = summary(q[window], fs, f0)
                d.update(
                    detail_difference_rms=obs.rms((q - loaded)[window]),
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
        "listening": listening(clips),
    }


def sweep(fs):
    # Electrical bench only. Same excitation in denominator, flat input spectrum
    # unnecessary. Include 0.5 s zero ring-out to close linear convolution.
    t = np.arange(3 * fs) / fs
    x = np.r_[
        0.1 * signal.chirp(t, 65, 3, 4000, method="logarithmic"), np.zeros(fs // 2)
    ]
    h, loaded, _ = apply_load(x, coefficients(fs))
    freq = np.fft.rfftfreq(len(x), 1 / fs)
    den = np.fft.rfft(x)
    valid = (freq >= 65) & (freq <= 4000) & (abs(den) / len(x) > FLOOR)
    rows = {}
    for noise in (0, 0.5, 1):
        measured = (
            np.fft.rfft(blend(h, loaded, noise))[valid] * ROOT2 / (0.85 * den[valid])
        )
        error = check_transfer(measured, reference(fs, freq[valid], noise))
        magnitude = abs(measured)
        maxima = signal.find_peaks(magnitude)[0]
        grid = freq[valid]
        flagged = []
        for peak in maxima[magnitude[maxima] > 10**0.3]:
            threshold = magnitude[peak] / ROOT2
            left = np.flatnonzero(magnitude[:peak] < threshold)
            right = np.flatnonzero(magnitude[peak:] < threshold)
            width = None
            if len(left) and len(right):
                width = grid[peak + right[0]] - grid[left[-1]]
            flagged.append(
                {"hz": grid[peak], "dB": guide.db(magnitude[peak]), "width_hz": width}
            )
        rows[str(noise)] = {
            "maximum_dB": guide.db(max(magnitude)),
            "peaks_above_6dB": flagged,
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

    changed = v17_row(2 * m5, m5, m5, m5, m5, m5, m5, fs, 196, 0)
    reject(
        "wrong denominator self-reference",
        lambda: check_transfer(
            np.array([ratio_db(2 * m5, 2 * m5)]), np.array([changed["final_over_M5"]])
        ),
    )
    reject("double wet level", lambda: grade_level(changed["final_over_M5"]))
    reject(
        "dominant H2",
        lambda: grade_pitch(v17_row(upper, m5, m5, m5, m5, m5, m5, fs, 196, 0)),
    )
    require(guide.db(0) == guide.db(1e-20) == -240, "amplitude ratio floor")
    return rejected


def main():
    OUT.mkdir(exist_ok=True, parents=True)
    build(OUT / "observed-render")
    result = {"electrical": {}, "errors": {}}
    for fs in RATES_EXTENDED:
        try:
            result["electrical"][str(fs)] = check_rate(fs)
        except ValueError as exc:
            result["errors"][str(fs)] = str(exc)
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
    result.update(
        data,
        corners=corners(),
        sweep=sweep(48000),
        negatives=negative_controls(),
        injected_vectors_dB=VECTORS,
    )
    failures = {
        key: [gate for gate, ok in row["passes"].items() if ok is False]
        for key, row in data["v17"].items()
    }
    result["failed_predicates"] = {k: v for k, v in failures.items() if v}
    result["neutral_candidate_passed"] = not result["errors"] and all(
        not gates for key, gates in failures.items() if not key.startswith("stress/")
    )
    result["all_tested_vectors_passed"] = (
        not result["errors"] and not result["failed_predicates"]
    )
    result["stage"] = "isolated offline; S5/V17 and S6R protection pending"
    result["head"] = guide.git("rev-parse", "HEAD")
    result["tool_hashes"] = {
        path.name: guide.digest(path)
        for path in [
            Path(__file__),
            Path(__file__).with_name("o2_response.py"),
            Path(obs.__file__),
            Path(o1.__file__),
            Path(guide.__file__),
            FIX / "o1_sha256.tsv",
            FIX / "o2_sha256.tsv",
            FIX / "acoustic/SHA256SUMS",
        ]
    }
    write_json("results.json", result)
    print("Review 5 witness", data["witness"])
    print(
        "Failed per-case predicates",
        result["failed_predicates"],
        "rate errors",
        result["errors"],
    )
    require(not result["errors"], "electrical rate failure; see results.json")
    require(
        all(
            not failed
            for key, failed in failures.items()
            if not key.startswith("stress/")
        ),
        "candidate exceedance; see results.json",
    )
    require(
        "fundamental" in failures["stress/196/inversion"],
        "identity stress missed inversion",
    )


if __name__ == "__main__":
    main()
