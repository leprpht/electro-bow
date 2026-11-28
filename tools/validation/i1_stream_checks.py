"""Independent direct projections and joined chronological acceptance predicates."""

import json
import math
from itertools import groupby

import numpy as np
from acoustic_observation import rms, tracking
from i1_clock import Confirm, Dispose, Frame, Gesture, Launch, due_stamps
from i1_joined import Eligibility, advance_lanes, fixture, lane_ticks, returns
from i1_motion import ALT, OUT, bank, inputs, witness
from i1_safety import GRID, drive
from i1_stream import Epoch, Site
from o2_bench import replay
from o2_coverage import RATE
from o2_response import require


def close(actual, expected, tolerance, predicate):
    error = float(np.max(np.abs(np.asarray(actual) - expected), initial=0))
    require(error <= tolerance, predicate)
    return error


def phase_basis(f, fs, initial, parity=0):
    angles = initial + np.cumsum(f * (2 * np.pi / fs))
    return np.column_stack(
        (
            np.ones(len(f)),
            np.cos(angles),
            np.sin(angles),
            np.cos(2 * angles),
            np.sin(2 * angles),
            (-1.0) ** (np.arange(len(f)) + parity),
        )
    ), angles


def direct_pass(x, neutral, m2, basis, orthogonal, triangular, recovery):
    a = np.linalg.solve(triangular, orthogonal.T @ x)
    residual = np.mean((x - basis @ a) ** 2)
    p = np.mean(x * x)
    h1, h2 = [
        np.linalg.norm(basis[:, i : i + 2] @ a[i : i + 2]) / math.sqrt(len(x))
        for i in (1, 3)
    ]
    margin = 10 ** ((0.30 if recovery else 0.05) / 20)
    factor = 10 ** ((2.85 if recovery else 3) / 10)
    return (
        min(neutral, m2) > 1e-24
        and p >= max(neutral, m2) / factor
        and p <= min(neutral, m2) * factor
        and h1
        >= margin * max(h2, math.sqrt(2) * abs(a[5]), math.sqrt(residual)) + 1e-12
    )


def packed(b, q, d, y, m, target):
    x = np.column_stack(
        [v for j in range(4) for v in (q[:, j], d[:, j], y[:, j])] + [m]
    )
    pair_q, pair_d = q[:, 1] - q[:, 0], d[:, 1] - d[:, 0]
    return np.r_[
        (b.T @ b)[np.tril_indices(6)],
        (x.T @ b).ravel(),
        np.array(
            [
                v
                for j in range(4)
                for v in (q[:, j] @ q[:, j], q[:, j] @ d[:, j], d[:, j] @ d[:, j])
            ]
        ),
        (y * y).sum(axis=0),
        m @ m,
        pair_q @ pair_q,
        pair_q @ pair_d,
        pair_d @ pair_d,
        sum((y[:, 1] - y[:, 0]) ** 2),
        sum((q[:, 3] + target * d[:, 3]) ** 2),
        q[:, 3] @ y[:, 3],
        d[:, 3] @ y[:, 3],
    ]


def differential(fs, kind, frequency=None, ratio=1.0):
    """Compare each packed field at the last open tick, then all 13 solved RHS."""
    f = 1320.0 if kind == "degenerate" else (196.0 if frequency is None else frequency)
    n = 2 * math.floor(4 * fs / f + 0.5)
    hz = np.full(n + 17, f)
    if kind in ("vibrato", "bend", "max-retune"):
        hz *= np.exp2(np.linspace(0, (35 if kind == "vibrato" else 200) / 1200, n + 17))
    if kind == "degenerate":
        hz[1:n] = 65.0
    basis, angles = phase_basis(hz, fs, 0)
    m = np.cos(angles)
    q = np.repeat((ratio * m)[:, None], 4, axis=1)
    if not kind.startswith("threshold"):
        q[:, 1] += 0.004 * np.cos(angles * 2 + 0.7)
        q[:, 2] += 0.006 * np.sin(angles * 2)
        q[:, 3] += 0.005 * np.sin(angles * 2 + 0.3) * np.cos(np.arange(len(q)) * 0.03)
    coeff = np.tile([-0.38, 0.77], 6)
    frames = np.column_stack(
        (hz, m, q, np.broadcast_to(coeff, (len(m), 12)), np.ones((len(m), 2)))
    )
    epoch = Epoch(fs)
    out = np.zeros((len(m), 20))
    epoch.run(frames[: n - 1], out[: n - 1])
    stats = np.empty(123)
    epoch.lib.epoch_stats(epoch.state, stats.ctypes.data, 0)
    d, y = out[: n - 1, :4], out[: n - 1, 4:8]
    err = close(
        stats / (n - 1),
        packed(basis[: n - 1], q[: n - 1], d, y, m[: n - 1], out[: n - 1, 18])
        / (n - 1),
        2e-11,
        "packed cross moments",
    )
    epoch.run(frames[n - 1 :], out[n - 1 :])
    require(np.array_equal(out[n:, 9], np.arange(17)), "candidate schedule")
    close(out[: n - 1, 17], angles[: n - 1], 2e-10, "committed phase")
    b = basis[:n]
    if kind == "degenerate":
        require(
            out[n - 1, 12] == 0 and not np.any(out[:, 8]), "finite rank ordinary veto"
        )
        return dict(ordinary_veto=True)
    u, r = np.linalg.qr(b, mode="reduced")
    q, d, y, m = q[:n], out[:n, :4], out[:n, 4:8], m[:n]
    epoch.lib.epoch_stats(epoch.state, stats.ctypes.data, 1)
    x = np.column_stack(
        [v for j in range(4) for v in (q[:, j], d[:, j], y[:, j])] + [m]
    )
    fit = close(
        stats[21:99].reshape(13, 6).T,
        np.linalg.solve(r, u.T @ x),
        2e-10,
        "direct QR coefficients",
    )
    energy = close(
        np.mean((x - b @ (np.linalg.solve(r, u.T @ x))) ** 2, axis=0),
        np.mean(x * x, axis=0)
        - np.mean((b @ stats[21:99].reshape(13, 6).T) ** 2, axis=0),
        2e-11,
        "direct QR residual",
    )
    neutral = np.mean(q * q, axis=0)
    mp = m @ m / n

    def test(x, j, recovery):
        return direct_pass(x, neutral[j], mp, b, u, r, recovery)

    masks = []
    for w in GRID:
        z = q + w * d
        masks.append(
            all(test(z[:, j], j, True) for j in range(3))
            and np.mean((z[:, 1] - z[:, 0]) ** 2)
            <= 10 ** (-2.85) * np.mean(z[:, 0] ** 2)
        )
    live = all(test(y[:, j], j, False) for j in range(4)) and np.mean(
        (y[:, 1] - y[:, 0]) ** 2
    ) <= 10 ** (-2.8) * np.mean(y[:, 0] ** 2)
    require(np.array_equal(masks, out[n:, 19]), "direct candidate classification")
    require(bool(out[n - 1, 11]) == live, "direct realized decision")
    return dict(
        moment_error=err,
        fit_error=fit,
        residual_error=energy,
        mask=list(map(bool, masks)),
    )


def motion_samples(root, case, reference, monitor):
    kind, rate = case.rsplit("-", 1)
    fs = int(rate)
    ctl, depth = inputs(fs, kind)
    with np.load(root / "motion" / (case + ".npz")) as z:
        raw = np.memmap(
            root / "motion" / (case + ".f64le"), mode="r", dtype="<f8"
        ).reshape(-1, 30)
        q = z["q"]
        require(
            np.array_equal(depth, z["depth"]) and np.array_equal(ctl[:, 0], z["pitch"]),
            "producer control schedule",
        )
        require(
            np.array_equal(
                q[:, 3].view("u8"), replay(fs, raw[:, 21], depth)[:, 3].view("u8")
            ),
            "automated electrical return",
        )
        expected = bank(q, fs, ctl[:, 0], z["request"])
        close(z["d"], expected, 2e-11, "fourth-bank correction")
        outage = [(round(1.35 * fs), round(1.55 * fs))] if kind == "veto" else []
        wet, a, log = drive(
            q, expected, raw[:, 11], fs, ctl[:, 0], outage, trusted=True
        )
        close(z["scale"], a, 2e-12, "independent scheduled slew")
        close(z["wet"], wet, 2e-11, "four actual wet paths")
        if kind in ("steady", "held"):
            require(
                np.array_equal(z["wet"][:, 3].view("u8"), z["wet"][:, 1].view("u8")),
                "factory final wet parity",
            )
        if kind == "vibrato":
            for j in (0, 3):
                require(
                    witness(z["wet"][:, j] - q[:, j], fs, reference)["passed"],
                    "non-neutral motion",
                )
        if kind == "veto":
            require(
                np.sqrt(
                    np.mean((z["wet"][5 * fs // 2 :, 3] - q[5 * fs // 2 :, 3]) ** 2)
                )
                > 1e-4,
                "fourth recovery",
            )
        require(len(log) == len(monitor), "retired window inventory")
        for measured, oracle in zip(monitor, log):
            require(
                all(
                    measured[k] == oracle[k]
                    for k in (
                        "start",
                        "end",
                        "conditioned",
                        "available",
                        "realized_ok",
                        "bridge_ok",
                    )
                ),
                "window equivalence",
            )
            if measured["selection_tick"] is not None:
                require(
                    measured["scale"] == oracle["scale"]
                    and measured["veto"] == oracle["veto"],
                    "selected grid index",
                )
        return len(log)


def play(
    events,
    n=6200,
    fs=48000,
    faults=None,
    bad_ticks=(),
    carrier=None,
    e=None,
    parts=(127,),
    **options,
):
    token = (0, 1)
    x = fixture(fs, n)
    da = int(fs // 8000)
    hop = da * math.floor(0.005 * fs / da + 0.5)
    frames = {
        t: Frame(196, owner=token, supported=t not in bad_ticks)
        for t in range(hop - 1, n, hop)
    }
    if carrier is None:
        carrier = returns(fs, n, "behavior")
    if e is None:
        e = np.zeros(n)
    permits = {i: Eligibility(0, i, fs, 0, n) for i in (1, 2)}
    return lane_ticks(
        fs,
        x,
        frames,
        events,
        e,
        {1: carrier, 2: carrier},
        faults or {},
        parts,
        due_stamps(n, fs),
        permits,
        **options,
    )


def events_at(t):
    return {0: (Gesture.PICK,), t: (Confirm((0, 1), 12000), Launch((0, 1), 1, 0.0))}


def collisions():
    rows = {}
    for t in (5759, 5760, 5761):
        tr, _, cap, _, _ = advance_lanes(play(events_at(t)))[0]
        require(bool(tr[t, 2]) == (t < 5760), "elapsed acquisition eligibility")
        rows[str(t)] = [
            int(tr[t, 2]),
            next(r["actions"] for r in cap if r["tick"] == t),
        ]
    fs = 32008.333333333336
    for t in (3841, 3842):
        tr = advance_lanes(play(events_at(t), fs=fs, n=4000))[0][0]
        require(
            bool(tr[t, 2]) == (t == 3841), "fractional elapsed acquisition eligibility"
        )
        rows[f"fractional/{t}"] = int(tr[t, 2])
    for label, extra, t, bad in (
        ("timeout", Gesture.TIMEOUT, 5760, ()),
        ("reset", Gesture.RESET, 5759, ()),
        ("lost support", None, 5759, (5759,)),
    ):
        ev = events_at(t)
        if extra is not None:
            ev[t] = (extra, *ev[t])
        tr = advance_lanes(play(ev, bad_ticks=bad))[0][0]
        require(not tr[:, 2].any(), "confirmation collision " + label)
    for e in (
        Gesture.HOLD,
        Gesture.PICK,
        Confirm((0, 1), 9000),
        Dispose(1, 0.0),
        Launch((0, 1), 2, 0.0),
    ):
        ev = events_at(479)
        ev[2000] = (e, e)
        tr, _, cap, _, _ = advance_lanes(play(ev, n=2800))[0]
        row = next(r for r in cap if r["tick"] == 2000)
        require(
            "invalid frame" in row["actions"]
            and row["phase"] == "active"
            and tr[2000, 0] == 1,
            "duplicate event rejection",
        )
    return rows


def fault_cases():
    n = 8000
    carrier = returns(48000, n, "uneven")
    # The interval starts after a successful launch with fractional nonzero cached gain.
    ev = events_at(479)
    e = np.full(n, 0.63)
    e[:480] = 0
    clean = advance_lanes(play(ev, n, carrier=carrier, e=e))[0][0]
    close_tick = int(np.flatnonzero(clean[:, 16])[0])
    cases = [
        ("phase", 3300, Site.PHASE),
        ("increment", 3100, Site.PHASE),
        ("bank", 2639, Site.BANK),
        ("bank-overflow", 3300, Site.BANK),
        ("candidate", 479, Site.COEFFICIENT),
        ("trial", close_tick + 1, Site.RESIDUAL),
        ("trial-nan", close_tick + 1, Site.TRIAL),
        ("normalization", close_tick, Site.NORMALIZE),
        ("gram", close_tick, Site.GRAM),
        ("residual", close_tick, Site.RESIDUAL),
        ("wet", 3300, Site.WET),
        ("conversion", 3300, Site.CONVERSION),
        ("moment", 3300, Site.MOMENT),
        ("cache", 3300, Site.CACHE),
        ("M5 accumulation", 0, Site.MOMENT),
        ("M5 closure", 0, Site.MOMENT),
    ]
    results = {}
    for label, tick, site in cases:
        c = carrier
        envelope = e.copy()
        if label.startswith("M5"):
            raw = carrier[0].copy()
            # 180 unscaled squares exceed DBL_MAX. Align the last add at closure too.
            start = 0 if label.endswith("accumulation") else close_tick - 479 + 1 - 180
            raw[start:1960, 11] = 1e153
            c = (raw, *carrier[1:])
            tick = 479 + start + 179
        elif label == "cache":
            # A finite wet safe for f32, multiplied by corrupted finite gain.
            envelope[tick] = np.finfo(float).max
            q = carrier[1].copy()
            q[tick - 479, 3] = 4.0
            c = (carrier[0], q, *carrier[2:])
        faults = {tick: label, tick + 4: label}
        left, right = advance_lanes(
            play(ev, n, carrier=c, e=envelope, faults=faults),
            play(ev, n, carrier=carrier, e=e),
        )
        tr, _, _, tx, _ = left
        require(
            len(tx) == 1 and tx[0]["site"] == site.name and tx[0]["tick"] == tick,
            "lane-local numerical ownership " + label,
        )
        require(np.array_equal(right[0], clean), "unaffected lane progression")
        expected = ((239 - np.arange(240)) / 240)[:, None] * tr[tick - 1, 11:13]
        require(
            tr[tick, 2] == tr[tick, 3] == 1
            and np.array_equal(tr[tick : tick + 240, 9:11], expected),
            "cached wet product fade",
        )
        results[label] = dict(
            site=site.name, tick=tick, zero=tick + 239, commits=int(tr[:, 2].sum())
        )
    return results


def joined_witness(fs, reference):
    # A registered held vector is an explicit test control at the feature port.
    # Confirm and launch after the first eligible A2 stamp. Score epoch-local time.
    ctl, depth = inputs(fs, "vibrato")
    n = len(ctl)
    raw = np.memmap(
        OUT / "motion" / f"vibrato-{fs}.f64le", dtype="<f8", mode="r"
    ).reshape(-1, 30)
    q = np.column_stack((raw[:, 25:28], replay(fs, raw[:, 21], depth)[:, 3]))
    carrier = (raw, q, depth, ctl)
    da = fs // 8000
    launch = da * math.floor(0.005 * fs / da + 0.5) - 1
    size = n + launch
    stamps = due_stamps(size, fs)
    envelope = np.ones(size)
    envelope[: launch + 1] = 0.0
    x = np.zeros(size)
    ev = events_at(launch)
    active = lane_ticks(
        fs,
        x,
        {launch: Frame(196)},
        ev,
        envelope,
        {1: carrier},
        {},
        (0, 1, 4097),
        stamps,
        {1: Eligibility(0, 1, fs, launch, size)},
        held_control=np.expm1(np.log(10) * ALT / 20),
        outage=(0, 0),
    )
    idle = lane_ticks(fs, x, {}, {}, np.zeros(size), {}, {}, (17,), stamps, {})
    left, right = advance_lanes(active, idle)
    tr = left[0][launch:]
    require(
        not right[0][:, 2].any() and np.sum(tr[:, 2]) == n and not left[3],
        "registered physical commits",
    )
    result = {}
    for name, wet, j in (("off", tr[:, 19], 0), ("delivery", tr[:, 8], 3)):
        delta = wet - q[:, j]
        row = witness(delta, fs, reference)
        require(row["passed"], "joined " + name + " motion")
        region = slice(fs // 2, 5 * fs // 2)
        row.update(
            rms=rms(delta[region]),
            peak=float(np.max(abs(wet))),
            neutral_samples=max(
                (len(list(g)) for flag, g in groupby(delta[region] == 0) if flag),
                default=0,
            ),
            pitch=tracking(wet, fs, 196, "vibrato"),
        )
        result[name] = row
    result.update(
        physical_commits=n,
        realized_veto_samples=int(tr[fs // 2 : 5 * fs // 2, 14].sum()),
    )
    return result


def path_behavior():
    from i1_motion import physical
    from i1_stream import render

    fs = 48000
    ctl, depth = inputs(fs, "vibrato")
    ctl = ctl[: fs // 2]
    n = len(ctl)
    raw = physical(ctl, fs, "behavior-motion", OUT / "joined")
    g = np.expm1(np.log(10) * ALT / 20)
    parity = []
    for col, fixed in enumerate((0, 0.5, 1)):
        q = np.column_stack(
            (raw[:, 25:28], replay(fs, raw[:, 21], np.full(n, fixed))[:, 3])
        )
        w, a, _, d = render(q, raw[:, 11], fs, ctl[:, 0], g, True)
        require(
            np.array_equal(w[a == 0].view("u8"), q[a == 0].view("u8")),
            "unsafe independent gain",
        )
        require(
            np.any(w[:, col] != q[:, col])
            and np.array_equal(w[:, col].view("u8"), w[:, 3].view("u8")),
            "constant delivery final wet",
        )
        parity.append(fixed)
    q[:, 3] = replay(fs, raw[:, 21], depth[:n])[:, 3]
    q[7500:12500, 3] += 0.5 * (-1.0) ** np.arange(5000)
    w, a, logs, d = render(q, raw[:, 11], fs, ctl[:, 0], g, True)
    independent = bank(q, fs, ctl[:, 0], g)
    close(d, independent, 2e-11, "fourth-bank correction")
    ew, ea, el = drive(q, independent, raw[:, 11], fs, ctl[:, 0], trusted=True)
    require(
        any(not v["realized_ok"] for v in el)
        and all(x["realized_ok"] == y["realized_ok"] for x, y in zip(logs, el)),
        "delivery-only realized veto",
    )
    require(
        np.any(a[:7500] > 0) and np.any(a[10000:15000] == 0) and a[-1] > 0,
        "delivery veto recovery",
    )
    close(a, ea, 2e-12, "independent scheduled slew")
    close(w, ew, 2e-11, "independent wet trajectory")
    q[0] = [-0.0, 0.0, -0.0, 0.0]
    w, a, _, _ = render(q, raw[:, 11], fs, ctl[:, 0], g, False)
    require(
        not np.any(a) and np.array_equal(w.view("u8"), q.view("u8")), "exact neutral"
    )
    return dict(parity=parity, vetoed=sum(not r["realized_ok"] for r in logs))


def lifecycle_behavior():
    from i1_clock import EventFrame
    from i1_clock import replay as capture

    n = 9000
    x = fixture(48000, n)
    events = events_at(479)
    events.update(
        {
            2500: EventFrame(9, (Gesture.RESET,)),
            2639: (Gesture.HOLD,),
            3000: (Gesture.COMPATIBLE,),
            3839: (Gesture.PICK,),
            5310: (Gesture.MUTE,),
            5370: (Dispose(1, 0.0),),
        }
    )
    log = capture(
        x,
        48000,
        {t: Frame(196) for t in range(239, n, 240)},
        events,
        identity_stamps=due_stamps(n, 48000),
    )
    row = {r["tick"]: r for r in log}
    require(
        row[479]["epoch"] == 1
        and row[2500]["epoch"] == 1
        and "invalid frame" in row[2500]["actions"],
        "physical epoch ownership",
    )
    require(
        "publish" not in row[2639]["actions"] and not any(row[2639]["held"]),
        "stale feature publication",
    )
    require(
        row[5370]["epoch"] is None and row[3839]["phase"] == "pending", "zero disposal"
    )
    job_states = []
    for pick in (0, 120):
        expiry = 5760 + pick
        ev = {
            pick: (Gesture.PICK,),
            expiry: (Confirm((0, 1), n), Launch((0, 1), 1, 0.0)),
        }
        cap = capture(
            x,
            48000,
            {t: Frame(196) for t in range(239, n, 240)},
            ev,
            identity_stamps=due_stamps(n, 48000),
        )
        prev = next(
            r["window"] for r in reversed(cap) if r["tick"] < expiry and r["window"]
        )
        z = next(r for r in cap if r["tick"] == expiry)
        require(
            z["epoch"] is None and z["job"] is None and not any(z["published"]),
            "expired capture publication",
        )
        job_states.append(prev[3] < expiry)
    require(job_states == [False, True], "expiry job phases")
    return job_states


def capture_projection():
    from acoustic_identity import analysis_stream, feature
    from i1_clock import replay as capture, rounded

    errors = []
    for fs, origin in ((48000, 0), (47001, 19)):
        n = 8200
        x = fixture(fs, n)
        d = fs // 16000
        da = fs // 8000
        hop = d * rounded(0.005 * fs / d)
        a2 = da * rounded(0.005 * fs / da)
        token = (int(origin != 0), 1)
        events = {
            origin + 1: (Gesture.PICK,),
            origin + 2 * a2 - 1: (Confirm(token, n),),
        }
        resets = []
        if origin:
            events[origin] = (Gesture.RESET,)
            resets = [origin]
        frames = {t: Frame(196, owner=token) for t in range(origin + a2 - 1, n, a2)}
        log = capture(x, fs, frames, events, identity_stamps=due_stamps(n, fs, resets))
        z, fa = analysis_stream(x[origin:], fs)
        published = np.zeros(12)
        m = rounded(4 * fa / 196)
        for row in log:
            if "publish" in row["actions"]:
                end = (row["tick"] - hop - origin + 1) // d
                target = feature(z[end - m : end], fa, 196)
                published += (-np.expm1(-hop / (0.04 * fs))) * (target - published)
                errors.append(
                    close(
                        row["published"], published, 2e-11, "sliced source projection"
                    )
                )
    require(len(errors) > 20, "source publication inventory")
    return dict(publications=len(errors), max_db_error=max(errors))


def retirement():
    fs, n = 48000, 16000
    ev = events_at(479)
    base = advance_lanes(play(ev, n))[0][0]
    close_tick = int(np.flatnonzero(base[:, 16])[0])
    trial = close_tick + 4
    for event in (Dispose(1, 0.0), Gesture.RESET):
        ev2 = {**ev, trial: (event,)}
        left, right = advance_lanes(play(ev2, n), play(ev, n))
        require(
            not left[0][trial:, 2].any() and not (left[0][trial:, 15] >= 0).any(),
            "retired epoch reuse",
        )
        if event == Gesture.RESET:
            require(not right[0][trial:, 2].any(), "stream generation reset")
        else:
            require(np.array_equal(right[0], base), "lane disposal progression")
    ev2 = {**ev, 6101: (Gesture.RESET,)}
    left, right = advance_lanes(
        play(ev2, n, faults={6100: "bank", 6102: "bank"}), play(ev, n)
    )
    require(
        len(left[3]) == 1
        and left[0][6101, 13] == -1
        and not left[0][6101:, 10].any()
        and right[0][6101, 0] == 0,
        "reset during cached fade",
    )
    updates = {
        7100: (1, None),
        7800: (1, Eligibility(0, 2, fs, 0, n)),
        8100: (1, Eligibility(0, 1, fs, 0, 8000)),
        8200: (1, Eligibility(0, 1, fs, 8200, n, pitch_range=(430, 450))),
        8500: (1, Eligibility(0, 1, fs, 8500, n)),
    }
    left = advance_lanes(play(ev, n, receipt_updates=updates, outage=(0, 0)))[0][0]
    require(
        left[7000, 7] > 0 and not left[7400:8500, 7].any() and left[-1, 7] > 0,
        "revocation freshness scope and causal recovery",
    )
    carrier = returns(fs, n, "uneven")
    raw, q, depth, ctl = (v.copy() for v in carrier)
    cut = 7001
    q[cut:, 3] *= -2
    ctl[cut:, 0] *= 1.2
    depth[cut:] = 0.0
    raw[cut:, 11] = 0.0
    original = advance_lanes(play(ev, n, carrier=carrier))[0][0]
    altered = advance_lanes(
        play(ev, n, carrier=(raw, q, depth, ctl), parts=(0, 7, 8193))
    )[0][0]
    require(
        np.array_equal(original[: 479 + cut], altered[: 479 + cut])
        and np.any(original[479 + cut :, 8] != altered[479 + cut :, 8]),
        "carrier automation causal prefix",
    )
    return dict(trial=trial, reset=6101, revoked=7100, renewed=8500)


def run():
    report = json.loads((OUT / "motion.json").read_text())
    return dict(
        joined_motion={
            str(fs): joined_witness(fs, report["rows"][f"steady-{fs}"]["reference"])
            for fs in RATE
        },
        collisions=collisions(),
        faults=fault_cases(),
        retirement=retirement(),
        capture_projection=capture_projection(),
        paths=path_behavior(),
        lifecycle=lifecycle_behavior(),
        equivalence={
            str((fs, kind, r)): differential(fs, kind, ratio=r)
            for fs, kind, r in [
                (48000, "steady", 1.0),
                (47001, "bend", 1.0),
                (192000, "vibrato", 1.0),
                (32000, "max-retune", 1.0),
                (48000, "degenerate", 1.0),
                (48000, "threshold-low", 10 ** (0.1425) * (1 - 2e-12)),
                (48000, "threshold-high", 10 ** (0.1425) * (1 + 2e-12)),
            ]
        },
    )


if __name__ == "__main__":
    (OUT / "stream-checks.json").write_text(
        json.dumps(run(), indent=2, allow_nan=False) + "\n"
    )
