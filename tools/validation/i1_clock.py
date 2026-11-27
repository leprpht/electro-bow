"""Offline sample-clock reference. Controller decisions and Hz are injected."""

import json
import math
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from acoustic_identity import analysis_stream, feature
from o2_response import require

OUT = Path("build/stage-08")
PRIORITY = ("reset", "fault", "pick", "mute", "change", "support", "release")
ZERO = np.zeros(12)


def rounded(x):
    return math.floor(x + 0.5)


@dataclass(frozen=True)
class Frame:
    hz: float
    quality: float = 0.95
    amplitude: float = 0.08
    floor: float = 0.00032
    supported: bool = True
    movement: float = 0.5
    owner: tuple = (0, 1)
    compatible: bool = True

    def good(self):
        return (
            self.supported
            and self.compatible
            and 65 <= self.hz <= 1320
            and 0.74 <= self.quality <= 1
            and math.isfinite(self.amplitude)
            and self.amplitude >= max(0.001, 3 * self.floor)
            and 0.0001 <= self.floor <= 0.002
            and 0 <= self.movement <= 1
        )


def replay(x, fs, frames, events, partitions=(127,), defect=None):
    """Restart FIR phase at reset. Vectorized causal FIR is indexed only through n."""
    d = int(fs // 16000)
    fa = fs / d
    hop = d * rounded(0.005 * fa)
    da = int(fs // 8000)
    ha = da * rounded(0.005 * fs / da)
    budget = math.ceil((math.ceil(4 * fa / 65) + 1) / hop)
    origin = 0
    z, _ = analysis_stream(x, fs)
    generation = serial = 0
    owner = live = None
    anchor = -1
    held = ZERO.copy()
    published = ZERO.copy()
    job = None
    published_end = None
    phase = "idle"
    history = {}
    log = []
    require(
        partitions and all(p >= 0 for p in partitions) and any(partitions),
        "nonadvancing partition schedule",
    )
    n = block = 0
    while n < len(x):
        end = min(n + partitions[block % len(partitions)], len(x))
        block += 1
        for tick in range(n, end):
            actions = events.get(tick, ())
            action = next((a for a in PRIORITY if a in actions), None)
            note = []
            consumed = None
            if action == "reset":
                origin = tick
                z, _ = analysis_stream(x[tick:], fs)
                generation += 1
                owner = live = job = None
                phase = "idle"
                held = ZERO.copy()
                published = ZERO.copy()
                history.clear()
            elif action in ("fault", "release", "mute"):
                owner = None
                phase = "release"
                job = None
                published = ZERO.copy()
            elif action in ("pick", "change"):
                serial += 1
                owner = (generation, serial)
                phase = "pending"
                published_end = None
                anchor = tick
                published = ZERO.copy()
                job = None
                history.clear()
            age = tick - origin
            if (age + 1) % ha == 0:
                history[tick] = frames.get(tick, Frame(0, supported=False))
                # A gap invalidates even a job whose snapshot was already taken.
                if (
                    not history[tick].good()
                    or history[tick].owner != owner
                    or (
                        job and abs(1200 * math.log2(history[tick].hz / job["hz"])) > 25
                    )
                ):
                    job = None
                history.pop(
                    tick - (math.ceil((4 / 65 + 0.035) * fs / ha) + 3) * ha, None
                )
            if "dispose" in actions or action == "fault":
                if owner == live:
                    owner = job = None
                    published = ZERO.copy()
                    phase = "idle"
                live = None
                held = ZERO.copy()
            if ("launch", owner) in actions and owner and phase == "pending":
                live = owner
                phase = "active"
                if published_end is None or tick - published_end > 2 * hop:
                    published = ZERO.copy()
                held = published.copy()
                consumed = live
            if job and tick == job["done"]:
                job["complete"] = True
                note.append("complete")
            due = (age + 1) % hop == 0
            if due and job:
                last = max(history, default=-1)
                ok = (
                    job["owner"] == owner
                    and job["pub"] == tick
                    and job["complete"]
                    and last >= 0
                    and history[last].good()
                    and abs(1200 * np.log2(history[last].hz / job["hz"])) <= 25
                )
                if ok:
                    published_end = tick - hop
                    published += job["alpha"] * (job["target"] - published)
                    if phase == "active" and owner == live:
                        held = published.copy()
                        consumed = live
                    note.append("publish")
                else:
                    note.append("discard")
                job = None
            snapshot = None
            if due and owner and history:
                last = max(history)
                frame = history[last]
                if frame.good() and frame.owner == owner:
                    m = rounded(4 * fa / frame.hz)
                    first_end = tick - (m - 1) * d
                    raw_first = first_end - 32 * d
                    center = first_end - 16 * d
                    baseline = origin + ((center - origin + 1) // ha) * ha - 1
                    support = range(baseline, last + 1, ha)
                    ok = (
                        raw_first >= origin
                        and raw_first >= anchor
                        and center - anchor >= math.ceil(0.025 * fs)
                        and all(
                            u in history
                            and history[u].good()
                            and history[u].owner == owner
                            and abs(1200 * math.log2(history[u].hz / frame.hz)) <= 25
                            for u in support
                        )
                    )
                    if ok:
                        j = (age + 1) // d
                        target = feature(z[j - m : j], fa, frame.hz, frame.floor)
                        if target is not None:
                            done = tick + math.ceil(m / budget) - 1
                            job = {
                                "owner": owner,
                                "pub": tick + hop,
                                "done": done,
                                "complete": done == tick,
                                "target": target,
                                "hz": frame.hz,
                                "alpha": -math.expm1(
                                    -hop / (fs * (0.060 - 0.040 * frame.movement))
                                ),
                            }
                            snapshot = [
                                raw_first,
                                tick,
                                center + tick - 16 * d,
                                done,
                                tick + hop,
                            ]
                            if defect == "wrong epoch":
                                job["owner"] = (-1, -1)
                            if defect == "late":
                                job["pub"] += 1
                            note.append("snapshot")
            if action or actions or due or note:
                log.append(
                    {
                        "tick": tick,
                        "generation": generation,
                        "event": action,
                        "phase": phase,
                        "owner": owner,
                        "live": live,
                        "consumed": consumed,
                        "actions": note,
                        "window": snapshot,
                        "held": held.tolist(),
                        "target": (10 ** (held / 20) - 1).tolist(),
                        "job": None
                        if job is None
                        else [*job["owner"], job["pub"] - hop],
                        "held_without_support": bool(
                            live
                            and (
                                phase != "active"
                                or not history
                                or not history[max(history)].good()
                            )
                        ),
                        "published": published.tolist(),
                    }
                )
        n = end
    return log


def controls():
    fs = 48000
    hop = 240
    n = fs
    t = np.arange(n) / fs
    x = sum(0.08 * np.cos(2 * np.pi * k * 196 * t) / (k**0.65) for k in range(1, 11))
    ticks = list(range(hop - 1, n, hop))
    frames = {u: Frame(196) for u in ticks}
    events = {
        0: ("pick",),
        ticks[10]: (("launch", (0, 1)),),
        ticks[38]: ("pick", "mute"),
        ticks[51]: ("dispose", ("launch", (0, 2))),
        ticks[59]: ("mute",),
        ticks[60]: ("dispose",),
        ticks[65]: ("change",),
        ticks[80]: (("launch", (0, 3)),),
        ticks[91]: ("release",),
        ticks[95]: ("reset", "pick", ("launch", (0, 3))),
        ticks[97] + 1: ("pick",),
    }
    for i in (18, 19):
        frames[ticks[i]] = Frame(196, amplitude=0.0001)
    del frames[ticks[23]]
    frames[ticks[28]] = Frame(196, quality=0.73)
    for i in range(66, 91):
        frames[ticks[i]] = Frame(
            440 * 2 ** ((30 * math.sin(i) if i > 82 else 0) / 1200)
        )
    frames[ticks[34]] = Frame(196, supported=False)
    from dataclasses import replace

    for i, tick in enumerate(ticks):
        if tick in frames:
            tag = (
                (0, 1) if i < 38 else (0, 2) if i < 65 else (0, 3) if i < 95 else (1, 4)
            )
            frames[tick] = replace(frames[tick], owner=tag)
    frames[ticks[31]] = Frame(196, owner=(-1, 1))
    logs = replay(x, fs, frames, events)
    for parts in (
        (1,),
        (17,),
        (64,),
        (127,),
        (256,),
        (1024,),
        (4097,),
        tuple(np.random.default_rng(8).integers(1, 500, 30)) + (0,),
    ):
        require(replay(x, fs, frames, events, parts) == logs, "clock partition")

    def valid(rows):
        by = {r["tick"]: r for r in rows}
        require(
            by[ticks[38]]["event"] == "pick"
            and "publish" not in by[ticks[38]]["actions"],
            "repick priority",
        )
        require(
            by[ticks[95]]["event"] == "reset" and by[ticks[95]]["live"] is None,
            "reset priority",
        )
        for row in rows:
            w = row["window"]
            if w:
                require(
                    w[1] == row["tick"]
                    and w[0] >= 0
                    and w[2] <= 2 * w[1]
                    and w[1] <= w[3] < w[4],
                    "causal capture",
                )
            if "publish" in row["actions"]:
                prev = by[row["tick"] - hop]
                require(
                    prev["window"]
                    and prev["window"][4] == row["tick"]
                    and prev["owner"] == row["owner"],
                    "stale publication",
                )
        for i in (18, 19, 23, 28, 31, 34, 59, 91):
            require("publish" not in by[ticks[i]]["actions"], "invalid refresh")
        first = next(r for r in rows if r["window"])
        require(
            first["tick"] == 2399 and first["window"][:3] == [1325, 2399, 3724],
            "first eligible capture",
        )
        require(
            np.any(by[ticks[38]]["held"]) and not np.any(by[ticks[38]]["published"]),
            "repick lost old sound or inherited old color",
        )
        require(not np.any(by[ticks[95]]["held"]), "reset retained voice")
        require(
            by[ticks[59]]["phase"] == "release" and by[ticks[60]]["live"] is None,
            "mute disposal",
        )
        require(
            "publish" in by[ticks[10]]["actions"]
            and by[ticks[10]]["consumed"] == by[ticks[10]]["live"],
            "coincident launch",
        )

    valid(logs)
    # Publication with a pending confirmed gesture is also legal before launch.
    before = replay(x, fs, frames, {0: ("pick",), ticks[13] + 1: (("launch", (0, 1)),)})
    launch = next(r for r in before if r["tick"] == ticks[13] + 1)
    require(
        any(launch["held"]) and launch["live"] == (0, 1), "publication before launch"
    )
    prefix = ticks[15]
    changed = x.copy()
    changed[prefix + 1 :] = -0.9
    require(
        [r for r in replay(changed, fs, frames, events) if r["tick"] <= prefix]
        == [r for r in logs if r["tick"] <= prefix],
        "future input changes past output",
    )
    # Invalidate an already confirmed job if its result is attached to a stale voice.
    stale = replay(x, fs, frames, {0: ("pick",), ticks[13]: (("launch", (0, 9)),)})
    require(all(r["live"] is None for r in stale), "wrong launch token")
    rejected = {"wrong launch token": "stale confirmation cannot promote a gesture"}
    import copy

    for name, i, edit in (
        ("wrong event order", 38, lambda r: r.update(event="mute")),
        ("future input", 10, lambda r: r["window"].__setitem__(1, r["tick"] + 1)),
        ("invalid refresh", 18, lambda r: r["actions"].append("publish")),
        ("stale owner", 14, lambda r: r.update(owner=(0, -2))),
    ):
        rows = copy.deepcopy(logs)
        edit(next(r for r in rows if r["tick"] == ticks[i]))
        try:
            valid(rows)
        except ValueError as e:
            rejected[name] = str(e)
        else:
            raise ValueError("escaped " + name)
    for defect in ("late", "wrong epoch"):
        altered = replay(x, fs, frames, events, defect=defect)
        require(all("publish" not in r["actions"] for r in altered), defect)
        rejected[defect] = "no obsolete publication"
    require(any(np.any(r["held"]) for r in logs), "no live identity")
    return {"records": logs, "negatives": rejected}


if __name__ == "__main__":
    OUT.mkdir(exist_ok=True, parents=True)
    (OUT / "clock.json").write_text(
        json.dumps(controls(), allow_nan=False, indent=2) + "\n"
    )
