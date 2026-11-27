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


def inspect(stats, scales, m5power, recovery):
    gram, projection = stats
    weights = np.stack((np.ones_like(scales), scales))
    power = np.sum(weights * (gram @ weights), axis=0)
    dc, h1, h2, nyq = abs(projection @ weights)
    rest = np.maximum(0, power - dc**2 - 2 * h1**2 - 2 * h2**2 - nyq**2)
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


def run(carriers, m5, fs, pitch, gains, force_unavailable=()):
    """Use the actual off/factory/full return streams on one physical clock.
    Returned arrays are off/factory/full before signed G. At most 17 candidate trials.
    """
    q = np.column_stack(carriers)
    delta = np.column_stack([correction(s, fs, pitch, gains) for s in carriers])
    return drive(q, delta, m5, fs, pitch, force_unavailable)


def drive(q, delta, m5, fs, pitch, force_unavailable=()):
    n = len(q)
    fq = np.broadcast_to(pitch, (n,))
    out = np.empty_like(q)
    alpha = np.zeros(n)
    logs = []
    start = 0
    scale = target = next_target = 0.0
    ready = 0
    latched = True
    ramp = max(1, math.floor(0.005 * fs + 0.5))
    while start < n:
        length = 2 * math.floor(4 * fs / fq[start] + 0.5)
        end = min(start + length, n)
        # Last block's trials run one per tick. First changed target at offset 17.
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
        out[sl] = q[sl] + alpha[sl, None] * delta[sl]
        if end - start != length:
            break
        mp = np.mean(m5[sl] ** 2)
        available = (
            np.min(fq[sl]) > 0
            and 1200 * np.log2(max(fq[sl]) / min(fq[sl])) <= 25
            and all(not (start < hi and end > lo) for lo, hi in force_unavailable)
        )
        bridge_ok = bool(
            inspect(moments(m5[sl], np.zeros(length)), np.zeros(1), mp, False)[0]
        )
        available = available and bridge_ok
        masks = np.ones(len(GRID), bool)
        live_ok = True
        for j in range(3):
            masks &= inspect(moments(q[sl, j], delta[sl, j]), GRID, mp, True)
            # Use a separate realized basis so coloration remains relative to q.
            live_ok &= bool(
                inspect(
                    moments(q[sl, j], out[sl, j] - q[sl, j]), np.ones(1), mp, False
                )[0]
            )
        gram, _ = moments(q[sl, 1] - q[sl, 0], delta[sl, 1] - delta[sl, 0])
        off_gram, _ = moments(q[sl, 0], delta[sl, 0])
        w = np.stack((np.ones_like(GRID), GRID))
        masks &= np.sum(w * (gram @ w), axis=0) <= 10 ** (-28.5 / 10) * np.sum(
            w * (off_gram @ w), axis=0
        )
        live_ok &= np.mean((out[sl, 1] - out[sl, 0]) ** 2) <= 10 ** (
            -28 / 10
        ) * np.mean(out[sl, 0] ** 2)
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
                "end": end,
                "available": bool(available),
                "bridge_ok": bridge_ok,
                "neutral_ok": bool(masks[-1]),
                "realized_ok": bool(live_ok),
                "veto": latched,
                "scale": next_target,
            }
        )
        start = end
    check_trajectory(alpha, ramp)
    require(np.all(np.isfinite(out)), "finite output")
    return out, alpha, logs
