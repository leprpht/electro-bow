"""Offline M2 cut-loop impulse and M4 chronological-history checks."""

from cmath import exp as cexp, phase
from itertools import product
from math import atan2, ceil, cos, exp, floor, pi, sin

omega = 2 * pi * 1320 / 32000
print(
    "phase max",
    max(
        abs(atan2(u * sin(omega), 1 - u + u * cos(omega)) / omega - u)
        for i in range(100001)
        for u in [i / 100000]
    ),
)


def tap_impulse(delay, reverse=False):
    m = floor(delay)
    u = delay - m
    weights = (u, 1 - u) if reverse else (1 - u, u)
    return {m + i: value for i, value in enumerate(weights) if value}


def convolve(first, second):
    response = {}
    for t, a in first.items():
        for s, b in second.items():
            response[t + s] = response.get(t + s, 0.0) + a * b
    return response


def circuit(period, reverse_bridge=False):
    bridge = convolve(tap_impulse(0.127 * period - 1, reverse_bridge),
                      {0: -0.98 * 0.125, 1: -0.98 * 0.75, 2: -0.98 * 0.125})
    nut = {n: -v for n, v in tap_impulse((1 - 0.127) * period).items()}
    return convolve(bridge, nut)


def absolute_delay(impulse, omega):
    # Anchor at the observed first arrival. The remaining positive taps lie
    # in one open half-plane from DC to omega, so their phase has no wrap.
    first = min(impulse)
    assert min(impulse.values()) > 0 and (max(impulse) - first) * omega < pi
    residual = sum(gain * cexp(-1j * omega * (n - first)) for n, gain in impulse.items())
    return first - phase(residual) / omega


worst = 0.0
mutant_errors = {}
for fs, fundamental in product(
    (32000, 44100, 48000, 88200, 96000, 176400, 192000), (65.41, 440, 1318.51)
):
    period = fs / fundamental
    omega0 = 2 * pi / period
    impulse = circuit(period)
    error = abs(absolute_delay(impulse, omega0) - period)
    assert error <= 0.003, (fs, fundamental, error)
    worst = max(worst, error)
    for name, wrong in (
        ("zero delay", {0: 0.98}),
        ("omitted sample", {n - 1: v for n, v in impulse.items()}),
        ("extra sample", {n + 1: v for n, v in impulse.items()}),
        ("reversed bridge weights", circuit(period, reverse_bridge=True)),
    ):
        error = abs(absolute_delay(wrong, omega0) - period)
        mutant_errors[name] = max(mutant_errors.get(name, 0.0), error)
print("absolute one-circuit max delay error, samples", worst)
for name, error in mutant_errors.items():
    assert error > 0.003, name
    print("absolute delay rejects", name, "max error, samples", error)
# Inject an impulse after each complete read. Exercise taps through repeated wraps.
for delay in (1, 3.25, 14):
    ring = [0.0] * 16
    for n in range(64):
        m = floor(delay)
        u = delay - m
        tap = (1 - u) * ring[(n - m) % 16] + u * ring[(n - m - 1) % 16]
        assert tap == (1 - u) * (n == m) + u * (n == m + 1)
        ring[n % 16] = float(n == 0)
beta, fs = 0.127, 48000
Q = [0, 0.005, 0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28]
mu = [0.35 + 0.85 * exp(-q / 0.01) + 0.45 * exp(-q / 0.1) for q in Q]
lo, hi = 0.08 + 0.1 * mu[5], 0.16 + 0.1 * mu[6]
q = 0.08 + 0.08 * (0.14 - lo) / (hi - lo)
print("upper slip, impulse", q, 0.14 - q, "static residual", 0.1 * mu[0] - 0.14)
# Independent lazy vs eager integer histories, including stale nonzero storage,
# lengthening both arms at sample 100, wraparound and saturation of seed age.
size = ceil(fs / 65) + 4


def check_history(neck):

    def seed(t):
        phase = t / (fs / 440)
        if neck:
            phase = phase + 1 - beta
        return (-1 if neck else 1) * (0.1 / beta) * (phase - floor(phase) - 0.5)

    eager = [seed(t) for t in range(-size, 0)]
    lazy = [3.25] * size

    def check(n, delay):
        m = floor(delay)
        u = delay - m
        reads = []
        for lag in (m, m + 1):
            age = min(n, size)
            a = lazy[(n - lag) % size] if lag <= age else seed(age - lag)
            reads.append((a, eager[size + n - lag]))
        assert (1 - u) * reads[0][0] + u * reads[1][0] == (1 - u) * reads[0][
            1
        ] + u * reads[1][1]

    for n in range(fs // 10):
        P = fs / (440 if n < 100 else 65)
        check(n, (1 - beta) * P if neck else beta * P - 1)
        if not neck:
            check(n, beta * P / 2)
            if n == 0:
                check(n, beta * P - 1 + 1)
                check(n, beta * P - 1 + 2)
        wave = sin(0.731 * n)
        eager.append(wave)
        lazy[n % size] = wave


for neck in (False, True):
    check_history(neck)
print("lazy taps agree", size, "slots,", fs // 10, "ticks per arm")
