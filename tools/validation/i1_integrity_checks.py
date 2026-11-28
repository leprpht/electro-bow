"""Decoded recording and exact-neutral counterexamples. All generated DI is synthetic."""

import json

import numpy as np
from acoustic_identity import audio
from guide_qualification import digest
from i1_motion import OUT, bank, witness
from i1_recordings import PROTOCOL, decoded
from i1_repeatability import qualify
from i1_safety import check_trajectory, drive, nonnegative
from o2_response import require
from scipy.io import wavfile


def containers(directory, altered=False):
    fs = 48000
    t = np.arange(round(0.34 * fs)) / fs
    classes = {}
    for c, factor in (("A", 1.65), ("B", 0.35)):
        takes = []
        h = 0.12 / np.arange(1, 13)
        h[5] *= factor
        pcm = (np.cos(2 * np.pi * 196 * t[:, None] * np.arange(1, 13)) @ h).astype("f4")
        for i in range(4):
            path = directory / f"{c}{i}.wav"
            # A unique tail must not make a duplicate capture into a separate attack.
            x = pcm.copy()
            if altered:
                x[-1] = 0.01 * i
            wavfile.write(path, fs, x)
            with path.open("ab") as stream:
                stream.write(i.to_bytes(2, "little"))
            _, y = audio(path)
            takes.append(
                {
                    "original": path.name,
                    "interval": [0, len(y)],
                    "order": 2 * i + 1 + (c == "B"),
                    "take_id": c + str(i),
                    "rate": fs,
                    "original_sha256": digest(path),
                    "decoded_sha256": decoded(y),
                    "segment_sha256": decoded(y),
                    "attack_record": "synthetic duplicate probe",
                }
            )
        classes[c] = {
            "pickup": c,
            "guitar": "synthetic",
            "string": "G",
            "fret": 0,
            "chain": "generated",
            "gain": 1,
            "gesture": "injected",
            "performer": "generator",
            "takes": takes,
        }
    manifest = directory / "manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "classes": classes,
                "hz": 196,
                "comparison": "pickup",
                "protocol_revision": PROTOCOL,
                "recording_mode": "separate-originals",
                "manifest_revision": 2,
            },
            indent=2,
        )
    )
    result = qualify(manifest)
    expected = "duplicate decoded capture" if altered else "duplicate decoded recording"
    require(
        any(expected in e for e in result["failures"])
        and len(result["di_pairs"]) == 28
        and "wet" not in result,
        "controlled duplication",
    )
    return result


def shifted_session(directory):
    directory.mkdir(exist_ok=True, parents=True)
    containers(directory)
    manifest = directory / "manifest.json"
    config = json.loads(manifest.read_text())
    config["recording_mode"] = "shared-session"
    rate, a = wavfile.read(directory / "A0.wav")
    _, b = wavfile.read(directory / "B0.wav")
    original = np.r_[a, np.zeros(rate, dtype=a.dtype), b]
    original_path = directory / "session.wav"
    wavfile.write(original_path, rate, original)
    log = []
    for label, start, pcm in (("A", 0, a), ("B", len(a) + rate, b)):
        for i, take in enumerate(config["classes"][label]["takes"]):
            offset = 5 * i
            part = pcm[offset:]
            path = directory / take["original"]
            wavfile.write(path, rate, part)
            _, y = audio(path)
            interval = [start + offset, start + len(pcm)]
            take.update(
                interval=[0, len(part)],
                session_interval=interval,
                original_sha256=digest(path),
                decoded_sha256=decoded(y),
                segment_sha256=decoded(y),
            )
            log.append(
                dict(
                    order=take["order"],
                    take_id=take["take_id"],
                    interval=interval,
                    attack=interval[0] + int(np.flatnonzero(abs(y) > 0.01)[0]),
                    gain=1,
                    pickup=label,
                    gesture="injected",
                )
            )
    logfile = directory / "attacks.json"
    logfile.write_text(json.dumps(sorted(log, key=lambda r: r["order"])))
    config["session"] = dict(
        original=original_path.name,
        sha256=digest(original_path),
        attack_log=logfile.name,
        attack_log_sha256=digest(logfile),
    )
    manifest.write_text(json.dumps(config))
    result = qualify(manifest)
    require(
        "shared session chronological independent attack intervals"
        in result["failures"]
        and "wet" not in result,
        "shifted original attacks",
    )
    return result


def main():
    root = OUT / "recording-controls"
    root.mkdir(exist_ok=True, parents=True)
    results = {"shifted-session": shifted_session(root / "shared-session")}
    for label, altered in (("containers", False), ("segments", True)):
        d = root / label
        d.mkdir(exist_ok=True)
        results[label] = containers(d, altered)
    fs = 48000
    with np.load(OUT / "motion/veto-48000.npz") as z:
        q, d, f = z["q"], z["d"], z["pitch"]
    m = np.memmap(OUT / "motion/veto-48000.f64le", mode="r", dtype="<f8").reshape(
        -1, 30
    )[:, 11]
    q[0] = [-0.0, 0.0, -0.0, 0.0]
    outage = [(round(1.35 * fs), round(1.55 * fs))]
    wet, a, _ = drive(q, d, m, fs, f, outage, moving=True, trusted=True)
    require(
        np.array_equal(wet[a == 0].view("u8"), q[a == 0].view("u8"))
        and np.any(a[:fs] > 0)
        and np.any(a[2 * fs :] > 0),
        "signed bypass entry/exit",
    )
    with np.load(OUT / "motion/fast-retune-48000.npz") as z:
        q, d, f = z["q"], z["d"], z["pitch"]
    q[fs] = [-0.0, 0.0, 1e-27, -0.25]
    w, a, _ = drive(q, d, m, fs, f, moving=True)
    require(
        np.array_equal(w.view("u8"), q.view("u8")) and np.all(a == 0),
        "unknown prerequisite/neutral retune",
    )
    require(nonnegative(np.array([-1e-16]), 1.0, 200)[0] == 0, "roundoff")
    for value in (-0.1, np.nan, np.inf):
        try:
            nonnegative(np.array([value]), 1.0, 200)
        except ValueError:
            continue
        raise ValueError("invalid moment accepted")
    with np.load(OUT / "motion/steady-48000.npz") as data:
        q, f = data["q"], data["pitch"]
    raw = np.memmap(OUT / "motion/steady-48000.f64le", mode="r", dtype="<f8").reshape(
        -1, 30
    )
    d = bank(q, fs, f, np.full(12, 10**0.25 - 1))
    _, scale, _ = drive(q[:, :3], d[:, :3], raw[:, 11], fs, f, moving=False)
    tick = 2 * fs + int(np.argmax(abs(d[2 * fs : 3 * fs, 0])))
    jump = scale[tick] * d[tick, 0]
    require(abs(jump + 0.16903128689) < 1e-6, "historical abrupt neutralization")
    scale[tick:] = 0
    try:
        check_trajectory(scale, round(0.005 * fs))
    except ValueError as err:
        results["abrupt_neutralization"] = {
            "tick": tick,
            "jump_FS": float(jump),
            "rejection": str(err),
        }
    else:
        raise ValueError("bypassed slew")
    reference = 0.0066
    for name in ("always-neutral", "infinitesimal", "single-cell", "stationary-ends"):
        correction = np.zeros(3 * fs)
        if name == "infinitesimal":
            correction[:] = reference * 1e-6
        if name == "single-cell":
            correction[fs : fs + fs // 50] = reference
        if name == "stationary-ends":
            correction[: fs // 2] = correction[5 * fs // 2 :] = reference
        row = witness(correction, fs, reference)
        require(not row["passed"], name)
        results[name] = {
            "active_cells": row["active"],
            "longest_gap_ms": row["longest_gap_ms"],
        }
    results["neutral"] = {
        "signed_zero": True,
        "neutral_retune": True,
        "veto_recovery": True,
    }
    (OUT / "integrity.json").write_text(
        json.dumps(results, allow_nan=False, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
