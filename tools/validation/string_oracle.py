"""Offline double-precision eager-history reference from V03–V07."""

from itertools import product
from math import ceil, exp, floor

import numpy as np
from scipy.signal import find_peaks


def trace(fs, frequency, ratio, speed, lift, seconds):
    beta = 0.127
    period = fs / frequency
    db, dn = beta * period - 1, (1 - beta) * period
    count = ceil(period) + 4
    q = (0, 0.005, 0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28)
    mu = [0.35 + 0.85 * exp(-v / 0.01) + 0.45 * exp(-v / 0.1) for v in q]
    k = ratio * speed / mu[0]
    loads = [v + k * m for v, m in zip(q, mu)]

    def saw(t):
        return t - floor(t) - 0.5

    bridge, neck = [0.0] * count, [0.0] * count
    for t in range(-count, 0):
        bridge[t % count] = speed / beta * saw(t / period)
        neck[t % count] = -speed / beta * saw(t / period + 1 - beta)

    def read(history, n, delay):
        lag = floor(delay)
        u = delay - lag
        return (1 - u) * history[(n - lag) % count] + u * history[(n - lag - 1) % count]

    x1, x2 = read(bridge, -1, db), read(bridge, -2, db)
    sliding = 1
    tail = []
    for n in range(seconds * fs):
        x = read(bridge, n, db)
        a = -read(neck, n, dn)
        b = -0.98 * (0.125 * x + 0.75 * x1 + 0.125 * x2)
        x2, x1 = x1, x
        d = speed - a - b
        h, sign = abs(d), 1 if d >= 0 else -1
        if lift and n >= fs:
            j, sliding = 0.0, 0
        else:
            slip = 0.0
            if h > loads[0] or sliding == sign:
                if h >= loads[-1]:
                    slip = h - k * mu[-1]
                else:
                    for i in range(len(q) - 2, -1, -1):
                        width = loads[i + 1] - loads[i]
                        if width > 1e-12 and loads[i] <= h <= loads[i + 1]:
                            slip = q[i] + (q[i + 1] - q[i]) * (h - loads[i]) / width
                            break
            sliding = sign if slip > 1e-12 else 0
            j = d - sign * slip if sliding else d
        tail.append((read(bridge, n, beta * period / 2), sliding))
        bridge[n % count], neck[n % count] = a + j, b + j
    return np.array(tail).T


min_eta = 1
for fs, f, ratio in product(
    (32000, 44100, 48000, 96000, 192000),
    (65.41, 82.41, 196, 440, 880, 1318.51),
    (1.2, 1.8, 2.4),
):
    p, branch = trace(fs, f, ratio, 0.1, False, 3)
    y = p[-fs:]
    spectrum = abs(np.fft.rfft((y - y.mean()) * np.hanning(fs), 8 * fs))
    peak = 1 + np.argmax(spectrum[1:])
    l, m, h = np.log(spectrum[peak - 1 : peak + 2])
    measured = (peak + 0.5 * (l - h) / (l - 2 * m + h)) / 8
    hz = np.arange(len(spectrum)) / 8
    k = np.rint(hz / measured)
    mask = (
        (k >= 1)
        & (k * measured < fs / 2)
        & (abs(hz - k * measured) <= np.maximum(2, 0.001 * k * measured))
    )
    eta = np.sum(spectrum[mask] ** 2) / np.sum(spectrum[1:] ** 2)
    min_eta = min(min_eta, eta)
    assert abs(1200 * np.log2(measured / f)) <= 12 and eta >= 0.9
    assert y.std() > 0.04 and abs(20 * np.log10(y.std() / p[-2 * fs : -fs].std())) <= 2
    if f <= 880:
        peaks = find_peaks(spectrum)[0]
        order = np.rint(hz[peaks] / f)
        near = abs(hz[peaks] - order * f) <= np.maximum(
            2, order * f * (2 ** (15 / 1200) - 1)
        )
        loud = spectrum[peaks] > spectrum[peak] * 10 ** (-36 / 20)
        assert len(set(order[(order >= 2) & near & loud])) >= 3
    if (fs, f, ratio) == (48000, 196, 1.8):
        slips = branch[-fs:] != 0
        episodes = np.sum(slips & (branch[-fs - 1 : -1] == 0))
        assert 0.95 <= episodes / 196 <= 1.05 and abs(slips.mean() - 0.127) <= 0.05
        print("nominal onsets/duty", episodes, slips.mean())
print("90 spectra; minimum eta", min_eta)
fs = 48000
for f in (65.41, 82.41, 440):
    for lift in (True, False):
        p, _ = trace(fs, f, 1.8, 0.1, lift, 7)
        delta_db = 20 * np.log10(p[-9600:].std() / p[38400:48000].std())
        assert delta_db <= -60 if lift else abs(delta_db) <= 2
        print(f, lift, delta_db)
band_fractions = {}
velocities = {}
for ratio, speed in ((1.2, 0.1), (1.8, 0.1), (2.4, 0.1), (1.8, 0.06), (1.8, 0.16)):
    p = trace(fs, 440, ratio, speed, False, 3)[0, -fs:]
    power = abs(np.fft.rfft((p - p.mean()) * np.hanning(fs))) ** 2
    fraction = power[1500:6001].sum() / power[1:].sum()
    band_fractions[ratio, speed] = fraction
    velocities[ratio, speed] = p.std()
    print(ratio, speed, p.std(), fraction)
assert band_fractions[2.4, 0.1] - band_fractions[1.2, 0.1] >= 0.01
assert velocities[1.8, 0.16] / velocities[1.8, 0.06] >= 1.8
for lift, expected in ((False, 0.228131), (True, 0.000144)):
    p, _ = trace(fs, 82.41, 1.8, 0.1, lift, 6)
    assert abs(p[-fs:].std() - expected) < 0.000002
    print("six-second low E", lift, p[-fs:].std())
