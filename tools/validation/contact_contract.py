"""Independent graph-intersection oracle for M3 and sample-index/host arithmetic checks."""

from bisect import bisect_right
from itertools import product
from math import copysign, exp, floor, inf, isfinite, nan, nextafter
from sys import float_info

Q = (0.0, 0.005, 0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28)
MU = tuple(0.35 + 0.85 * exp(-q / 0.01) + 0.45 * exp(-q / 0.1) for q in Q)


def graph(d, k, previous):
    if not isfinite(k) or not isfinite(d) or k < 0:
        return None
    if k == 0:
        return 0.0, d, 0
    if not isfinite(k * MU[0]):
        return None
    h = abs(d)
    sign = 1 if d > 0 else -1
    if d == 0 or (previous != sign and h <= k * MU[0]):
        return d, 0.0, 0
    # Enumerate feasible positive-slope intersections in forward order, then maximize q.
    loads = [q + k * mu for q, mu in zip(Q, MU)]
    roots = [h - k * MU[-1]] if h >= loads[-1] else []
    for i, (left, right) in enumerate(zip(loads, loads[1:])):
        if left <= h <= right and right - left > 1e-12:
            roots.append(Q[i] + (Q[i + 1] - Q[i]) * (h - left) / (right - left))
    if not roots:
        if h <= k * MU[0] + 1e-12:
            return copysign(min(h, k * MU[0]), d), 0.0, 0
        return None
    q = max(roots)
    if q <= 1e-12:
        return d, 0.0, 0
    return d - sign * q, sign * q, sign


def verify(d, k, prev):
    j, r, mode = graph(d, k, prev)
    v_bow, a, z = 0.1, 0.13, 1.0
    b = v_bow - a - d
    v = a + b + j
    force = 2 * z * j
    power = z * ((a + j) ** 2 + (b + j) ** 2 - a * a - b * b)
    assert abs(power - force * v) < 1e-10 and force * r >= -1e-12
    assert abs((v_bow - v) - r) < 1e-10 and abs(d - r - j) < 1e-10
    assert abs(force) <= 2 * z * k * MU[0] + 2e-10
    if k == 0:
        assert (j, r, mode) == (0, d, 0)
    elif mode == 0:
        assert abs(r) < 1e-10 and abs(d - j) < 1e-10
    else:
        q = abs(r)
        i = bisect_right(Q, q) - 1
        mu = MU[i]
        if i + 1 < len(Q):
            mu += (MU[i + 1] - mu) * (q - Q[i]) / (Q[i + 1] - Q[i])
        assert r * d > 0 and abs(q + k * mu - abs(d)) < 1e-10
    return mode


if __name__ == "__main__":
    count = 0
    for k in (0.0, 0.03, 0.1, 0.23):
        seq = [i / 1000 for i in range(-2000, 2001)]
        knots = [q + k * mu for q, mu in zip(Q, MU)]
        for h in knots + [knots[0] - 1e-11, knots[0] + 1e-11]:
            seq.extend([h, -h])
        for d, prev in product(seq, (-1, 0, 1)):
            verify(d, k, prev)
            count += 1
        for direction in (sorted(seq), sorted(seq, reverse=True)):
            previous = 0
            for d in direction:
                previous = verify(d, k, previous)
                count += 1
    vectors = [
        (0.14, 0.1, 0, 0.14, 0.0, 0),
        (0.14, 0.1, 1, 0.05447776066490598, 0.08552223933509404, 1),
        (-0.14, 0.1, 1, -0.14, 0.0, 0),
        (-0.20, 0.1, 1, -0.044747854541221044, -0.15525214545877897, -1),
        (0.14, 0.0, 1, 0.0, 0.14, 0),
    ]
    for d, k, prev, expected_j, expected_r, expected_mode in vectors:
        j, r, mode = graph(d, k, prev)
        assert (
            abs(j - expected_j) < 1e-10
            and abs(r - expected_r) < 1e-10
            and mode == expected_mode
        )
        print("d,k,previous =", d, k, prev, " j,r,mode =", j, r, mode)
    # Exactly horizontal segment (0.04,0.08), both endpoints meet the upper branch.
    k = (Q[5] - Q[4]) / (MU[4] - MU[5])
    h = Q[5] + k * MU[5]
    assert abs(graph(h, k, 1)[1] - 0.08) < 1e-10
    print("flat fold", k, h, graph(h, k, 1))
    for d, k in (
        (nan, 0.0),
        (inf, 0.1),
        (0.0, nan),
        (0.0, -0.1),
        (0.14, float_info.max),
    ):
        assert graph(d, k, 0) is None
    print("branch/flux cases", count)
    for fs, expected in (
        (32000, 160),
        (44100, 221),
        (44150, 221),
        (48000, 240),
        (192000, 960),
    ):
        n = max(1, floor(0.005 * fs + 0.5))
        assert n == expected
        ramp = [(n - i - 1) / n for i in range(n)]
        assert (
            ramp[-1] == 0
            and 0 < ramp[0] < 1
            and all(b < a for a, b in zip(ramp, ramp[1:]))
        )
        print("fault fade", fs, n, ramp[0], n - 1)
    edge = 1200.0 / float_info.max
    for fs in (
        1e-307,
        nextafter(edge, 0.0),
        edge,
        nextafter(edge, inf),
        4000.0,
        48000.0,
    ):
        print("host tail divided at", fs.hex(), 1200.0 / fs)
    assert edge.hex() == "0x1.2c00000000001p-1014"
    assert not isfinite(1200.0 / 1e-307) and not isfinite(1200.0 / nextafter(edge, 0.0))
    assert isfinite(1200.0 / edge) and isfinite(1200.0 / nextafter(edge, inf))
