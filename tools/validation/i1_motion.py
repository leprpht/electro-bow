"""Stage 08.1 host-time moving projection, four actual Rust electrical returns."""

import json
import os
import subprocess
from itertools import groupby
from pathlib import Path

import numpy as np
from acoustic_observation import rms, tracking
from guide_qualification import digest, trajectory
from i1_safety import drive, smooth_targets
from o2_bench import BINARY, replay
from o2_coverage import RATE, keys
from o2_qualification import v17_row
from o2_records import publish
from o2_response import require

OUT = Path(os.environ.get("ELECTROBOW_I1_OUT", "build/stage-08-1/qualified/i1"))
ALT = np.tile([-5.0, 5.0], 6)
STRESS = ("fast-retune", "max-retune", "bow-controls", "veto", "lift", "held")


def inputs(fs, kind):
    n = fs * (7 if kind == "lift" else 3)
    t = np.arange(n) / fs
    f = 196 * np.exp2(trajectory(t, kind) / 1200)
    v, ratio = np.full(n, 0.1), np.full(n, 1.8)
    if kind == "fast-retune":
        f[fs : fs + fs // 50] = np.linspace(196, 440, fs // 50)
        f[fs + fs // 50 :] = 440
    if kind == "max-retune":
        f = np.minimum(1320.0, 196 * np.exp2(np.maximum(0.0, 4 * (t - 1))))
    if kind == "bow-controls":
        v = np.interp(
            t, [0, 0.8, 0.85, 1.6, 1.65, 3], [0.06, 0.06, 0.16, 0.16, 0.1, 0.1]
        )
        ratio = np.interp(t, [0, 1, 1.05, 2, 2.05, 3], [1.2, 1.2, 2.4, 2.4, 1.8, 1.8])
    k = v * ratio / 1.65
    if kind == "lift":
        k[fs:] = 0
    depth = (
        np.full(n, 0.5)
        if kind in ("steady", "held")
        else np.interp(t, [0, 0.5, 1, 1.5, 2, 3], [0, 1, 0, 1, 0, 0.5])
    )
    if kind in ("vibrato", "fast-retune", "veto"):
        fast = (t >= 1) & (t < 1.6)
        depth[fast] = (np.arange(n)[fast] // 7) % 2
    return np.column_stack((f, v, k)), depth


def physical(ctl, fs, name, directory):
    path = directory / (name + ".f64le")
    with path.open("wb") as stream:
        proc = subprocess.run(
            [BINARY, "--o2-guide", str(fs)],
            input=ctl.astype("<f8").tobytes(),
            stdout=stream,
            stderr=subprocess.PIPE,
            check=True,
        )
    counts = list(map(int, proc.stderr.split()))
    publish(path, BINARY, ctl, fs, ctl[0, 0], counts)
    return np.memmap(path, mode="r", dtype="<f8").reshape(-1, 30)


def bank(q, fs, f, gains):
    data = np.column_stack((f, q, np.broadcast_to(gains, (len(q), 12))))
    proc = subprocess.run(
        [OUT / "motion-bank", str(fs)],
        input=data.astype("<f8").tobytes(),
        capture_output=True,
        check=True,
    )
    taps = np.frombuffer(proc.stdout, "<f8").reshape(len(q), 6)
    require(
        np.isfinite(taps).all()
        and np.all(taps[:, 4] <= np.maximum.accumulate(taps[:, 5]) + 1e-12),
        "bank numerical state",
    )
    return taps[:, :4]


def prerequisite(raw, fs, f, q=None, m5=None, epoch=True):
    m = raw[:, 11] if m5 is None else m5
    q = raw[:, 25:28] if q is None else q
    rows = [
        v17_row(
            q[:, i],
            q[:, 0],
            raw[:, 22],
            raw[:, 24],
            m,
            raw[:, 21],
            raw[:, 3],
            fs,
            f,
            d,
            operation="neutral",
        )
        for i, d in enumerate((0, 0.5, 1))
    ]
    failures = [
        f"{i}/{g}"
        for i, row in enumerate(rows)
        for g, v in row["passes"].items()
        if v is False
    ]
    return {
        "passed": bool(epoch and not failures),
        "failures": failures if epoch else ["invalid epoch", *failures],
        "rows": rows,
    }


def witness(difference, fs, stationary):
    cells = [
        rms(
            difference[
                round((0.5 + i * 0.02) * fs) : round((0.5 + (i + 1) * 0.02) * fs)
            ]
        )
        for i in range(100)
    ]
    active = np.asarray(cells) >= stationary * 0.5
    gap = max((len(list(g)) for k, g in groupby(active) if not k), default=0)
    return {
        "cell_rms": cells,
        "active": int(sum(active)),
        "longest_gap_ms": 20 * gap,
        "reference": stationary,
        "passed": bool(stationary > 1e-4 and sum(active) >= 75 and gap <= 3),
    }


def defect_rows(raw, fs):
    n = np.arange(len(raw))
    q = raw[:, 25:28]
    scale = np.sqrt(2) * rms(q[-fs:, 0])
    h1 = np.cos(2 * np.pi * 196 * n / fs)
    h2 = np.cos(2 * np.pi * 392 * n / fs)
    signals = {
        "199Hz": np.cos(2 * np.pi * 199 * n / fs),
        "dominant-H2": 0.5 * h1 + 0.8 * h2,
        "missing-H1": h2,
        "weak-H1": 0.02 * h1 + h2,
        "Nyquist": 0.5 * h1 + 0.8 * (-1.0) ** n,
    }
    rows = {
        name: prerequisite(raw, fs, 196, np.repeat((scale * x)[:, None], 3, axis=1))
        for name, x in signals.items()
    }
    for name, m in (("invalid-anchor", scale * h2), ("ineligible", np.zeros(len(n)))):
        rows[name] = prerequisite(raw, fs, 196, m5=m)
    rows["invalid-epoch"] = prerequisite(raw, fs, 196, epoch=False)
    require(
        all(not row["passed"] for row in rows.values()), "escaped neutral prerequisite"
    )
    counter = np.repeat((scale * signals["199Hz"])[:, None], 3, axis=1)
    counter = np.column_stack((counter, counter[:, 1]))
    d = bank(
        counter,
        fs,
        np.full(len(raw), 196.0),
        np.expm1(np.log(10) * np.r_[2.0, np.zeros(11)] / 20),
    )
    w, a, _ = drive(
        counter, d, raw[:, 11], fs, 196.0, moving=True, trusted=rows["199Hz"]["passed"]
    )
    old, oa, _ = drive(counter[:, :3], d[:, :3], raw[:, 11], fs, 196.0)
    require(
        np.array_equal(w, counter) and max(a) == 0 and oa[-1] > 0,
        "detuned upstream escape",
    )
    rows["199Hz"].update(
        old_scale=float(oa[-1]),
        old_wet_rms=rms(old[:, 0] - counter[:, 0]),
        revised_scale=float(a[-1]),
        revised_wet_rms=rms(w[:, 0] - counter[:, 0]),
    )
    return rows


def measure(fs, kind, ref, trusted, directory):
    ctl, depth = inputs(fs, kind)
    name = f"{kind}-{fs}"
    raw = physical(ctl, fs, name, directory)
    f = ctl[:, 0]
    q = np.column_stack((raw[:, 25:28], replay(fs, raw[:, 21], depth)[:, 3]))
    if kind in ("steady", "held"):
        require(np.array_equal(q[:, 1].view("u8"), q[:, 3].view("u8")), "delivery tap")
    g = np.expm1(np.log(10) * ALT / 20)
    outage = []
    if kind == "veto":
        g = smooth_targets(len(raw), fs, [(0, ALT), (fs, -ALT), (2 * fs, ALT)])
        outage = [(round(1.35 * fs), round(1.55 * fs))]
    d = bank(q, fs, f, g)
    old, old_a, _ = drive(q[:, :3], d[:, :3], raw[:, 11], fs, f, outage)
    if kind == "steady":
        ref = rms(old[-fs:, 0] - q[-fs:, 0])
        neutral = prerequisite(raw, fs, 196)
        trusted = neutral["passed"]
    wet, a, logs = drive(q, d, raw[:, 11], fs, f, outage, moving=True, trusted=trusted)
    trace = directory / (name + ".npz")
    np.savez(
        trace,
        pitch=f,
        depth=depth,
        request=g,
        q=q,
        d=d,
        wet=wet,
        scale=a,
        old_scale=old_a,
    )
    correction = wet - q
    result = {
        "physical": digest(Path(raw.filename)),
        "samples": len(q),
        "trace": digest(trace),
        "monitor": logs,
        "delta_rms": [rms(s) for s in correction.T],
        "old_rms": rms(old[:, 0] - q[:, 0]),
        "active_samples": float(np.mean(correction[:, 0] != 0)),
        "max_step": float(np.max(abs(np.diff(wet, axis=0)))),
        "peak": float(np.max(abs(wet))),
        "paired_ratio": rms(wet[:, 1] - wet[:, 0]) / rms(wet[:, 0]),
        "longest_neutral_samples": max(
            (len(list(g)) for k, g in groupby(a == 0) if k), default=0
        ),
    }
    if kind == "steady":
        result.update(reference=ref, prerequisite=neutral)
        if fs == 48000:
            result["upstream_controls"] = defect_rows(raw, fs)
    if kind in ("vibrato", "bend"):
        result.update(
            tracking=tracking(wet[:, 0], fs, 196, kind),
            neutral_tracking=tracking(q[:, 0], fs, 196, kind),
        )
    if kind == "vibrato":
        result.update(
            witness=witness(correction[:, 0], fs, ref),
            old_witness=witness(old[:, 0] - q[:, 0], fs, ref),
        )
    if outage:
        stop = outage[0][1]
        result["recovery_samples"] = int(np.flatnonzero(a[stop:] > 0)[0])
    if kind == "lift":
        e = np.clip(1 - (np.arange(len(q)) - fs + 1) / round(0.1 * fs), 0, 1)
        result["lift_rms"] = [rms(wet[fs // 2 : fs, 3]), rms(wet[-fs:, 3])]
        result["after_release_peak"] = float(max(abs((e * wet[:, 3])[e == 0])))
    return result


def build_bank():
    OUT.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "clang++",
            "-std=c++17",
            "-O3",
            "-Wall",
            "-Wextra",
            "-Werror",
            Path(__file__).with_name("i1_motion_bank.cpp"),
            "-o",
            OUT / "motion-bank",
        ],
        check=True,
    )


def main():
    directory = OUT / "motion"
    directory.mkdir(parents=True, exist_ok=True)
    build_bank()
    rows = {}
    for fs in RATE:
        base = measure(fs, "steady", 0, False, directory)
        rows[f"steady-{fs}"] = base
        for kind in ("vibrato", "bend", *(STRESS if fs == 48000 else ())):
            rows[f"{kind}-{fs}"] = measure(
                fs, kind, base["reference"], base["prerequisite"]["passed"], directory
            )
        print(fs, rows[f"vibrato-{fs}"]["witness"]["active"], flush=True)
    keys(
        rows,
        [f"{k}-{fs}" for fs in RATE for k in ("steady", "bend", "vibrato")]
        + [f"{k}-48000" for k in STRESS],
        "motion inventory",
    )
    report = {
        "rows": rows,
        "motion_pass": all(
            rows[f"vibrato-{fs}"]["witness"]["passed"]
            and rows[f"steady-{fs}"]["prerequisite"]["passed"]
            for fs in RATE
        ),
    }
    (OUT / "motion.json").write_text(
        json.dumps(report, allow_nan=False, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
