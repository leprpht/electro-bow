"""Independent graph-intersection oracle for M3 and sample-index/host arithmetic checks."""

import argparse
import subprocess
from bisect import bisect_right
from decimal import Decimal, localcontext
from fractions import Fraction
from itertools import product
from math import copysign, exp, floor, inf, isfinite, nan, nextafter, ulp
from struct import unpack
from sys import float_info

Q = (0.0, 0.005, 0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28)
MU = tuple(0.35 + 0.85 * exp(-q / 0.01) + 0.45 * exp(-q / 0.1) for q in Q)


def prepared_coefficients(binary):
    words = subprocess.check_output([binary, "--coefficients"], text=True).split()
    assert len(words) == len(Q) and all(len(word) == 16 for word in words)
    actual = tuple(unpack(">d", bytes.fromhex(word))[0] for word in words)
    assert actual[0] == 1.65
    # Independent M3a preparation check. Bracket each high-precision exp by
    # one representable neighbor each way, then propagate through the f64 sums.
    with localcontext() as ctx:
        ctx.prec = 80
        for q, mu in zip(Q, actual):
            e1, e2 = (float(Decimal.from_float(-q / t).exp()) for t in (0.01, 0.1))
            bounds = [
                0.35 + 0.85 * nextafter(e1, end) + 0.45 * nextafter(e2, end)
                for end in (0.0, inf)
            ]
            assert bounds[0] <= mu <= bounds[1], (q, mu, bounds)
    print("Rust M3a coefficient bits", *words)
    return actual


def intersections(h, k):
    """Enumerate the entire unsigned graph, including unstable and flat roots."""
    loads = [q + k * mu for q, mu in zip(Q, MU)]
    roots = []
    for i, (left, right) in enumerate(zip(loads, loads[1:])):
        if min(left, right) <= h <= max(left, right):
            if left == right:
                # The full interval intersects. Its endpoints suffice for extrema.
                roots.extend((q, "flat", i) for q in Q[i : i + 2])
            else:
                # Exact barycentric interpolation of stored f64 endpoints;
                # this also works for the descending intersection.
                lo, hi, x = map(Fraction, (left, right, h))
                q = ((hi - x) * Fraction(Q[i]) + (x - lo) * Fraction(Q[i + 1])) / (hi - lo)
                roots.append((float(q), "rising" if left < right else "falling", i))
    if h >= loads[-1]:
        roots.append((Q[-1] if h == loads[-1] else h - k * MU[-1], "tail", len(Q) - 1))
    return roots


def above_slip_floor(root, h, k):
    q, slope, i = root
    if abs(q - 1e-12) > 4 * ulp(q):
        return q > 1e-12
    # The graph already selected the intersection. Only its threshold decision
    # uses the normative evaluation order inside the interpolation error band.
    left = Q[i] + k * MU[i]
    if slope == "tail":
        rounded = Q[i] if h == left else h - k * MU[i]
    else:
        right = Q[i + 1] + k * MU[i + 1]
        if h == right:
            rounded = Q[i + 1]
        else:
            fraction = (h - left) / (right - left)
            rounded = Q[i] + (Q[i + 1] - Q[i]) * fraction
    return rounded > 1e-12


def graph(d, k, previous):
    if not isfinite(k) or not isfinite(d) or k < 0:
        return None
    if k == 0:
        return 0.0, d, 0
    limit = k * MU[0]
    if not isfinite(limit):
        return None
    h = abs(d)
    sign = 1 if d > 0 else -1
    j, slip, mode = d, 0.0, 0
    if d != 0 and (previous == sign or h > limit):
        admissible = [root for root in intersections(h, k) if root[1] != "falling"]
        if admissible:
            root = max(admissible, key=lambda entry: entry[0])
            q = root[0]
            if above_slip_floor(root, h, k):
                slip, mode = sign * q, sign
                j = d - slip
        elif h <= limit + 1e-12:
            j = copysign(min(h, limit), d)
        else:
            return None
    return (j, slip, mode) if isfinite(2 * j) else None


def verify_physics(d, k, solution, *, exact_reference=False):
    j, r, mode = solution
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
        assert (q > 1e-12 or (exact_reference and abs(q - 1e-12) <= 4 * ulp(q)))
        assert mode == copysign(1, r) and r * d > 0
        assert abs(q + k * mu - abs(d)) < 1e-10 and abs(j - copysign(k * mu, r)) < 1e-10
    return mode


def verify(d, k, prev):
    return verify_physics(d, k, graph(d, k, prev), exact_reference=True)


def sweep():
    for k in (0.0, 0.03, 0.1, 0.23):
        seq = [i / 1000 for i in range(-2000, 2001)]
        knots = [q + k * mu for q, mu in zip(Q, MU)]
        for h in knots + [knots[0] - 1e-11, knots[0] + 1e-11]:
            seq.extend([h, -h])
        for d, prev in product(seq, (-1, 0, 1)):
            yield d, k, prev
        for direction in (sorted(seq), sorted(seq, reverse=True)):
            previous = 0
            for d in direction:
                yield d, k, previous
                previous = graph(d, k, previous)[2]


def fold_cases():
    # Perturb both the load parameter and the query by single binary64 steps.
    for i in range(len(Q) - 1):
        for width in (0.0, 5e-13, 1e-14):
            center = (Q[i + 1] - Q[i] - width) / (MU[i] - MU[i + 1])
            for k in (nextafter(center, 0), center, nextafter(center, inf)):
                left, right = (Q[t] + k * MU[t] for t in (i, i + 1))
                for h in (left, right, (left + right) / 2):
                    for x in (nextafter(h, 0), h, nextafter(h, inf)):
                        for sign, previous in product((-1, 1), (-1, 0, 1)):
                            yield x * sign, k, previous


def compare(d, actual, expected):
    j, r, mode = actual
    ref_j, ref_r, ref_mode = expected
    assert mode == ref_mode, (d, actual, expected)
    tolerance = 4 * ulp(ref_r)
    assert abs(r - ref_r) <= tolerance, (d, actual, expected)
    assert abs(j - ref_j) < 1e-10
    # The two subtractions forming j and reconstructing r round separately.
    assert abs((d - j) - ref_r) <= tolerance + ulp(j) + ulp(d - j)


def counterexample():
    i = 3
    k = (Q[i + 1] - Q[i] - 5e-13) / (MU[i] - MU[i + 1])
    left, right = (Q[t] + k * MU[t] for t in (i, i + 1))
    d = (left + right) / 2
    # Re-form q+k*mu exactly from prepared knot bits, before rounding the loads.
    lo, hi = (Fraction(Q[t]) + Fraction(k) * Fraction(MU[t]) for t in (i, i + 1))
    fraction = (Fraction(d) - lo) / (hi - lo)
    q = Fraction(Q[i]) + (Fraction(Q[i + 1]) - Fraction(Q[i])) * fraction
    expected = graph(d, k, 1)
    assert expected[2] == 1 and Q[i] < expected[1] < Q[i + 1]
    print("review k,d,width,j,r,mode", k, d, right - left, *expected)
    print("root with unrounded loads from prepared knots", float(q))
    return d, k


def negative_controls():
    d, k = counterexample()
    # At the reported query the original solver exhausts its search and takes
    # the admissible static fallback. Check those preconditions explicitly.
    loads = [q + k * mu for q, mu in zip(Q, MU)]
    assert d < loads[-1] and d <= loads[0]
    assert not any(
        left <= d <= right and right - left > 1e-12
        for left, right in zip(loads, loads[1:])
    )
    cases = [("old cutoff", d, k, (d, 0.0, 0))]
    d, k = 0.14, 0.10
    cases.append(("always static when admissible", d, k, (d, 0.0, 0)))
    lower = min(q for q, _, _ in intersections(d, k))
    cases.append(("lower sliding intersection", d, k, (d - lower, lower, 1)))
    for name, d, k, wrong in cases:
        verify_physics(d, k, wrong)
        try:
            compare(d, wrong, graph(d, k, 1))
        except AssertionError:
            print("branch comparison rejected", name)
        else:
            raise AssertionError(name)


def check_rust(binary, cases):
    rows = "".join(f"{d!r} {k!r} {prev}\n" for d, k, prev in cases)
    output = subprocess.run([binary], input=rows, text=True, capture_output=True, check=True)
    lines = output.stdout.splitlines()
    assert len(lines) == len(cases)
    max_probes = 0
    min_dissipation = inf
    max_slip_ulps = 0.0
    for (d, k, prev), line in zip(cases, lines):
        j, r, mode, probes = line.split()
        actual = float(j), float(r), int(mode)
        expected = graph(d, k, prev)
        compare(d, actual, expected)
        min_dissipation = min(min_dissipation, 2 * actual[0] * actual[1])
        max_slip_ulps = max(max_slip_ulps, abs(actual[1] - expected[1]) / ulp(expected[1]))
        verify_physics(d, k, actual)
        assert 0 <= int(probes) <= 9
        max_probes = max(max_probes, int(probes))
    assert max_probes == 9 and min_dissipation >= 0.0
    print("minimum dissipation W", min_dissipation, "maximum slip error ULP", max_slip_ulps)
    print("Rust/oracle finite comparisons", len(cases), "max probes", max_probes)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust", help="compiled tools/validation/contact_replay.rs")
    args = parser.parse_args()
    if args.rust:
        MU = prepared_coefficients(args.rust)
    standard = list(sweep())
    assert len(standard) == 80500
    folds = list(fold_cases())
    assert len(folds) == 4374
    review_h, review_k = 1.7431847738615466e-12, 4.5041501448100105e-13
    floor_cases = [
        (s * h, review_k, s)
        for s in (-1, 1)
        for h in (nextafter(review_h, 0.0), review_h, nextafter(review_h, inf))
    ]
    # Target-prepared exact coincidence plus both adjacent queries, signed.
    center = (Q[5] - Q[4]) / (MU[4] - MU[5])
    neighbors = [center]
    lo = hi = center
    for _ in range(64):
        lo, hi = nextafter(lo, 0.0), nextafter(hi, inf)
        neighbors.extend((lo, hi))
    tie = next(k for k in neighbors if Q[4] + k * MU[4] == Q[5] + k * MU[5])
    h = Q[5] + tie * MU[5]
    ties = [
        (s * x, tie, s)
        for s in (-1, 1)
        for x in (nextafter(h, 0.0), h, nextafter(h, inf))
    ]
    for s in (-1, 1):
        assert graph(s * h, tie, s)[1:] == (s * Q[5], s)
    cases = standard + folds + floor_cases + ties
    for d, k, prev in cases:
        verify(d, k, prev)
    negative_controls()
    if args.rust:
        check_rust(args.rust, cases)
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
    print("stored flat fold", tie, Q[5] + tie * MU[5])
    for d, k in (
        (nan, 0.0),
        (inf, 0.1),
        (0.0, nan),
        (0.0, -0.1),
        (0.14, float_info.max),
        (-1e308, 1e308),
    ):
        assert graph(d, k, 0) is None
    print("standard/fold branch and flux cases", len(standard), len(folds))
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
