"""Apply the preregistered calibration/withheld test to a labeled recording cell."""

import argparse
import json
import subprocess
from itertools import combinations, product
from pathlib import Path

import numpy as np
from acoustic_identity import audio
from acoustic_observation import db, partials, rms
from guide_qualification import digest
from i1_clock import OUT
from i1_safety import run, smooth_targets
from i1_study import O2, cues, harmonics, inspect_row, spectrum, updates_for
from o2_response import require


def representation(h):
    v = spectrum(h)
    floor = max(1e-5, 3 * 0.00032, 0.015 * max(h))
    usable = h > floor
    return {
        "vector": (v - v.mean()).tolist(),
        "cues": cues(h),
        "eligible": [bool(usable[4:7].all()), bool(usable[:10].all())],
        "orders": usable.tolist(),
        "bands": [
            float(db(np.linalg.norm(h[a:b]) / np.linalg.norm(h)))
            for a, b in ((0, 2), (2, 4), (4, 8), (8, 12))
        ],
    }


def discrimination(records):
    def dist(a, b):
        return rms(np.subtract(records[a]["vector"], records[b]["vector"]))

    out = {}
    for split, idx in (("calibration", (1, 2)), ("withheld", (3, 4))):
        groups = [[f"{c}{i}" for i in idx] for c in ("A", "B")]
        within = [dist(a, b) for group in groups for a, b in combinations(group, 2)]
        between = [dist(a, b) for a, b in product(*groups)]
        out[split] = {
            "within": within,
            "between": between,
            "margin": min(between) - max(within),
        }
    centroids = {
        c: np.mean([records[f"{c}{i}"]["vector"] for i in (1, 2)], axis=0)
        for c in ("A", "B")
    }
    out["matches"] = {
        c + str(i): bool(
            rms(records[c + str(i)]["vector"] - centroids[c])
            < rms(records[c + str(i)]["vector"] - centroids[other])
        )
        for c, other in (("A", "B"), ("B", "A"))
        for i in (3, 4)
    }
    return out


def stable_cue(records, k, indices, threshold):
    rows = [records[c + str(i)] for c, i in product(("A", "B"), indices)]
    if not all(r["eligible"][k] for r in rows):
        return 0
    diff = [
        records["A" + str(i)]["cues"][k] - records["B" + str(j)]["cues"][k]
        for i, j in product(indices, repeat=2)
    ]
    return 1 if min(diff) >= threshold else -1 if max(diff) <= -threshold else 0


def qualify(manifest):
    # One cell per invocation. Only ordinal takes 1/2 are used to nominate a cue.
    config = json.loads(manifest.read_text())
    pitch = config["hz"]
    require(65 <= pitch <= 1320, "unsupported pitch")
    cell = config["classes"]
    require(
        set(cell) == {"A", "B"}
        and config["comparison"] in ("pickup", "string", "guitar"),
        "class labels",
    )
    required = (
        "guitar",
        "pickup",
        "string",
        "fret",
        "chain",
        "gain",
        "gesture",
        "performer",
    )
    varies = {
        "pickup": {"pickup"},
        "string": {"string", "fret"},
        "guitar": {"guitar", "pickup", "string", "fret"},
    }[config["comparison"]]
    for field in required:
        require(
            all(str(cell[c][field]).strip() for c in cell), "missing label " + field
        )
        if field not in varies:
            require(cell["A"][field] == cell["B"][field], "uncontrolled " + field)
    require(
        cell["A"][config["comparison"]] != cell["B"][config["comparison"]],
        "identical class",
    )
    takes = {}
    inputs = {}
    used = set()
    for c in ("A", "B"):
        require(len(cell[c]["takes"]) == 4, "four independent takes per setting")
        for i, filename in enumerate(cell[c]["takes"], 1):
            path = manifest.parent / filename
            rate, x = audio(path)
            sha = digest(path)
            require(
                sha not in used
                and x.ndim == 1
                and np.isfinite(x).all()
                and max(abs(x)) < 1,
                "invalid/duplicate take",
            )
            used.add(sha)
            starts = np.flatnonzero(abs(x) > 0.01)
            require(len(starts) > 0, "absent attack")
            x = x[starts[0] :]
            require(len(x) >= round(0.30 * rate), "incomplete DI")
            segment = x[round(0.08 * rate) : round(0.23 * rate)]
            hzgrid = np.arange(
                pitch * 2 ** (-40 / 1200), pitch * 2 ** (40 / 1200) + 0.025, 0.025
            )
            hz = float(
                hzgrid[
                    np.argmax([abs(partials(segment, rate, f, 1)[0]) for f in hzgrid])
                ]
            )
            require(abs(1200 * np.log2(hz / pitch)) <= 15, "DI pitch mismatch")
            take = c + str(i)
            h = abs(partials(segment, rate, hz))
            require(len(h) == 12 and max(h) > 1e-5, "DI harmonic eligibility")
            inputs[take] = representation(h)
            takes[take] = (rate, x, hz, sha)
    require(
        all(
            abs(1200 * np.log2(takes["A" + str(i)][2] / takes["B" + str(j)][2])) <= 10
            for i, j in product(range(1, 5), repeat=2)
        ),
        "between-take pitch mismatch",
    )
    di = discrimination(inputs)
    signs = [stable_cue(inputs, k, (1, 2), 2) for k in (0, 1)]
    repeat = all(di[s]["margin"] >= 1 for s in ("calibration", "withheld")) and all(
        di["matches"].values()
    )
    signs = [
        s if s and stable_cue(inputs, k, (1, 2, 3, 4), 2) == s else 0
        for k, s in enumerate(signs)
    ]
    report = {
        "di": di,
        "registered_signs": signs,
        "inputs": inputs,
        "source_hashes": {k: t[3] for k, t in takes.items()},
    }
    if not repeat or not any(signs):
        return dict(report, status="DI discrimination ineligible")
    fs = 48000
    controls = np.tile([pitch, 0.1, 1.8 * 0.1 / 1.65], (3 * fs, 1)).astype("<f8")
    proc = subprocess.run(
        [O2 / "render", "--o2-guide", str(fs)],
        input=controls.tobytes(),
        capture_output=True,
        check=True,
    )
    require(
        list(map(int, proc.stderr.split())) == [3 * fs, 1, 0, 1, 3 * fs, 3 * fs],
        "reference clock",
    )
    raw = np.frombuffer(proc.stdout, "<f8").reshape(3 * fs, 30)
    from o2_qualification import strongest_hz

    anchor = strongest_hz(raw[-fs:, 11], fs)
    wet = {}
    for name, (rate, x, hz, _) in takes.items():
        updates = updates_for(x, rate, pitch)
        require(any(np.any(g) for _, g in updates), "missing capture")
        gains = smooth_targets(
            len(raw), fs, [(round(n * fs / rate), g) for n, g in updates]
        )
        out, _, _ = run(raw[:, 25:28].T, raw[:, 11], fs, pitch, gains)
        for row in inspect_row(raw, out, fs, pitch, name):
            require(
                all(v is not False for v in row["passes"].values()), "unsafe held wet"
            )
        wet[name] = representation(harmonics(out[:, 0], fs, anchor))
    score = discrimination(wet)
    preserved = all(
        not s or s == stable_cue(wet, k, (3, 4), 0.5) for k, s in enumerate(signs)
    )
    passed = (
        all(score[s]["margin"] >= 0.5 for s in ("calibration", "withheld"))
        and all(score["matches"].values())
        and preserved
    )
    return dict(
        report,
        status="passed" if passed else "held discrimination failed",
        wet=wet,
        score=score,
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    result = {"status": "failed"}
    try:
        result = qualify(args.manifest)
    except Exception as exc:
        result["error"] = str(exc)
        raise
    finally:
        (OUT / "controlled.json").write_text(
            json.dumps(result, allow_nan=False, indent=2) + "\n"
        )
    require(result["status"] == "passed", result["status"])
