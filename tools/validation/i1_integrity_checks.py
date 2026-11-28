"""Decoded recording and exact-neutral counterexamples. All generated DI is synthetic."""

import json

import numpy as np
from acoustic_identity import audio
from guide_qualification import digest
from i1_motion import OUT
from i1_recordings import PROTOCOL, decoded
from i1_repeatability import qualify
from i1_safety import drive, nonnegative
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
                "manifest_revision": 1,
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


def main():
    root = OUT / "recording-controls"
    root.mkdir(exist_ok=True, parents=True)
    results = {}
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
