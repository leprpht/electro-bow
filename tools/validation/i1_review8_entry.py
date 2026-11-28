"""Reproduce the Review 8 exit defects using only the pinned entry checkout."""

import argparse
import json
import os
import subprocess
import sys
from copy import deepcopy
from dataclasses import replace
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("checkout", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    checkout = args.checkout.resolve()
    entry = "ef0e63177ec11390a9604d46d7265a4ec39076cb"
    if (
        subprocess.check_output(
            ["git", "-C", checkout, "rev-parse", "HEAD"], text=True
        ).strip()
        != entry
    ):
        raise ValueError("requires exact Review 8 entry")
    os.chdir(checkout)
    sys.path[0] = str(checkout / "tools/validation")
    os.environ["ELECTROBOW_O2_OUT"] = str(out / "o2")
    import i1_clock as clock

    clock.OUT = out / "i1"
    import i1_acceptance as audit
    import i1_safety as safety
    import i1_study as study
    import numpy as np
    import o2_coverage as coverage
    import o2_qualification as o2
    from i1_repeatability import qualify
    from scipy.io import wavfile

    facts = {}
    original_exit = o2.grade_complete

    def interrupt(measured):
        original_exit(measured)
        raise KeyboardInterrupt("during final grade")

    o2.grade_complete = interrupt
    try:
        o2.main()
    except KeyboardInterrupt:
        report = json.loads((o2.OUT / "results.json").read_text())
        assert report["neutral_candidate_passed"] and not report.get("error")
        facts["R8-2"] = {
            "status_after_interruption": True,
            "error": report.get("error"),
        }
    study.main()
    old = json.loads((clock.OUT / "i1-results.json").read_text())
    facts["stationary_depths"] = sum(
        len(c["depths"]) for c in old["stationary"].values()
    )
    facts["historical_failures"] = report["failed_predicates"]
    changed = deepcopy(report)
    changed["sweep"]["0.5"].update(
        maximum={"hz": 65, "dB": -100, "width_hz": None},
        maximum_dB=-100,
        complex_error=0,
        peaks_above_6dB=[],
    )
    coverage.grade_complete(changed)
    facts["R8-1"] = {
        "accepted": changed["sweep"]["0.5"],
        "actual": report["sweep"]["0.5"],
    }
    changed = deepcopy(report)
    rates = changed["local_timing"]["rates"]
    rates["192000"] = deepcopy(rates["32000"])
    for row in rates["192000"]["records"]:
        if row[0] == "near-denormal":
            row[3] = 5e-324
    coverage.grade_complete(changed)
    facts["R8-4"] = (
        "32 kHz substituted at 192 kHz, arbitrary positive subnormal accepted"
    )
    fs = 48000
    raw = study.carrier(fs, 196)
    q, m = raw[:, 25:28], raw[:, 11]
    ref = o2.reference_trace(
        m, q[:, 0], fs, 196, "review replay", "first launch", ("unity",)
    )
    shifted = replace(
        ref, start_tick=1, end_tick=len(q) + 1, window=(len(q) - fs + 1, len(q) + 1)
    )
    rw = o2.v17_row(
        q[:, 1],
        q[:, 0],
        raw[:, 22],
        raw[:, 24],
        m,
        raw[:, 21],
        raw[:, 3],
        fs,
        196,
        0.5,
        operation="corrected",
        clock=shifted,
        corrected_off=replace(shifted, samples=np.roll(q[:, 0], -1)),
        corrected=replace(shifted, samples=np.roll(q[:, 1], -1), depth=0.5),
    )
    assert all(v is not False for v in rw["passes"].values())
    facts["R8-3"] = rw
    n = np.arange(len(q))
    tone = (
        np.sqrt(2)
        * np.sqrt(np.mean(q[-fs:, 0] ** 2))
        * np.cos(2 * np.pi * 199 * n / fs)
    )
    fake = np.tile(tone[:, None], (1, 3))
    coeff = np.expm1(np.log(10) * np.r_[2.0, np.zeros(11)] / 20)
    w, a, logs = safety.run(fake.T, m, fs, 196, coeff)

    def neutral(wet):
        return o2.v17_row(
            wet,
            tone,
            raw[:, 22],
            raw[:, 24],
            m,
            raw[:, 21],
            raw[:, 3],
            fs,
            196,
            0,
            operation="neutral",
        )

    facts["R8-5"] = {
        "scale": float(a[-1]),
        "neutral": neutral(tone),
        "colored": neutral(w[:, 0]),
        "realized_failures": sum(not r["realized_ok"] for r in logs),
    }
    facts["R8-6"] = old["dynamic"]["vibrato"]
    assert facts["R8-6"]["delta_rms"] == 0
    case = "48000/196/0.1/1.8/opposed"
    forged = deepcopy(old)
    forged["stationary"][case]["depths"][1].update(
        factory_wet_difference_rms_FS=0, factory_difference_over_off=-240
    )
    forged["stationary"][case]["depths"][0]["harmonics"] = dict.fromkeys(
        ("peak_FS", "phase_deg", "dBr"), []
    )
    forged["measured_sweep"]["weak-H1/0.5"].update(
        peak={"hz": -1, "dB": -200, "width_hz": None}, complex_error=0
    )
    for r in forged["dynamic"].values():
        r["timeline"] = r["timeline"][:1]
    forged["negatives"] = dict.fromkeys(forged["negatives"], False)
    assert not study.grade(forged)[0]
    audit.audit(forged)
    facts["R8-7"] = (
        "accepted coherent residual, empty arrays, invented sweep, truncated timelines and false controls"
    )
    x = study.source(fs, 196, "bright")[:24000]
    frames = {t: clock.Frame(196) for t in range(239, len(x), 240)}
    expired = clock.replay(
        x, fs, frames, {0: ("pick",), 5760: ("timeout",), 23900: (("launch", (0, 1)),)}
    )
    replace_rows = clock.replay(
        x,
        fs,
        frames,
        {
            0: ("pick",),
            2399: (("launch", (0, 1)),),
            7440: ("pick",),
            8400: (("launch", (0, 2)),),
        },
    )
    facts["R8-8"] = {
        "expired_publications": sum(
            r["tick"] > 5760 and "publish" in r["actions"] for r in expired
        ),
        "expired_launch": next(r for r in expired if r["tick"] == 23900),
        "replacement": next(r for r in replace_rows if r["tick"] == 8400),
    }
    fault = clock.replay(
        x, fs, frames, {0: ("pick",), 2399: (("launch", (0, 1)),), 7500: ("fault",)}
    )
    facts["R8-9"] = next(r for r in fault if r["tick"] == 7500)
    assert (
        not {"committed_physical", "cached_product", "fade_index"}
        & facts["R8-9"].keys()
    )
    classes = {}
    for c, factor in (("A", 1.65), ("B", 0.35)):
        h = 0.12 / np.arange(1, 13)
        h[5] *= factor
        y = (
            np.cos(2 * np.pi * 196 * np.arange(fs)[:, None] * np.arange(1, 13) / fs) @ h
        ).astype("f4")
        takes = []
        for i in range(4):
            path = out / f"{c}{i}.wav"
            wavfile.write(path, fs, y)
            with path.open("ab") as f:
                f.write(i.to_bytes(2, "little"))
            takes.append(path.name)
        classes[c] = {
            "pickup": c,
            "string": "G",
            "fret": 0,
            "guitar": "synthetic",
            "chain": "generated",
            "gesture": "synthetic",
            "performer": "generator",
            "gain": 1,
            "takes": takes,
        }
    path = out / "container-manifest.json"
    path.write_text(json.dumps({"hz": 196, "comparison": "pickup", "classes": classes}))
    dup = qualify(path)
    assert "score" in dup
    facts["R8-10"] = dup
    (out / "entry.json").write_text(json.dumps(facts, indent=2, allow_nan=False) + "\n")


if __name__ == "__main__":
    main()
