"""Joined causal oracle. Only typed A3 decisions/energy and fault probes are injected."""

import json
from functools import lru_cache

import numpy as np
from i1_clock import (
    Confirm,
    Dispose,
    Frame,
    Gesture,
    Launch,
    clock_ticks,
    due_stamps,
    replay,
    rounded,
)
from i1_motion import OUT, physical
from i1_safety import GRID, ORDERS, permission_block
from o2_bench import replay as electrical
from o2_records import sample_hash
from o2_response import require


class WetEpoch:
    def __init__(self, fs):
        self.fs = fs
        self.z = np.zeros((4, 12), complex)
        self.zz = self.z.copy()
        self.c = np.zeros(12)
        self.a = self.dest = self.next = self.phase = 0.0
        self.ready = self.count = 0
        self.length = 2 * rounded(4 * fs / 196)
        self.buffer = np.empty((self.length, 14))
        self.ramp = rounded(0.005 * fs)
        self.observed = True
        self.veto = True

    def tick(self, q, m5, pitch, target, trusted, observation, fault):
        # Fixed-size prospective state, published at the end of a valid tick.
        if self.count == 17:
            self.dest = self.next
        a = self.a + np.clip(self.dest - self.a, -1 / self.ramp, 1 / self.ramp)
        c = self.c + (-np.expm1(-1 / (0.015 * self.fs))) * (target - self.c)
        if fault == "candidate":
            c[0] = np.nan
        require(np.isfinite(c).all(), "candidate")
        r = np.exp(-2 * np.pi * 0.22 * pitch / self.fs)
        poles = r * np.exp(2j * np.pi * pitch * ORDERS / self.fs)
        z = poles * self.z + (1 - r) * q[:, None]
        zz = poles * self.zz + (1 - r) * z
        if fault == "bank":
            zz[3, 0] = np.inf
        require(np.isfinite(zz).all() and np.isfinite(z).all(), "bank")
        taper = np.clip(
            (0.28 * (self.fs / (self.fs // 16000)) - pitch * ORDERS)
            / (0.03 * (self.fs / (self.fs // 16000))),
            0,
            1,
        )
        d = 2 * zz.real @ (taper * c)
        wet = q.copy() if a == 0 else q + a * d
        if fault == "wet":
            wet[3] = np.nan
        require(np.isfinite(wet).all(), "wet")
        row = np.r_[q, d, wet, m5, pitch]
        if fault == "moment":
            row[11] = np.nan
        require(np.isfinite(row).all() and np.isfinite(row @ row), "moment")
        terminal = float(wet[3]) if fault != "conversion" else np.finfo(float).max
        require(abs(terminal) <= np.finfo(np.float32).max, "conversion")
        self.buffer[self.count] = row
        self.count += 1
        self.observed &= observation
        if self.count == self.length:
            b = self.buffer
            self.phase, mask, live, available, _, _ = permission_block(
                b[:, :4],
                b[:, 4:8],
                b[:, 8:12],
                b[:, 12],
                self.fs,
                b[:, 13],
                True,
                self.phase,
                trusted,
                self.observed,
            )
            self.ready = (
                min(2, self.ready + 1) if live and available and mask[-1] else 0
            )
            self.veto = self.ready < 2
            self.next = 0.0 if self.veto else float(GRID[np.flatnonzero(mask)[0]])
            self.count = 0
            self.observed = True
        self.z, self.zz, self.c, self.a = z, zz, c, a
        return terminal


def fixture(fs, n):
    t = np.arange(n) / fs
    return sum(0.12 * np.cos(2 * np.pi * 196 * k * t) / k**0.7 for k in range(1, 11))


@lru_cache(None)
def returns(fs, count, kind):
    ctl = np.tile([196.0, 0.1, 1.8 * 0.1 / 1.65], (count, 1))
    if kind == "lifted":
        ctl[8000:, 2] = 0
    depth = 0.5 + 0.45 * np.sin(2 * np.pi * 7 * np.arange(count) / fs)
    directory = OUT / "joined"
    directory.mkdir(parents=True, exist_ok=True)
    raw = physical(ctl, fs, f"{kind}-{fs}-{count}", directory)
    q = np.column_stack((raw[:, 25:28], electrical(fs, raw[:, 21], depth)[:, 3]))
    return raw, q, depth


def lane_ticks(
    fs, x, frames, events, envelope, carriers, faults, partitions, stamps, trusted
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
    clock = clock_ticks(x, fs, frames, events, partitions, identity_stamps=stamps)
    post = False
    for tick in range(n):
        at, row = clock.send(post if tick else None)
        require(at == tick, "host clock")
        post = False
        if row:
            capture.append(row)
            if row["event"] == "reset":
                epoch = state = None
                fade = -1
                cache = (0.0, 0.0)
            if row["epoch"] != epoch:
                require(
                    epoch is None
                    or row["event"] in ("reset", "fault")
                    or (
                        envelope[tick] == 0
                        and any(
                            isinstance(e, Dispose) and e.epoch == epoch
                            for e in events.get(tick, ())
                        )
                    ),
                    "nonzero disposal",
                )
                epoch = row["epoch"]
                state = None
                cache = (0.0, 0.0)
                if epoch is not None:
                    require(fade < 0 and envelope[tick] == 0, "launch energy/fade")
                    state = WetEpoch(fs)
                    pointer = 0
            dest = np.asarray(row["target"])
        site = faults.get(tick)
        gc = oc = 0
        wet = qval = depth = a = 0.0
        index = -1
        e = product = 0.0
        old_epoch = epoch
        veto = True
        if state is not None:
            raw, q, depths = carriers[epoch]
            index = pointer
            try:
                require(site != "pre-guide", "pre-guide")
                gc = 1
                commit = raw[index]
                pointer += 1
                require(site != "post-o2", "post-o2")
                oc = 1
                depth, qval = depths[index], q[index, 3]
                live_q = q[index].copy()
                if site == "delivery-state":
                    live_q[3] = np.nan
                require(np.isfinite(live_q).all(), "delivery-state")
                wet = state.tick(
                    live_q,
                    commit[11],
                    196.0,
                    dest,
                    trusted,
                    not (6200 <= tick < 7100),
                    site,
                )
                a, veto = state.a, state.veto
                e = envelope[tick]
                product = e * wet
                require(np.isfinite(product), "cache")
                cache = (e, product)
                vectors[tick] = np.r_[dest, state.c]
            except ValueError as err:
                require(str(err) == site, "unexpected transaction failure " + str(err))
                transactions.append(
                    {
                        "tick": tick,
                        "site": site,
                        "epoch": epoch,
                        "physical_index": index,
                        "physical": raw[index, :22].tolist() if gc else None,
                        "o2": q[index].tolist() if oc else None,
                        "preceding_cache": cache,
                    }
                )
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
        ]
        yield tick
    return trace, vectors, capture, transactions, columns


def advance_lanes(*lanes):
    # One physical-cache transaction per lane before the next host sample.
    tick = 0
    while True:
        finished = []
        for lane in lanes:
            try:
                require(next(lane) == tick, "stereo clock")
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
    trusted = json.loads((OUT / "motion.json").read_text())["rows"]["steady-48000"][
        "prerequisite"
    ]["passed"]

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
            trusted,
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
        "faults": {},
        "fs": fs,
        "reference": sample_hash(base),
        "gain_hash": sample_hash(gains),
    }
    for parts in (
        (1,),
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
        ("conversion", replace + 600),
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
