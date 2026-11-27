"""Replay every Stage 03 control stream through the committed Rust O1 boundary.

Run guide_qualification.py first. Keep the 15 acoustic traces and their physical
measurements. The other extended rows are temporary. No O1 audio gain is used.
"""

import json
import subprocess
from hashlib import sha256
from pathlib import Path

import acoustic_observation as research
import guide_qualification as guide
import numpy as np
from guide_audit_build import build
from scipy import signal

OUT = Path("build/stage-05")
# Canonical name/raw/WAV digest list from the Review 3 independently reproduced run.
BASELINE = "a8bd257a945a47aecb31c81cf4569613b87b296db52fd5b93a568ce39eb124b3"
HASHES = Path(__file__).with_name("fixtures") / "o1_sha256.tsv"
FIELDS = [
    f"{station}_{field}"
    for station in ("left", "center", "right")
    for field in ("direct", "reflected_history", "velocity")
] + ["aperture"]


def compare(actual, expected, name, field, hz, fs, delays):
    error = abs(actual - expected)
    failed = np.flatnonzero(~(error <= 1e-9))
    if len(failed):
        n = failed[0]
        refs = [(s, d[n], n - int(np.ceil(d[n])), n - int(d[n])) for s, d in delays]
        raise ValueError(
            f"{name} {field} first divergence rate={fs} tick={n} Hz={hz[n]} "
            f"station/delay/indices={refs} {actual[n]} vs {expected[n]}"
        )
    return float(error.max())


def summarize(rows, e, f0):
    fs, kind = e["rate"], e["trajectory"]
    frames = fs * e["seconds"]
    x = rows[:frames, -1]
    tail = rows[frames - fs : frames]
    harmonics = research.partials(tail[:, -1], fs, f0)
    center = research.partials(tail[:, 17], fs, f0)
    outgoing = research.partials(tail[:, 3], fs, f0)
    stats = {
        "rms_m_s": research.rms(x),
        "steady_rms_m_s": research.rms(x[-fs:]),
        "peak_m_s": float(abs(x).max()),
        "projection_Hz": f0,
        "harmonic_peak_m_s": abs(harmonics).tolist(),
        "harmonic_phase_deg": np.angle(harmonics, deg=True).tolist(),
        "envelope_dBr": research.db(abs(harmonics / harmonics[0])).tolist(),
        "center_over_N_dB": research.db(abs(center / outgoing)).tolist(),
        "aperture_over_center_dB": research.db(abs(harmonics / center)).tolist(),
    }
    if kind in ("bend", "vibrato"):
        stats["auxiliary_motion"] = research.tracking(rows[:, -1], fs, e["hz"], kind)
    if kind == "lift":
        stats["rms_50ms_at_0.95_1_1.10_2s"] = [
            research.rms(x[n : n + fs // 20])
            for n in (fs - fs // 20, fs, fs + fs // 10, 2 * fs)
        ]
    if e["hz"] == 1318.51:
        # Same one-second, 0..16 kHz analysis band for all three rates.
        from math import gcd

        divisor = gcd(32000, fs)
        y = signal.resample_poly(
            x, 32000 // divisor, fs // divisor, window=("kaiser", 10)
        )[-32000:]
        power = abs(np.fft.rfft((y - y.mean()) * np.hanning(len(y)))) ** 2
        stats["common_32k_centroid_Hz"] = float(
            power @ np.arange(len(power)) / power.sum()
        )
        stats["common_32k_above_6k_fraction"] = float(power[6000:].sum() / power.sum())
    return stats


def replay(binary, name, e, keep, f0):
    fs = e["rate"]
    n = e["successful_ticks"]
    t = np.arange(n) / fs
    hz = e["hz"] * np.exp2(guide.trajectory(t, e["trajectory"]) / 1200)
    controls = np.empty((n, 3), dtype="<f8")
    controls[:, 0], controls[:, 1], controls[:, 2] = hz, e["speed"], e["k"]
    if e["trajectory"] == "lift":
        controls[fs:, 2] = 0
    payload = controls.tobytes()
    if sha256(payload).hexdigest() != e["controls_sha256"]:
        raise ValueError(f"control trajectory {name}")
    path = OUT / (name + ".f64le" if keep else "tmp/extended.f64le")
    with path.open("wb") as stream:
        run = subprocess.run(
            [binary, str(fs), "--o1"],
            input=payload,
            stdout=stream,
            stderr=subprocess.PIPE,
            check=True,
        )
    ticks, launches, faults, invalidations = map(int, run.stderr.split())
    if (ticks, launches, faults, invalidations) != (n, 1, 0, 1):
        raise ValueError(f"actual guide lifecycle {name}: {run.stderr!r}")
    rows = np.memmap(path, dtype="<f8", mode="r").reshape(n, 22)
    # Compare all bits including signed zero. Reference digest also pins entry M5.
    original = guide.OUT / e["raw"]
    if guide.digest(original) != e["raw_sha256"]:
        raise ValueError(f"guide trace hash {name}")
    baseline = np.memmap(original, dtype="<u8", mode="r").reshape(n, 12)
    for start in range(0, n, 65536):
        block = slice(start, start + 65536)
        diff = np.argwhere(rows[block, :12].view("<u8") != baseline[block])
        if len(diff):
            frame, col = diff[0]
            raise ValueError(
                f"physical parity {name} tick={start + frame} {guide.COLUMNS[col]}"
            )
    if "wav" in e and guide.digest(guide.OUT / e["wav"]) != e["wav_sha256"]:
        raise ValueError(f"M5 audio hash {name}")
    period = fs / hz
    worst = {}
    all_delays = []
    aperture = np.zeros(n)
    for i, s in enumerate(research.STATIONS):
        rays = []
        ds = [(s, (s - 0.127) * period / 2), (s, (2 - s - 0.127) * period / 2)]
        for j, (_, delay) in enumerate(ds):
            ref = research.read_arm(rows[:, 3], delay, period[0], e["speed"])
            field = FIELDS[3 * i + j]
            worst[field] = compare(
                rows[:, 12 + 3 * i + j], ref, name, field, hz, fs, ds[j : j + 1]
            )
            rays.append(ref)
        total = rays[0] - rays[1]
        field = FIELDS[3 * i + 2]
        worst[field] = compare(rows[:, 14 + 3 * i], total, name, field, hz, fs, ds)
        aperture += [1 / 6, 4 / 6, 1 / 6][i] * total
        all_delays.extend(ds)
    if keep:
        aperture = research.spatial_velocity(rows, fs, hz, e["speed"])
    worst["aperture"] = compare(
        rows[:, -1], aperture, name, "aperture", hz, fs, all_delays
    )
    result = {
        "ticks": n,
        "peak_m_s": float(abs(rows[:, -1]).max()),
        "max_error_m_s": worst,
        "launches": launches,
        "faults": faults,
        "invalidations": invalidations,
    }
    if keep:
        result.update(
            file=path.name,
            sha256=guide.digest(path),
            measurements=summarize(rows, e, f0),
        )
    else:
        del rows
        path.unlink()
    return result


def main():
    (OUT / "tmp").mkdir(parents=True, exist_ok=True)
    binary = OUT / "observed-render"
    build(binary)
    source = json.loads((guide.OUT / "manifest.json").read_text())
    cases = source["cases"]
    baseline = "".join(
        f"{n} {e['raw_sha256']} {e.get('wav_sha256', '-')}\n"
        for n, e in sorted(cases.items())
    )
    if (
        not source["passed"]
        or len(cases) != 152
        or sha256(baseline.encode()).hexdigest() != BASELINE
    ):
        raise ValueError("qualified physical/M5 baseline mismatch")
    research_cases = json.loads((research.OUT / "observation.json").read_text())
    retain = set(cases).intersection(research_cases)
    if len(retain) != 15:
        raise ValueError("expected the fifteen Stage 04 acoustic fixtures")
    results = {}
    for name, e in cases.items():
        f0 = None
        if name in retain:
            nominal = (
                f"steady_{e['rate']}_{e['hz']:g}_R{e['ratio']:g}_V{e['speed']:g}_3s"
            )
            f0 = cases[nominal]["pickup"]["hz"]
        results[name] = replay(binary, name, e, name in retain, f0)
    paths = [
        *guide.SOURCES,
        guide.ADAPTER,
        guide.PROBES,
        Path(__file__),
        Path(research.__file__),
        Path(guide.__file__),
        Path(guide.__file__).with_name("guide_audit_build.py"),
    ]
    manifest = {
        "head": guide.git("rev-parse", "HEAD"),
        "format": "row-major little-endian f64",
        "columns": guide.COLUMNS + FIELDS,
        "O1_units": "m/s",
        "baseline": BASELINE,
        "stage03_manifest_sha256": guide.digest(guide.OUT / "manifest.json"),
        "source_sha256": {str(p): guide.digest(p) for p in paths},
        "cases": results,
    }
    (OUT / "manifest.json").write_text(
        json.dumps(manifest, indent=2, allow_nan=False) + "\n"
    )
    hashes = "".join(
        f"{results[n]['sha256']}\t{results[n]['file']}\n" for n in sorted(retain)
    )
    (OUT / "o1_sha256.tsv").write_text(hashes)
    print(
        f"{len(results)} cases, {sum(e['ticks'] for e in results.values())} observed ticks, "
        f"{len(retain)} retained traces, worst {max(max(e['max_error_m_s'].values()) for e in results.values()):.3e} m/s"
    )


if __name__ == "__main__":
    main()
    if HASHES.read_bytes() != (OUT / "o1_sha256.tsv").read_bytes():
        raise ValueError("O1 trace SHA-256 mismatch")
