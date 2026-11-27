"""Stage 06 isolated binary64 candidate and independent electrical bench."""

import math

import numpy as np
from guide_qualification import RATES

ROOT2 = math.sqrt(2)
RATES_EXTENDED = sorted(
    [
        *RATES,
        math.nextafter(32000.0, math.inf),
        44150.5,
        48100.0,
        math.nextafter(192000.0, 0),
    ]
)
COMPLEX_TOL = 1e-9


def require(condition, message):
    if not condition:
        raise ValueError(message)


def coefficients(fs):
    require(math.isfinite(fs) and 32000 <= fs <= 192000, "unsupported rate")
    c = math.exp(-2 * math.pi * 20 / fs)
    k = math.tan(math.pi * 3200 / fs)
    d = 1 + ROOT2 * k + k * k
    b = k * k / d
    return c, b, 2 * b, b, 2 * (k * k - 1) / d, (1 - ROOT2 * k + k * k) / d


def apply_load(x, coeff, initial=(0.0, 0.0, 0.0, 0.0)):
    """Stage 06 offline candidate order. Return prospective taps at f64; finite offline inputs."""
    c, b0, b1, b2, a1, a2 = coeff
    previous, h, z1, z2 = initial
    blocked, loaded = np.empty(len(x)), np.empty(len(x))
    for i, v in enumerate(x):
        h = c * (h + v - previous)
        previous = v
        a = 0.85 * h / ROOT2
        sample = b0 * a + z1
        z1 = (b1 * a - a1 * sample) + z2
        z2 = b2 * a - a2 * sample
        blocked[i], loaded[i] = h, sample
    require(
        np.isfinite(blocked).all()
        and np.isfinite(loaded).all()
        and np.isfinite([z1, z2]).all(),
        "nonfinite stage or state",
    )
    return blocked, loaded, (previous, h, z1, z2)


def blend(blocked, loaded, noise):
    return loaded + (0.025 * noise) * (0.85 * blocked / ROOT2 - loaded)


def reference(fs, hz, noise=0):
    # Analog prototype evaluated on the warped imaginary axis, no biquad coefficients.
    omega = 2 * np.pi * hz / fs
    u = np.tan(omega / 2) / np.tan(np.pi * 3200 / fs)
    lp = 1 / (1 - u * u + 1j * ROOT2 * u)
    c = np.exp(-2 * np.pi * 20 / fs)
    dc = c * (-np.expm1(-1j * omega)) / (1 - c * np.exp(-1j * omega))
    return dc * (lp + (0.025 * noise) * (1 - lp))


def check_transfer(observed, expected):
    error = float(np.max(abs(observed - expected)))
    require(error <= COMPLEX_TOL, "complex transfer")
    return error


def verify_taps(h, a, loaded, detail, common, noise):
    check_transfer(a, 0.85 * h / ROOT2)
    check_transfer(detail, loaded + 0.025 * noise * (a - loaded))
    check_transfer(common, ROOT2 * detail)
