"""Joined causal oracle. Only typed A3 decisions/energy and fault probes are injected."""

import json
from dataclasses import dataclass
from functools import lru_cache

import numpy as np
from i1_clock import (
    Confirm,
    Dispose,
    Frame,
    Gesture,
    Launch,
    clock_ticks,
    decode,
    due_stamps,
    replay,
    rounded,
)
from i1_motion import OUT, physical, prerequisite
from i1_stream import Epoch, NumericalFault, Site
from o2_bench import replay as electrical
from o2_records import sample_hash
from o2_response import require


@dataclass(frozen=True)
class Eligibility:
    generation: int
    epoch: int
    fs: int
    issued: int
    through: int
    pitch_range: tuple = (65.0, 1320.0)
    depth_range: tuple = (0.0, 1.0)
    speed_range: tuple = (0.06, 0.16)
    force_range: tuple = (0.0, 0.16 * 2.4 / 1.65)

    def permits(self, generation, epoch, rate, tick, controls, depth):
        pitch, speed, force = controls
        return (
            (self.generation, self.epoch, self.fs)
            == (
                generation,
                epoch,
                rate,
            )
            and self.issued <= tick <= self.through
            and self.pitch_range[0] <= pitch <= self.pitch_range[1]
            and self.depth_range[0] <= depth <= self.depth_range[1]
            and self.speed_range[0] <= speed <= self.speed_range[1]
            and self.force_range[0] <= force <= self.force_range[1]
        )


# Offline harness poisons native operands. Fault ownership comes from the checked
# operation; an unexpected numerical fault follows exactly the same M6 branch.
PROBES = {
    "phase": 1,
    "candidate": 2,
    "bank": 3,
    "trial": 4,
    "moment": 5,
    "normalization": 6,
    "gram": 7,
    "residual": 8,
    "wet": 9,
    "conversion": 10,
    "increment": 11,
    "bank-overflow": 13,
    "trial-nan": 14,
}


def fixture(fs, n):
    t = np.arange(n) / fs
    return sum(0.12 * np.cos(2 * np.pi * 196 * k * t) / k**0.7 for k in range(1, 11))


@lru_cache(None)
def returns(fs, count, kind):
    ctl = np.tile([196.0, 0.1, 1.8 * 0.1 / 1.65], (count, 1))
    if kind in ("lifted", "uneven", "replacement"):
        ctl[:, 0] *= np.exp2(35 * np.sin(2 * np.pi * 5 * np.arange(count) / fs) / 1200)
    if kind == "lifted":
        ctl[8000:, 2] = 0
    depth = 0.5 + 0.45 * np.sin(2 * np.pi * 7 * np.arange(count) / fs)
    directory = OUT / "joined"
    directory.mkdir(parents=True, exist_ok=True)
    raw = physical(ctl, fs, f"{kind}-{fs}-{count}", directory)
    q = np.column_stack((raw[:, 25:28], electrical(fs, raw[:, 21], depth)[:, 3]))
    return raw, q, depth, ctl


@dataclass
class LaneSource:
    events: dict
    generator: object


def lane_ticks(
    fs,
    x,
    frames,
    events,
    envelope,
    carriers,
    faults,
    partitions,
    stamps,
    trusted,
    *,
    held_control=None,
    outage=(6200, 7100),
    receipt_updates=None,
    signed_gain=1.0,
):
    schedule = dict(events)
    work = _lane_samples(
        fs,
        x,
        frames,
        schedule,
        envelope,
        carriers,
        faults,
        partitions,
        stamps,
        dict(trusted),
        held_control,
        outage,
        receipt_updates or {},
        signed_gain,
    )
    return LaneSource(schedule, work)


def _lane_samples(
    fs,
    x,
    frames,
    events,
    envelope,
    carriers,
    faults,
    partitions,
    stamps,
    trusted,
    held_control,
    outage,
    receipt_updates,
    signed_gain,
):
    # Arrays retain complete sample coverage. Sparse capture records hold until next event.
    n = len(x)
    columns = (
        "epoch",
        "physical_index",
        "guide_commit",
        "o2_commit",
        "invalidation_count",
        "depth",
        "q",
        "lambda",
        "wet",
        "E",
        "product",
        "cached_E",
        "cached_product",
        "fade_index",
        "veto",
        "trial",
        "window_closed",
        "phase",
        "target",
        "off_wet",
        "factory_wet",
        "full_wet",
    )
    trace = np.zeros((n, len(columns)))
    vectors = np.zeros((n, 24))
    capture, transactions = [], []
    epoch = None
    state = None
    pointer = invalidations = 0
    fade = -1
    cache = (0.0, 0.0)
    dest = np.zeros(12)
    nf = rounded(0.005 * fs)
    clock = clock_ticks(
        x, fs, frames, events, partitions, identity_stamps=stamps, energy=envelope
    )
    post = False
    for tick in range(n):
        at, row = clock.send(post if tick else None)
        require(at == tick, "host clock")
        post = False
        if tick in receipt_updates:
            key, receipt = receipt_updates[tick]
            trusted[key] = receipt
        site = faults.get(tick)
        if row:
            generation = row["generation"]
            capture.append(row)
            if row["event"] == "fault" and state is not None:
                site = "pre-guide"
            if row["event"] == "reset":
                epoch = state = None
                fade = -1
                cache = (0.0, 0.0)
            if row["epoch"] != epoch and row["event"] != "fault":
                require(
                    epoch is None
                    or row["event"] in ("reset", "fault")
                    or (
                        envelope[tick] == 0
                        and any(
                            isinstance(e, Dispose) and e.epoch == epoch
                            for e in decode(events.get(tick, ()), generation)[0]
                        )
                    ),
                    "nonzero disposal",
                )
                epoch = row["epoch"]
                state = None
                cache = (0.0, 0.0)
                if epoch is not None:
                    require(fade < 0 and envelope[tick] == 0, "launch energy/fade")
                    state = Epoch(fs)
                    pointer = 0
            dest = np.asarray(row["target"])
            if held_control is not None and row["epoch"] is not None:
                dest = held_control
        gc = oc = 0
        wet = qval = depth = a = 0.0
        index = -1
        e = product = 0.0
        old_epoch = epoch
        veto = True
        if state is not None:
            raw, q, depths, controls = carriers[epoch]
            index = pointer
            try:
                if site == "pre-guide":
                    raise NumericalFault(Site.PHYSICAL)
                gc = 1
                commit = raw[index]
                pointer += 1
                if site == "post-o2":
                    raise NumericalFault(Site.ELECTRICAL)
                oc = 1
                depth, qval = depths[index], q[index, 3]
                live_q = q[index].copy()
                if site == "delivery-state":
                    live_q[3] = np.nan
                if not np.isfinite(live_q).all():
                    raise NumericalFault(Site.ELECTRICAL)
                permission = trusted.get(epoch)
                eligible = permission is not None and permission.permits(
                    generation, epoch, fs, tick, controls[index], depth
                )
                wet = state.tick(
                    live_q,
                    commit[11],
                    controls[index, 0],
                    dest,
                    eligible,
                    not (outage[0] <= tick < outage[1]),
                    PROBES.get(site, 0),
                )
                a, veto = state.a, state.veto
                e = envelope[tick]
                product = float(e) * (float(signed_gain) * float(wet))
                if not np.isfinite(product):
                    raise NumericalFault(Site.CACHE)
                mix = (1.0 - e) * x[tick] + product
                if not (
                    0 <= e <= 1
                    and np.isfinite(mix)
                    and abs(mix) <= np.finfo(np.float32).max
                ):
                    raise NumericalFault(Site.CONVERSION)
                cache = (e, product)
                vectors[tick, :12], vectors[tick, 12:] = dest, state.c
            except NumericalFault as err:
                transactions.append(
                    {
                        "tick": tick,
                        "site": err.site.name,
                        "epoch": epoch,
                        "physical_index": index,
                        "physical": raw[index, :22].tolist() if gc else None,
                        "o2": q[index].tolist() if oc else None,
                        "preceding_cache": cache,
                    }
                )
                wet = a = 0.0
                veto = True
                post = True
                epoch = state = None
                invalidations += 1
                fade = 0
        fade_index = fade
        if fade >= 0:
            scale = (nf - 1 - fade) / nf
            e, product = cache[0] * scale, cache[1] * scale
            fade += 1
            if fade == nf:
                cache = (0.0, 0.0)
                fade = -1
        trace[tick] = [
            old_epoch or 0,
            index,
            gc,
            oc,
            invalidations,
            depth,
            qval,
            a,
            wet,
            e,
            product,
            *cache,
            fade_index,
            veto,
            *(
                state.sample[0, [9, 10, 17, 18, 4, 5, 6]]
                if state
                else [-1, 0, 0, 0, 0, 0, 0]
            ),
        ]
        yield tick
    return trace, vectors, capture, transactions, columns


def advance_lanes(*lanes):
    # One physical-cache transaction per lane before the next host sample.
    tick = generation = 0
    while True:
        if any(
            Gesture.RESET in decode(lane.events[tick], generation)[0]
            for lane in lanes
            if tick in lane.events
        ):
            generation += 1
            for lane in lanes:
                lane.events[tick] = (Gesture.RESET,)
        finished = []
        for lane in lanes:
            try:
                require(next(lane.generator) == tick, "stereo clock")
            except StopIteration as end:
                finished.append(end.value)
        if finished:
            require(len(finished) == len(lanes), "incomplete stereo clock")
            return finished
        tick += 1


def capture_controls():
    fs = 48000
    n = 24000
    token = (0, 1)
    x = fixture(fs, n)
    frames = {t: Frame(196) for t in range(239, n, 240)}
    stamps = due_stamps(n, fs)

    def play(events):
        return replay(x, fs, frames, events, identity_stamps=stamps)

    expired = play(
        {0: (Gesture.PICK,), 5760: (Gesture.TIMEOUT,), 23900: (Launch(token, 1, 0.0),)}
    )
    auto = play(
        {
            0: (Gesture.PICK,),
            23900: (
                Confirm(token, n),
                Launch(token, 1, 0.0),
            ),
        }
    )
    late = play(
        {
            0: (Gesture.PICK,),
            480: (Confirm(token, 3000),),
            3010: (Launch(token, 1, 0.0),),
        }
    )
    for log in (expired, auto, late):
        require(
            not any(
                r["epoch"] or ("publish" in r["actions"] and r["tick"] > 5760)
                for r in log
            ),
            "expired identity",
        )
    replacement = {
        0: (Gesture.PICK,),
        479: (Confirm(token, n), Launch(token, 1, 0.0)),
        7440: (Gesture.PICK,),
        8399: (Confirm((0, 2), 9000),),
        8400: (Launch((0, 2), 2, 0.0),),
        8401: (Dispose(1, 0.1), Launch((0, 2), 2, 0.0)),
        8402: (Dispose(2, 0.0), Launch((0, 2), 2, 0.0)),
        8403: (Dispose(1, 0.0), Launch((0, 2), 2, 0.0)),
    }
    stale = {r["tick"]: r for r in play(replacement)}
    require(stale[8403]["epoch"] is None, "stale support confirmed replacement")
    frames.update({t: Frame(196, owner=(0, 2)) for t in frames if t >= 7440})
    r = {r["tick"]: r for r in play(replacement)}
    require(
        all(r[t]["epoch"] == 1 and any(r[t]["held"]) for t in (8400, 8401, 8402))
        and r[8403]["epoch"] == 2
        and not any(r[8403]["held"]),
        "replacement ownership",
    )
    fs = 47001
    frames = {t: Frame(196, owner=(1, 1)) for t in range(19 + 234, n, 235)}
    ev = {19: (Gesture.RESET,), 20: (Gesture.PICK,), 488: (Confirm((1, 1), n),)}
    stamps = due_stamps(n, fs, [19])
    logs = []
    for a2, i1 in ((True, True), (False, True), (True, False)):
        observations = dict(frames)
        if not a2:
            del observations[2603]
        log = replay(
            x,
            fs,
            observations,
            ev,
            identity_stamps=stamps if i1 else stamps - {(1, 2614)},
        )
        first = next(r for r in log if r["window"])
        require(first["window"] == [1356, 2378, 3734, 2446, 2614], "uneven FIR stamps")
        require(
            any("publish" in r["actions"] and r["tick"] == 2614 for r in log)
            == (a2 and i1),
            "missing due stamp",
        )
        logs.append(first["window"])
    return {
        "timeout_attempt": expired[-1],
        "automatic_expiry": auto[-1],
        "confirmed_expiry": late[-1],
        "replacement": [r[t] for t in range(8400, 8404)],
        "uneven": logs,
    }


def uneven_delivery():
    fs, n = 47001, 14500
    x = fixture(fs, n)
    token = (1, 1)
    events = {
        19: (Gesture.RESET,),
        20: (Gesture.PICK,),
        488: (Confirm(token, 5000),),
        2614: (Launch(token, 1, 0.0),),
    }
    stamps = due_stamps(n, fs, [19])
    energy = np.clip((np.arange(n) - 2614) / rounded(0.008 * fs), 0, 1)
    carrier = returns(fs, 3 * fs, "uneven")
    neutral = returns(fs, 3 * fs, "reference")
    trusted = prerequisite(neutral[0], fs, 196)["passed"]
    require(trusted, "uneven reference prerequisite")
    summaries = {}
    arrays = {}
    for key in ("complete", "missing-A2", "missing-I1"):
        frames = {t: Frame(196, owner=token) for t in range(253, 2604, 235)}
        if key == "missing-A2":
            del frames[2603]
        received = stamps - {(1, 2614)} if key == "missing-I1" else stamps
        gen = lane_ticks(
            fs,
            x,
            frames,
            events,
            energy,
            {1: carrier},
            {},
            (0, 1, 255, 19),
            received,
            {1: Eligibility(1, 1, fs, 2614, n)} if trusted else {},
        )
        tr, g, cap, _, _ = advance_lanes(gen)[0]
        launch = next(r for r in cap if r["tick"] == 2614)
        # Support deliberately ends immediately after the due result, so later
        # frames cannot replace the disputed publication.
        changed = bool(np.any(tr[2614:, 8] != tr[2614:, 6]))
        require(
            changed == (key == "complete") and bool(any(launch["held"])) == changed,
            "uneven joined publication/held delivery",
        )
        arrays[key] = tr
        arrays[key + "_gains"] = g
        summaries[key] = {
            "launch": launch,
            "capture": cap,
            "changed": changed,
            "trace": sample_hash(tr),
        }
    np.savez(OUT / "joined/uneven.npz", **arrays)
    return summaries


def main():
    fs = 48000
    n = 18500
    x = fixture(fs, n)
    # The joined fixture supplies the A3 envelope, independently of I1 admission.
    times = np.arange(n)
    first, release, replace = 479, 9479, 9719
    events = {
        0: (Gesture.PICK,),
        first: (Confirm((0, 1), 5500), Launch((0, 1), 1, 0.0)),
        7679: (Gesture.HOLD,),
        release: (Gesture.PICK,),
        replace: (Dispose(1, 0.0), Confirm((0, 2), 11000), Launch((0, 2), 2, 0.0)),
        13000: (Gesture.MUTE,),
        14680: (Dispose(2, 0.0),),
        15000: (Gesture.PICK,),
        15599: (Confirm((0, 3), 17000), Launch((0, 3), 3, 0.0)),
    }
    frames = {
        t: Frame(
            196,
            owner=(0, 1 if t < release else 2 if t < 15000 else 3),
            supported=t < 7679 or t >= release,
        )
        for t in range(239, n, 240)
    }
    envelope = np.interp(
        times,
        [
            0,
            first,
            first + 384,
            release,
            replace,
            replace + 384,
            13000,
            14680,
            15599,
            15983,
            n,
        ],
        [0, 0, 1, 1, 0, 1, 1, 0, 0, 1, 1],
    )
    carriers = {
        1: returns(fs, n, "lifted"),
        2: returns(fs, n, "replacement"),
        3: returns(fs, n, "fresh"),
    }
    neutral = np.memmap(
        OUT / "motion/steady-48000.f64le", dtype="<f8", mode="r"
    ).reshape(-1, 30)
    trusted = prerequisite(neutral, fs, 196)["passed"]

    def stream(faults, parts=(127,), di=x):
        return lane_ticks(
            fs,
            di,
            frames,
            events,
            envelope,
            carriers,
            faults,
            parts,
            due_stamps(len(di), fs),
            {e: Eligibility(0, e, fs, 0, n) for e in carriers} if trusted else {},
        )

    def simulate(*args, **kwargs):
        return advance_lanes(stream(*args, **kwargs))[0]

    base, gains, capture, _, columns = simulate({})
    directory = OUT / "joined"
    np.savez(directory / "reference.npz", samples=base, coefficients=gains, di=x)
    result = {
        "columns": columns,
        "capture": capture,
        "clocks": capture_controls(),
        "uneven_delivery": uneven_delivery(),
        "faults": {},
        "fs": fs,
        "reference": sample_hash(base),
        "gain_hash": sample_hash(gains),
    }
    for parts in (
        (1,),
        (n + 17,),
        (first, replace - first, n - replace),
        (0, 17, 256, 1, 1024, 64),
        tuple(np.random.default_rng(81).integers(0, 700, 31)),
    ):
        tr, g, c, _, _ = simulate({}, parts)
        require(
            np.array_equal(tr, base) and np.array_equal(g, gains) and c == capture,
            "joined partition",
        )
    prefix = x.copy()
    prefix[7001:] = 0.0
    tr, g, c, _, _ = simulate({}, di=prefix)
    require(
        np.array_equal(tr[:7001], base[:7001])
        and np.array_equal(g[:7001], gains[:7001])
        and [r for r in c if r["tick"] <= 7000]
        == [r for r in capture if r["tick"] <= 7000],
        "joined causal prefix",
    )
    for site, when in (
        ("candidate", first),
        ("moment", 6100),
        ("bank", 7800),
        ("wet", 9000),
        ("conversion", replace + 192),
        ("delivery-state", 6000),
        ("pre-guide", 5600),
        ("post-o2", 5600),
    ):
        # Second lane runs from its own clock and continues through this lane's fault.
        left, right = advance_lanes(stream({when: site, when + 4: site}), stream({}))
        tr, g, c, tx, _ = left
        other, other_g, other_c, _, _ = right
        require(
            np.array_equal(other, base)
            and np.array_equal(other_g, gains)
            and other_c == capture,
            "unaffected stereo lane",
        )
        require(
            len(tx) == 1
            and tr[-1, 4] == 1
            and tr[when, 2] == int(site != "pre-guide")
            and tr[when, 3] == int(site not in ("pre-guide", "post-o2")),
            "fault ownership",
        )
        if site == "conversion":
            require(
                0 < tr[when - 1, 11] < 1 and tr[when - 1, 12] != 0,
                "fractional cached product",
            )
            check, coeff, cap, probe, _ = simulate(
                {when: site, when + 4: site}, (0, 1, 37, 255)
            )
            require(
                np.array_equal(check, tr)
                and np.array_equal(coeff, g)
                and cap == c
                and probe == tx,
                "faultful partition invariance",
            )
        nf = rounded(0.005 * fs)
        expected = (nf - 1 - np.arange(nf)) / nf
        require(
            np.array_equal(
                tr[when : when + nf, 9:11], expected[:, None] * tr[when - 1, 11:13]
            )
            and np.all(tr[when + 1 : when + nf, 2:4] == 0)
            and tr[-1, 0] == 3
            and tr[-1, 2] == 1,
            "cached fade or fresh acquisition",
        )
        np.savez(
            directory / (site + ".npz"),
            left=tr,
            right=other,
            left_coefficients=g,
            right_coefficients=other_g,
        )
        result["faults"][site] = {
            "transaction": tx[0],
            "capture": c,
            "trace": sample_hash(tr),
            "other": sample_hash(other),
            "zero_tick": when + nf - 1,
        }
    (OUT / "joined.json").write_text(
        json.dumps(result, indent=2, allow_nan=False) + "\n"
    )


if __name__ == "__main__":
    main()
