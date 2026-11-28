"""Causal past-window ray selection and realized-output veto, binary64 offline oracle."""

import math

import numpy as np
from acoustic_identity import resonant_bands
from o2_response import require
from scipy import signal

ORDERS = np.arange(1, 13)
GRID = np.arange(16, -1, -1) / 16
FLOOR = 1e-12


def transfer(fs, hz, freq, coeff):
    fa = fs / (fs // 16000)
    rho = np.exp(-2 * np.pi * 0.22 * hz / fs)
    p = rho * np.exp(2j * np.pi * ORDERS * hz / fs)
    u = np.exp(-2j * np.pi * np.asarray(freq) / fs)[..., None]
    taper = np.clip((0.28 * fa - ORDERS * hz) / (0.03 * fa), 0, 1)
    return 1 + (
        (1 - rho) ** 2 * ((1 - p * u) ** -2 + (1 - p.conjugate() * u) ** -2)
    ) @ (coeff * taper)


def check_state(peak, input_peak):
    require(
        math.isfinite(peak) and peak <= input_peak + 1e-12, "contractive bank bound"
    )


def check_trajectory(alpha, ramp):
    require(
        np.all(np.isfinite(alpha))
        and np.min(alpha) >= 0
        and np.max(alpha) <= 1
        and np.max(abs(np.diff(alpha)), initial=0.0) <= 1 / ramp + 1e-12,
        "smoothing trajectory",
    )


def correction(q, fs, pitch, gains):
    """gains are linear correction coefficients per sample, or a frozen vector."""
    fq = np.broadcast_to(pitch, (len(q),))
    fa = fs / (fs // 16000)
    if np.all(fq == fq[0]):
        bank = resonant_bands(q, fs, fq[0], fa)
        return gains @ bank if gains.ndim == 1 else np.einsum("kn,nk->n", bank, gains)
    z = np.zeros(12, complex)
    zz = z.copy()
    delta = np.empty(len(q))
    peak = 0.0
    weights = np.broadcast_to(gains, (len(q), 12))
    for n, f in enumerate(fq):
        r = math.exp(-2 * math.pi * 0.22 * f / fs)
        p = r * np.exp(2j * np.pi * f * ORDERS / fs)
        z = p * z + (1 - r) * q[n]
        zz = p * zz + (1 - r) * z
        peak = max(peak, np.max(abs(z)), np.max(abs(zz)))
        taper = np.clip((0.28 * fa - f * ORDERS) / (0.03 * fa), 0, 1)
        delta[n] = 2 * np.dot(taper * weights[n], zz.real)
    check_state(peak, max(abs(q)))
    return delta


def moments(q, delta):
    # Two columns span all constant ray scales. Hermitian projections include
    # DC/Nyquist once. Other positive frequencies have a conjugate partner.
    x = np.column_stack((q, delta))
    n = len(x)
    phase = np.exp((-2j * np.pi * 8 / n) * np.arange(n))
    projections = np.vstack(
        (
            x.mean(axis=0),
            phase @ x / n,
            (phase**2) @ x / n,
            (x[::2].sum(axis=0) - x[1::2].sum(axis=0)) / n,
        )
    )
    return x.T @ x / n, projections


def nonnegative(value, magnitude, count):
    # Conservative forward error envelope for length-N binary64 dot products.
    tolerance = 64 * count * np.finfo(float).eps * magnitude
    require(
        np.isfinite(value).all()
        and np.isfinite(tolerance).all()
        and np.all(value >= -tolerance),
        "nonfinite or negative moment",
    )
    return np.maximum(value, 0)


def quadratic(matrix, weights, count):
    terms = weights * (matrix @ weights)
    magnitude = np.sum(abs(weights) * (abs(matrix) @ abs(weights)), axis=0)
    return nonnegative(np.sum(terms, axis=0), magnitude, count)


def moving_basis(pitch, fs, initial):
    phase = initial + (2 * np.pi / fs) * np.cumsum(pitch)
    b = np.column_stack(
        (
            np.ones(len(phase)),
            np.cos(phase),
            np.sin(phase),
            np.cos(2 * phase),
            np.sin(2 * phase),
            (-1.0) ** np.arange(len(phase)),
        )
    )
    gram = b.T @ b / len(b)
    # Gershgorin lower/upper bounds. Same fixed cost as a rank screen in Rust.
    radii = np.sum(abs(gram), axis=1) - gram.diagonal()
    valid = min(gram.diagonal() - radii) >= 0.25 and max(gram.diagonal() + radii) <= 2
    return b, gram, valid, float(phase[-1] % (2 * np.pi))


def moving_test(b, gram, x, scales, mp, recovery):
    n = len(b)
    cross = b.T @ x / n
    coeff = np.linalg.solve(gram, cross)
    w = np.stack((np.ones_like(scales), scales))
    a = coeff @ w
    moment = x.T @ x / n
    power = quadratic(moment, w, n)
    fitted = quadratic(gram, a, n)
    rest = nonnegative(power - fitted, abs(power) + abs(fitted), n)

    def group(s):
        return np.sqrt(quadratic(gram[s, s], a[s], n))

    h1, h2 = group(slice(1, 3)), group(slice(3, 5))
    # Compare host-time RMS of fitted components and all orthogonal remainder.
    margin = 10 ** ((0.30 if recovery else 0.05) / 20)
    factor = 10 ** ((2.85 if recovery else 3.0) / 10)
    return (
        (mp > FLOOR**2)
        & (moment[0, 0] > FLOOR**2)
        & (
            h1
            >= margin * np.maximum.reduce((h2, np.sqrt(2) * abs(a[5]), np.sqrt(rest)))
            + FLOOR
        )
        & (power >= mp / factor)
        & (power <= mp * factor)
        & (power >= moment[0, 0] / factor)
        & (power <= moment[0, 0] * factor)
    )


def inspect(stats, scales, m5power, recovery):
    gram, projection = stats
    weights = np.stack((np.ones_like(scales), scales))
    power = np.sum(weights * (gram @ weights), axis=0)
    dc, h1, h2, nyq = abs(projection @ weights)
    used = dc**2 + 2 * h1**2 + 2 * h2**2 + nyq**2
    rest = nonnegative(power - used, abs(power) + used, 24000)
    competitor = np.maximum.reduce((h2, nyq, np.sqrt(rest / 2)))
    bound = 2.85 if recovery else 3.0
    factor = 10 ** (bound / 10)
    margin = 10 ** ((0.30 if recovery else 0.05) / 20)
    return (
        (m5power > FLOOR**2)
        & (gram[0, 0] > FLOOR**2)
        & (h1 > FLOOR)
        & (h1 >= margin * competitor + 1e-12)
        & (power >= m5power / factor)
        & (power <= m5power * factor)
        & (power >= gram[0, 0] / factor)
        & (power <= gram[0, 0] * factor)
    )


def smooth_targets(count, fs, updates):
    """Only coefficient targets change at publication. Same smoother on every sample."""
    targets = np.zeros((count, 12))
    for n, db in updates:
        if n < count:
            targets[n:] = np.expm1(np.log(10) * np.asarray(db) / 20)
    a = -math.expm1(-1 / (0.015 * fs))
    return signal.lfilter([a], [1, a - 1], targets, axis=0)


def historical_run(carriers, m5, fs, pitch, gains, force_unavailable=()):
    """Use the actual off/factory/full return streams on one physical clock.
    Returned arrays are off/factory/full before signed G. At most 17 candidate trials.
    """
    q = np.column_stack(carriers)
    delta = np.column_stack([correction(s, fs, pitch, gains) for s in carriers])
    return drive(q, delta, m5, fs, pitch, force_unavailable, moving=False)


def permission_block(q, delta, out, m5, fs, fq, moving, phase, trusted, observable):
    mp = np.mean(m5**2)
    length = len(m5)
    require(
        np.isfinite(mp) and all(np.isfinite(x).all() for x in (q, delta, out, fq)),
        "nonfinite monitor input",
    )
    if moving:
        b, gram, valid, phase = moving_basis(fq, fs, phase)
    else:
        valid = 1200 * np.log2(max(fq) / min(fq)) <= 25

    def test(x, d, scales, recovery):
        if not valid:
            return np.zeros(len(scales), dtype=bool)
        if moving:
            return moving_test(b, gram, np.column_stack((x, d)), scales, mp, recovery)
        return inspect(moments(x, d), scales, mp, recovery)

    bridge_ok = bool(test(m5, np.zeros(length), np.zeros(1), False)[0])
    available = trusted and observable and valid and bridge_ok
    masks = np.ones(len(GRID), bool)
    live_ok = True
    for j in range(q.shape[1]):
        if j < 3:
            masks &= test(q[:, j], delta[:, j], GRID, True)
        live_ok &= bool(test(q[:, j], out[:, j] - q[:, j], np.ones(1), False)[0])
    pair = np.column_stack((q[:, 1] - q[:, 0], delta[:, 1] - delta[:, 0]))
    off = np.column_stack((q[:, 0], delta[:, 0]))
    gram, off_gram = pair.T @ pair / length, off.T @ off / length
    w = np.stack((np.ones_like(GRID), GRID))
    masks &= quadratic(gram, w, length) <= 10 ** (-28.5 / 10) * quadratic(
        off_gram, w, length
    )
    live_ok &= np.mean((out[:, 1] - out[:, 0]) ** 2) <= 10 ** (-28 / 10) * np.mean(
        out[:, 0] ** 2
    )
    return phase, masks, bool(live_ok), bool(available), bool(valid), bridge_ok


def drive(q, delta, m5, fs, pitch, force_unavailable=(), *, moving=True, trusted=None):
    trusted = (not moving) if trusted is None else trusted is True
    n = len(q)
    fq = np.broadcast_to(pitch, (n,))
    out = np.empty_like(q)
    alpha = np.zeros(n)
    logs = []
    start = 0
    phase = 0.0
    scale = target = next_target = 0.0
    ready = 0
    latched = True
    ramp = max(1, math.floor(0.005 * fs + 0.5))
    while start < n:
        length = 2 * math.floor(4 * fs / fq[start] + 0.5)
        end = min(start + length, n)
        # Last block's trials run one per tick. First changed target at offset 17.
        previous_target = target
        for a, b, dest in (
            (start, min(start + 17, end), target),
            (min(start + 17, end), end, next_target),
        ):
            if b > a:
                alpha[a:b] = scale + np.sign(dest - scale) * np.minimum(
                    abs(dest - scale), np.arange(1, b - a + 1) / ramp
                )
                scale = alpha[b - 1]
        target = next_target
        sl = slice(start, end)
        out[sl] = q[sl]
        active = alpha[sl] != 0
        out[sl][active] = q[sl][active] + alpha[sl][active, None] * delta[sl][active]
        if end - start != length:
            break
        observable = all(not (start < hi and end > lo) for lo, hi in force_unavailable)
        phase, masks, live_ok, available, valid, bridge_ok = permission_block(
            q[sl],
            delta[sl],
            out[sl],
            m5[sl],
            fs,
            fq[sl],
            moving,
            phase,
            trusted,
            observable,
        )
        if not available or not live_ok or not masks[-1]:
            ready = 0
            latched = True
        else:
            ready = min(2, ready + 1)
            if ready == 2:
                latched = False
        next_target = 0.0 if latched else float(GRID[np.flatnonzero(masks)[0]])
        logs.append(
            {
                "start": start,
                "end": end,
                "conditioned": bool(valid),
                "observable": observable,
                "trusted": trusted,
                "available": bool(available),
                "bridge_ok": bridge_ok,
                "neutral_ok": bool(masks[-1]),
                "realized_ok": bool(live_ok),
                "veto": latched,
                "scale": next_target,
            }
        )
        if moving:
            before_slew = np.full(length, target)
            before_slew[:17] = previous_target
            ideal = q[sl, -1] + before_slew * delta[sl, -1]
            streams = np.column_stack((q[sl, -1], delta[sl, -1], out[sl, -1]))
            logs[-1]["delivery_pre_slew_power"] = float(np.mean(ideal**2))
            logs[-1]["delivery_realized_power"] = float(np.mean(out[sl, -1] ** 2))
            logs[-1]["delivery_gram_q_d_y"] = (streams.T @ streams / length).tolist()
            logs[-1]["paired_power"] = float(np.mean((out[sl, 1] - out[sl, 0]) ** 2))
        start = end
    check_trajectory(alpha, ramp)
    require(np.all(np.isfinite(out)), "finite output")
    return out, alpha, logs
