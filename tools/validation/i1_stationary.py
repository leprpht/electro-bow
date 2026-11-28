"""Regrade the revised permission on the frozen 36-case inventory and six DI takes."""

import json
from itertools import combinations, product

import numpy as np
from acoustic_identity import FIXTURES, audio, distances
from acoustic_observation import rms
from i1_acceptance import grade_depths, stationary_inventory
from i1_motion import OUT, prerequisite
from i1_repeatability import representation
from i1_safety import smooth_targets, transfer
from i1_stream import render
from i1_study import (
    VECTORS,
    carrier,
    harmonics,
    inspect_row,
    source,
    swept_output,
    updates_for,
)
from o2_bench import replay
from o2_qualification import strongest_hz
from o2_records import sample_hash
from o2_response import reference, require


def grade_case(raw, fs, f, update, name, upstream, q):
    if not upstream["passed"]:
        return {"upstream": upstream, "eligible": False}
    gains = (
        update
        if isinstance(update, np.ndarray)
        else smooth_targets(len(raw), fs, update)
    )
    wet, a, log, d = render(
        q, raw[:, 11], fs, f, gains, upstream["passed"], settings=(name,)
    )
    rows = inspect_row(raw, wet, fs, f, name)
    failures = [
        f"{i}/{g}"
        for i, r in enumerate(rows)
        for g, v in r["passes"].items()
        if v is False
    ]
    if not failures:
        grade_depths(rows, fs, f, rms(raw[-fs:, 11]), [rms(y) for y in q[-fs:, :3].T])
    final = gains[-1] if gains.ndim == 2 else gains
    freq = np.arange(65.0, 4000.25, 0.25)
    peaks = []
    for dep in (0, 0.5, 1):
        h = transfer(fs, f, freq, a[-1] * final) * reference(fs, freq, dep)
        idx = np.argmax(abs(h))
        peaks.append([freq[idx], 20 * np.log10(abs(h[idx]))])
    if max(p[1] for p in peaks) > 6:
        failures.append("narrow peak")
    data = {
        "eligible": True,
        "sweep": peaks,
        "failures": failures,
        "depths": rows,
        "neutral": upstream["rows"],
        "requested_dB": (20 * np.log10(1 + final)).tolist(),
        "accepted_dB": (20 * np.log10(1 + a[-1] * final)).tolist(),
        "wet": sample_hash(wet),
        "delta": sample_hash(d),
        "requested": sample_hash(gains),
        "scale": sample_hash(a),
        "timeline": log,
    }
    return data


def main():
    rows, raws, sources = {}, {}, {}

    def load(fs, f, v, r):
        key = (fs, f, v, r)
        if key not in raws:
            raw = carrier(fs, f, v, r)
            q = np.column_stack(
                (raw[:, 25:28], replay(fs, raw[:, 21], np.full(len(raw), 0.5))[:, 3])
            )
            require(np.array_equal(q[:, 1], q[:, 3]), "constant delivery")
            raws[key] = raw, prerequisite(raw, fs, f), q
        return raws[key]

    for name, (fs, f, v, r, vector) in sorted(stationary_inventory().items()):
        raw, upstream, q = load(fs, f, v, r)
        update = (
            np.expm1(np.log(10) * VECTORS[vector] / 20)
            if vector in VECTORS
            else updates_for(source(fs, f, vector), fs, f)
        )
        rows[name] = grade_case(raw, fs, f, update, name, upstream, q)
    fs, f = 48000, 196
    raw, upstream, q = load(fs, f, 0.1, 1.8)
    anchor = strongest_hz(raw[-fs:, 11], fs)
    clips = {}
    for player, passage in product((1, 2), (60, 132, 204)):
        name = f"techs-P{player}-{passage}"
        rate, x = audio(FIXTURES / (name + ".wav"))
        x = x[np.flatnonzero(abs(x) > 0.01)[0] :]
        updates = [(round(t * fs / rate), g) for t, g in updates_for(x, rate, f)]
        row = grade_case(raw, fs, f, updates, name, upstream, q)
        if not row["eligible"]:
            sources[name] = row
            continue
        gains = smooth_targets(len(q), fs, updates)
        wet, _, _, _ = render(
            q, raw[:, 11], fs, f, gains, upstream["passed"], settings=(name,)
        )
        require(sample_hash(wet) == row["wet"], "source wet reconstruction")
        sources[name] = {
            "row": row,
            "held": representation(harmonics(wet[:, 0], fs, anchor)),
        }
        if passage == 60:
            clips[name + "-held"] = wet[-fs:, 0]
    pairs = {}
    for a, b in combinations(sources, 2):
        if "held" in sources[a] and "held" in sources[b]:
            x, y = sources[a]["held"], sources[b]["held"]
            pairs[a + "/" + b] = {
                "distance": rms(np.subtract(x["vector"], y["vector"])),
                "signed": np.subtract(x["cues"], y["cues"]).tolist(),
            }
    np.savez(OUT / "revised-held.npz", **clips)
    safe = all(row["eligible"] and not row["failures"] for row in rows.values())
    sweep = swept_output(rows) if safe else None

    def row(speed, pressure):
        return rows[f"48000/440/{speed}/{pressure}/opposed"]

    def pressure(field):
        return (
            row(0.1, 2.4)[field][0]["spectral"]["high_fraction"]
            - row(0.1, 1.2)[field][0]["spectral"]["high_fraction"]
        )

    def speed(field):
        return 20 * np.log10(
            row(0.16, 1.8)[field][0]["rms_FS"] / row(0.06, 1.8)[field][0]["rms_FS"]
        )

    expr = {}
    if safe:
        expr = {
            "pressure": [pressure(k) for k in ("neutral", "depths")],
            "speed_dB": [speed(k) for k in ("neutral", "depths")],
        }
        orig, p = expr["pressure"]
        orig_speed, v = expr["speed_dB"]
        spread = [
            np.ptp(
                [
                    20 * np.log10(r["depths"][i]["rms_FS"])
                    for k, r in rows.items()
                    if "/1318.51/" in k
                ]
            )
            for i in range(3)
        ]
        ids = [
            rows["48000/196/0.1/1.8/" + key]["depths"][0]["harmonics"]["peak_FS"]
            for key in ("bright", "dark")
        ]
        expr.update(
            high_note_spread_dB=spread,
            synthetic_distance_dB=distances(*map(np.asarray, ids)),
        )
        safe = (
            p * orig > 0
            and abs(p) >= 0.5 * abs(orig)
            and v >= 0.75 * orig_speed
            and max(spread) < 3
            and expr["synthetic_distance_dB"] >= 1
        )
    result = {
        "expression": expr,
        "rows": rows,
        "sources": sources,
        "pairs": pairs,
        "sweep": sweep,
        "passed": bool(safe)
        and all(v.get("row", {}).get("failures") == [] for v in sources.values()),
    }
    (OUT / "stationary.json").write_text(
        json.dumps(result, allow_nan=False, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
