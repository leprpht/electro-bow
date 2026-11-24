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


# Phase after one full circuit, measured on the returning wave at the cut.
# The complex FIR retains the minus sign of the bridge reflection; the nut also inverts.
def tap_response(delay, z):
    lag = floor(delay)
    u = delay - lag
    return z**lag * (1 - u + u * z)


worst = 0.0
for fs, fundamental in product(
    (32000, 44100, 48000, 88200, 96000, 176400, 192000), (65.41, 440, 1318.51)
):
    period = fs / fundamental
    omega0 = 2 * pi / period
    z = cexp(-1j * omega0)
    bridge = (
        -0.98 * (0.125 + 0.75 * z + 0.125 * z * z) * tap_response(0.127 * period - 1, z)
    )
    nut = -tap_response((1 - 0.127) * period, z)
    error = abs(phase(bridge * nut * cexp(1j * omega0 * period)) / omega0)
    worst = max(worst, error)
assert worst <= 0.003
print("one-circuit max delay error, samples", worst)
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
        phase = t / (fs / 440) + ((1 - beta) if neck else 0)
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
            reads.append((a, eager[(n - lag) % size]))
        assert (1 - u) * reads[0][0] + u * reads[1][0] == (1 - u) * reads[0][
            1
        ] + u * reads[1][1]

    for n in range(3 * size):
        P = fs / (440 if n < 100 else 65)
        check(n, (1 - beta) * P if neck else beta * P - 1)
        if not neck:
            check(n, beta * P / 2)
            if n == 0:
                check(n, beta * P - 1 + 1)
                check(n, beta * P - 1 + 2)
        eager[n % size] = lazy[n % size] = sin(0.731 * n)


for neck in (False, True):
    check_history(neck)
print("lazy taps agree", size)
