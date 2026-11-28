"""Replay Review 8.1 counterexamples with the pinned entry modules and qualified O2 evidence."""

import argparse
import json
import os
from copy import deepcopy
from dataclasses import replace
from pathlib import Path
import subprocess
import sys
from unittest.mock import patch

ENTRY = "9a54cd721b604731d1af24b1092f2d7b7972ded5"


def run(entry, directory, o2_root, mutant_report):
    for name in ("i1_clock.py", "i1_joined.py", "o2_records.py", "o2_coverage.py"):
        pinned = subprocess.check_output(
            ["git", "show", f"{ENTRY}:tools/validation/{name}"]
        )
        if (entry / name).read_bytes() != pinned:
            raise ValueError("entry source revision " + name)
    os.environ.update(
        ELECTROBOW_O2_OUT=str(o2_root), ELECTROBOW_I1_OUT=str(directory / "i1")
    )
    sys.path.insert(0, str(entry.resolve()))
    import numpy as np
    import o2_bench as bench
    import o2_coverage as coverage
    from i1_clock import Confirm, Frame, Gesture, Launch, due_stamps, replay
    from i1_joined import WetEpoch, advance_lanes, fixture, lane_ticks, returns
    from i1_recordings import PROTOCOL, decoded, inventory
    from o2_audio import physical_key
    from o2_qualification import v17_row
    from o2_records import producer_clock, sample_hash
    from o2_response import require
    from guide_qualification import digest
    from scipy.io import wavfile

    fs = 48000
    directory.mkdir(parents=True, exist_ok=True)
    path = o2_root / "traces" / (physical_key(fs, 196) + ".f64le")
    raw = np.memmap(path, mode="r", dtype="<f8").reshape(-1, 30)
    clock, _ = producer_clock(raw[:, 11], raw[:, 25], fs, 196, ("entry",))

    def score(data, reference, m5, shift):
        return v17_row(
            data[:, 26],
            data[:, 25],
            data[:, 22],
            data[:, 24],
            m5,
            data[:, 21],
            data[:, 3],
            fs,
            196,
            0.5,
            operation="corrected",
            clock=reference,
            settings=("entry",),
            corrected=replace(
                reference, samples=np.roll(data[:, 26], shift), depth=0.5
            ),
            corrected_off=replace(reference, samples=np.roll(data[:, 25], shift)),
        )

    rolled = score(raw, clock, raw[:, 11], 1)
    traces = directory / "traces"
    traces.mkdir(exist_ok=True)
    fake = traces / path.name
    if not fake.exists():
        os.link(path, fake)
    record = json.loads(path.with_suffix(".json").read_text())
    record["taps"]["11"] = sample_hash(raw[:, 25])
    fake.with_suffix(".json").write_text(json.dumps(record))
    alias = np.memmap(fake, mode="r", dtype="<f8").reshape(-1, 30)
    clock, _ = producer_clock(alias[:, 25], alias[:, 25], fs, 196, ("entry",))
    wrong = score(alias, clock, alias[:, 25], 0)
    require(
        all(False not in row["passes"].values() for row in (rolled, wrong)),
        "entry pair repro",
    )
    current = json.loads((o2_root / "results.json").read_text())
    alt = deepcopy(current)
    alt["negatives"] = dict.fromkeys(alt["negatives"], "unexecuted")
    coverage.grade_complete(alt)
    alt = deepcopy(current)
    for rate in (32000, 48000):
        name = f"timing-{rate}.tsv"
        (directory / name).write_bytes((o2_root / name).read_bytes())
    forged = []
    for line in (directory / "timing-32000.tsv").read_text().splitlines():
        run, _, work, batch, frames, ns, h = line.split()
        forged.append(
            f"{run} 192000 {work} {batch} {6 * int(frames)} {6 * int(ns)} "
            + ("-5e-324" if work == "near-denormal" else h)
        )
    (directory / "timing-192000.tsv").write_text("\n".join(forged) + "\n")
    with patch.object(bench, "OUT", directory):
        alt["local_timing"]["rates"]["192000"] = bench.measured_timing(
            192000, current["run_id"]
        )
        coverage.grade_complete(alt)
    mutant = json.loads(mutant_report.read_text())
    require(mutant["status"] == "passed", "entry mutant complete exit")
    with np.load(mutant_report.parent / "motion/vibrato-48000.npz") as z:
        require(
            np.array_equal(z["wet"][:, 3], z["q"][:, 3])
            and np.any(z["wet"][:, 0] != z["q"][:, 0]),
            "entry fourth waveform",
        )
    n = 6100
    frames = {t: Frame(196) for t in range(239, n, 240)}
    ev = {
        0: (Gesture.PICK,),
        5760: (Confirm((0, 1), n), Launch((0, 1), 1, 0.0)),
        6000: (Gesture.HOLD, Gesture.HOLD),
    }
    x = fixture(fs, n)
    log = {
        r["tick"]: r
        for r in replay(x, fs, frames, ev, identity_stamps=due_stamps(n, fs))
    }
    require(
        log[5760]["epoch"] == 1 and log[6000]["phase"] == "hold", "entry event repro"
    )
    state = WetEpoch(fs)
    state.phase = np.nan
    for k in range(state.length):
        amp = 0.2 * np.cos(2 * np.pi * 196 * (k + 1) / fs)
        wet = state.tick(np.full(4, amp), amp, 196, np.zeros(12), True, True, None)
    require(np.isfinite(wet) and np.isnan(state.phase), "entry phase repro")
    carrier, q, depth = returns(fs, n, "entry-overflow")
    bad = carrier.copy()
    bad[:1960, 11] = 1e153
    events = {0: (Gesture.PICK,), 479: (Confirm((0, 1), n), Launch((0, 1), 1, 0.0))}
    try:
        advance_lanes(
            lane_ticks(
                fs,
                x,
                frames,
                events,
                np.zeros(n),
                {1: (bad, q, depth)},
                {},
                (127,),
                due_stamps(n, fs),
                True,
            )
        )
    except ValueError as err:
        overflow = str(err)
        require(
            "unexpected transaction failure nonfinite monitor input" == overflow,
            "entry accumulator site",
        )
    else:
        raise ValueError("entry accumulator unexpectedly survived")
    classes = {}
    for j, c in enumerate("AB"):
        a = 0.1 * np.cos(2 * np.pi * (196 + j) * np.arange(fs) / fs)
        takes = []
        for i in range(4):
            y = a[5 * i :]
            path = directory / f"{c}{i}.wav"
            wavfile.write(path, fs, y)
            takes.append(
                dict(
                    original=path.name,
                    interval=[0, len(y)],
                    rate=fs,
                    take_id=c + str(i),
                    order=i * 2 + 1 + j,
                    attack_record="shifted test crop",
                    original_sha256=digest(path),
                    decoded_sha256=decoded(y),
                    segment_sha256=decoded(y),
                )
            )
        classes[c] = dict(takes=takes)
    _, accepted, fail = inventory(
        dict(classes=classes, protocol_revision=PROTOCOL, manifest_revision=1),
        directory / "manifest.json",
    )
    require(len(accepted) == 8 and not fail, "entry crop reproduction")
    from i1_safety import drive

    freq = np.full(n, 196.0)
    freq[1960:] = 440.0
    _, _, blocks = drive(q, np.zeros_like(q), carrier[:, 11], fs, freq, trusted=True)
    require(
        blocks[1]["end"] == 1960 + 872 and 2 * state.length == 3920,
        "entry split schedule",
    )
    return dict(
        revision=ENTRY,
        rolled_residual_db=rolled["factory_difference_over_off"],
        substituted_m5_db=wrong["final_over_M5"],
        coherent_timing="accepted",
        unexecuted_control="accepted",
        fourth_mutant=mutant["run_id"],
        expired_epoch=log[5760]["epoch"],
        duplicate=log[6000]["phase"],
        poisoned_phase="live",
        overflow=overflow,
        shifted_crops=len(accepted),
        second_close_exclusive=[2 * state.length, blocks[1]["end"]],
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    for key in ("entry_tools", "output", "entry_o2", "mutant_report"):
        parser.add_argument(key, type=Path)
    a = parser.parse_args()
    result = run(a.entry_tools, a.output, a.entry_o2, a.mutant_report)
    (a.output / "counterexamples.json").write_text(
        json.dumps(result, indent=2, allow_nan=False) + "\n"
    )
