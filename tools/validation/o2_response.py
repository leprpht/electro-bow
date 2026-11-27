"""Stage 06 isolated binary64 candidate and independent electrical bench."""

import math

import numpy as np
from scipy import signal, optimize

from acoustic_observation import SPECTRAL_FLOOR
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
TAIL_TOL = 1e-10


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
    """Production order. Return prospective taps at f64; finite offline inputs."""
    c, b0, b1, b2, a1, a2 = coeff
    previous, h, z1, z2 = initial
    blocked, loaded = np.empty(len(x)), np.empty(len(x))
    for i, v in enumerate(x):
        h = c * (h + v - previous)
        previous = v
        a = 0.85 * h / ROOT2
        l = b0 * a + z1
        z1 = (b1 * a - a1 * l) + z2
        z2 = b2 * a - a2 * l
        blocked[i], loaded[i] = h, l
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


def check_rate(fs):
    coeff = coefficients(fs)
    c, *lp = coeff
    b, a = lp[:3], [1.0, *lp[3:]]
    poles = abs(np.roots(a))
    require(max(c, *poles) < 1, "unstable poles")
    n = 2 ** math.ceil(math.log2(0.5 * fs))
    x = np.zeros(n)
    x[0] = 1
    h, l, state = apply_load(x, coeff)
    freq = np.fft.rfftfreq(n, 1 / fs)
    errors = {}
    for noise in (0, 0.5, 1):
        measured = np.fft.rfft(blend(h, l, noise)) * ROOT2 / 0.85
        errors[str(noise)] = check_transfer(measured, reference(fs, freq, noise))
    tail = max(abs(l[-1]), *map(abs, state))
    require(tail < TAIL_TOL, "impulse tail")

    def lp_at(hz):
        return signal.freqz(b, a, worN=[hz], fs=fs)[1][0]

    cutoff = optimize.brentq(lambda f: abs(lp_at(f)) - 1 / ROOT2, 1000, 6000)
    require(abs(cutoff - 3200) < 1e-6, "cutoff")
    # One second scalar step, followed by 59 more seconds of the same step.
    # Hold v exactly 1. In h'=c*(h+1-1) roundoff reaches a fixed point.
    step_h, step_l, end = apply_load(np.ones(math.ceil(fs)), coeff)
    remaining = math.ceil(60 * fs) - len(step_l)
    final_h = c * (end[1] + 1 - 1)
    require(final_h == end[1], "step blocker failed to settle")
    remaining_output, final_z = signal.lfilter(
        b, a, np.full(remaining, 0.85 * final_h / ROOT2), zi=end[2:]
    )
    require(
        np.isfinite(remaining_output).all()
        and np.max(abs(remaining_output)) < TAIL_TOL
        and max(abs(final_h), *abs(final_z)) < TAIL_TOL,
        "long step",
    )
    response = []
    times = np.arange(n) / fs
    for f in [0, 20, 82.41, 196, 440, 1000, 3200, 6000, 12000, fs / 2]:
        val = reference(fs, f)
        eligible = abs(val) > SPECTRAL_FLOOR
        # Symmetric complex log derivative at interior nonzero frequencies.
        delay = (
            (
                -np.angle(reference(fs, f + 0.001) / reference(fs, f - 0.001))
                / (0.002 * 2 * np.pi)
            )
            if eligible
            else None
        )
        transform = np.exp(-2j * np.pi * f * times) * l * ROOT2 / 0.85
        observed = transform.sum()
        check_transfer(observed, val)
        measured_delay = (
            float(((times @ transform) / observed).real) if eligible else None
        )
        if eligible:
            require(abs(measured_delay - delay) < 1e-8, "group delay seconds")
        response.append(
            dict(
                hz=f,
                re=float(observed.real),
                im=float(observed.imag),
                lp_abs=abs(lp_at(f)),
                phase_deg=np.angle(observed, deg=True) if eligible else None,
                delay_ms=1000 * measured_delay if eligible else None,
            )
        )
    return dict(
        rate=fs,
        coefficients=coeff,
        dc_pole=c,
        lp_pole_radii=poles.tolist(),
        cutoff_hz=cutoff,
        impulse_end=tail,
        impulse_frames=n,
        max_complex_error=errors,
        response=response,
        step_peak_FS=float(max(abs(step_l))),
        step_first_FS=step_l[0],
        nyquist_abs={
            str(noise): abs(np.fft.rfft(blend(h, l, noise))[-1]) * ROOT2 / 0.85
            for noise in (0, 0.5, 1)
        },
        step_final_state=[1.0, final_h, *final_z],
        long_step_frames=len(step_l) + remaining,
    )


def verify_taps(h, a, loaded, detail, common, noise):
    check_transfer(a, 0.85 * h / ROOT2)
    check_transfer(detail, loaded + 0.025 * noise * (a - loaded))
    check_transfer(common, ROOT2 * detail)
