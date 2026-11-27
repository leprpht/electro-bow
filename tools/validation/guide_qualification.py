"""Run from repository root. Writes actual Rust traces, WAVs and measurements under build/."""

if not __debug__:
    raise RuntimeError("guide qualification requires enabled Python assertions")

from hashlib import sha256
from itertools import product
import json
from pathlib import Path
import subprocess

from guide_audit_build import build, PROBES

import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, find_peaks, hilbert, sosfiltfilt

OUT = Path("build/stage-03")
OUT.mkdir(parents=True, exist_ok=True)
BINARY = OUT / "guide-render"
RATES = [32000, 44100, 48000, 88200, 96000, 176400, 192000]
NOTES = [65.41, 82.41, 196, 440, 880, 1318.51]
RATIOS = [1.2, 1.8, 2.4]
COLUMNS = "a b B N x pickup j slip force string_velocity branch audio".split()
SOURCES = [
    Path("rust/dsp/src/instrument") / (name + ".rs")
    for name in ("waveguide", "ring", "friction")
]
ADAPTER = Path("tools/validation/guide_render.rs")


def digest(path):
    result = sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(2**20), b""):
            result.update(block)
    return result.hexdigest()


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


def db(ratio):
    if not np.isfinite(ratio) or ratio < 0:
        raise ValueError("expected finite nonnegative amplitude ratio")
    return float(20 * np.log10(max(ratio, 1e-12)))


def decay_levels(start, end):
    if not (np.isfinite(start) and start > 0):
        raise ValueError("decay requires a voiced reference RMS")
    ratio = end / start
    return dict(
        start_rms=start,
        end_rms=end,
        delta_db=db(ratio),
        unfloored_delta_db=float(20 * np.log10(ratio)) if ratio > 0 else None,
    )


def cents(ratio):
    return 1200 * np.log2(ratio)


def spectrum(signal, fs, target, include_orders=False):
    """One-second mean-free Hann, strongest positive peak and merged harmonic bins."""
    assert len(signal) == fs
    amp = abs(np.fft.rfft((signal - signal.mean()) * np.hanning(fs), 8 * fs))
    peaks = find_peaks(amp)[0]
    peak = peaks[np.argmax(amp[peaks])]
    left, mid, right = np.log(amp[peak - 1 : peak + 2])
    f = (peak + 0.5 * (left - right) / (left - 2 * mid + right)) / 8
    hz = np.arange(len(amp)) / 8
    harmonic = np.rint(hz / f)
    mask = (
        (harmonic >= 1)
        & (harmonic * f < fs / 2)
        & (abs(hz - harmonic * f) <= np.maximum(2, 0.001 * harmonic * f))
    )
    power = amp**2
    order = np.rint(hz[peaks] / target)
    near = abs(hz[peaks] - order * target) <= np.maximum(
        2, order * target * (2 ** (15 / 1200) - 1)
    )
    partials = sorted(
        set(
            order[near & (order >= 2) & (amp[peaks] > amp[peak] * 10 ** (-36 / 20))]
            .astype(int)
            .tolist()
        )
    )
    stats = dict(
        hz=f,
        cents=cents(f / target),
        eta=power[mask].sum() / power[1:].sum(),
        upper_partials=partials,
        rms=signal.std(),
        bright_fraction=power[12000:48001].sum() / power[1:].sum(),
    )
    if include_orders:
        # Index 0 of this list is the fundamental. Each power bin is assigned
        # once, to its nearest admitted harmonic band.
        fractions = (
            np.bincount(harmonic[mask].astype(int), power[mask]) / power[1:].sum()
        )
        stats["power_by_harmonic"] = fractions[1:].tolist()
    return stats


def trajectory(t, kind):
    if kind == "bend":
        return 200 * np.maximum(0, 1 - np.abs(t - 1.2) / 0.2)
    if kind == "vibrato":
        return 35 * np.sin(2 * np.pi * 5 * t)
    return np.zeros_like(t)


def render(fs, hz, ratio=1.8, speed=0.1, seconds=3, kind="steady", listening=False):
    name = f"{kind}_{fs}_{hz:g}_R{ratio:g}_V{speed:g}_{seconds}s"
    # Only the phase estimator consumes the extra right context. Delivered audio
    # and all published scoring windows end at the requested frame.
    pad = fs // 4 if kind == "vibrato" else 0
    frames = fs * seconds
    t = np.arange(frames + pad) / fs
    controls = np.empty((len(t), 3), dtype="<f8")
    controls[:, 0] = hz * np.exp2(trajectory(t, kind) / 1200)
    controls[:, 1] = speed
    controls[:, 2] = ratio * speed / 1.65
    if kind == "lift":
        controls[fs:, 2] = 0
    path = OUT / (name + ".f64le")
    with path.open("wb") as stream:
        run = subprocess.run(
            [BINARY, str(fs)],
            input=controls.tobytes(),
            stdout=stream,
            stderr=subprocess.PIPE,
        )
        if run.returncode:
            raise RuntimeError(run.stderr.decode())
    ticks, launches, faults, invalidations = map(int, run.stderr.split())
    assert (
        ticks == len(t)
        and launches == 1
        and faults == 0
        and invalidations == 1
        and path.stat().st_size == ticks * len(COLUMNS) * 8
    )
    data = np.memmap(path, dtype="<f8", mode="r", shape=(len(t), len(COLUMNS)))
    raw = data[:frames]
    assert np.isfinite(raw).all(), name
    maxima = dict(zip(COLUMNS, np.max(abs(raw), axis=0)))
    assert max(maxima[k] for k in "a b B N x pickup j".split()) <= 4
    entry = dict(
        rate=fs,
        hz=hz,
        ratio=ratio,
        speed=speed,
        k=ratio * speed / 1.65,
        seconds=seconds,
        trajectory=kind,
        phase_context_samples=pad,
        launches=launches,
        successful_ticks=ticks,
        faults=faults,
        invalidations=invalidations,
        epoch_checks=ticks + 1,
        raw=path.name,
        raw_sha256=digest(path),
        controls_sha256=sha256(controls.tobytes()).hexdigest(),
        peaks=maxima,
    )
    if listening:
        # Nominal M5 0.85 FS/(m/s), G=1, polarity positive, extra gain exactly one.
        audio = raw[:, -1]
        assert maxima["audio"] < 1
        wav = OUT / (name + ".wav")
        wavfile.write(wav, fs, np.asarray(audio, dtype="<f4"))
        entry.update(wav=wav.name, wav_sha256=digest(wav))
    RESULTS[name] = entry
    return entry, data


def static_case(fs, hz, ratio, speed=0.1):
    e, data = render(
        fs,
        hz,
        ratio,
        speed,
        listening=(
            fs == 48000 and ((ratio == 1.8 and hz in (82.41, 196, 440)) or hz == 440)
        ),
    )
    for key, col in [("bridge", 2), ("pickup", 5)]:
        stats = spectrum(
            data[-fs:, col], fs, hz, include_orders=(fs == 48000 and hz == 440)
        )
        stats["drift_db"] = db(stats["rms"] / data[-2 * fs : -fs, col].std())
        e[key] = stats
        assert abs(stats["cents"]) <= 12 and stats["eta"] >= 0.90, (
            e["raw"],
            key,
            stats,
        )
        assert stats["rms"] > 0.04 and abs(stats["drift_db"]) <= 2
        assert hz > 880 or len(stats["upper_partials"]) >= 3
    slip = data[-fs:, 10] != 0
    onsets = np.flatnonzero(slip & (data[-fs - 1 : -1, 10] == 0))
    e.update(
        slip_onsets=len(onsets),
        mean_slip_interval_ms=float(np.diff(onsets).mean() * 1000 / fs)
        if len(onsets) > 1
        else None,
        duty=slip.mean(),
        normal_force=2 * ratio * speed / 1.65,
        static_force_limit=2 * ratio * speed,
        force_rms=np.sqrt(np.mean(data[-fs:, 8] ** 2)),
        measured_period_ms=1000 / e["pickup"]["hz"],
    )
    pickup = data[-fs:, 5]
    power = abs(np.fft.rfft((pickup - pickup.mean()) * np.hanning(fs))) ** 2
    e["band_1500_6000"] = power[1500:6001].sum() / power[1:].sum()
    assert e["peaks"]["force"] <= e["static_force_limit"] + 2e-10
    if (fs, hz, ratio) == (48000, 196, 1.8):
        assert 0.95 <= e["slip_onsets"] / 196 <= 1.05 and abs(e["duty"] - 0.127) <= 0.05
    return e


def decay(fs, hz):
    prefix = None
    for kind in ("driven", "lift"):
        e, y = render(fs, hz, seconds=7, kind=kind, listening=(hz == 82.41))
        if prefix is None:
            prefix = y[:fs].copy()
        else:
            assert np.array_equal(prefix, y[:fs])
            assert np.all(y[fs:, 6] == 0) and np.all(y[fs:, 10] == 0)
        p = y[:, 5]
        start, end = p[4 * fs // 5 : fs].std(), p[-fs // 5 :].std()
        last = p[-fs:]
        e.update(
            **decay_levels(start, end),
            six_second_rms=p[5 * fs : 6 * fs].std(),
            last_second=None
            if kind == "lift" and last.std() == 0
            else spectrum(last, fs, hz),
        )
        assert e["delta_db"] <= -60 if kind == "lift" else abs(e["delta_db"]) <= 2
        # V05 measures pitch while voiced. Also retain the unconstrained peak in
        # the final second, even for the exponentially vanishing passive tail.
        e["first_second_after_lift_time"] = spectrum(p[fs : 2 * fs], fs, hz)
        assert abs(e["first_second_after_lift_time"]["cents"]) <= 12
        if kind == "driven":
            assert abs(e["last_second"]["cents"]) <= 12
        if hz == 82.41:
            expected = 0.00014420 if kind == "lift" else 0.22813106
            assert abs(e["six_second_rms"] - expected) < 0.000002


def stationarity(fs):
    e, y = render(fs, 196, seconds=60)
    rows = []
    for i in range(60):
        segment = y[i * fs : (i + 1) * fs]
        stats = spectrum(segment[:, 5], fs, 196)
        bridge = spectrum(segment[:, 2], fs, 196)
        stats.update(
            bridge_rms=bridge["rms"],
            bridge_hz=bridge["hz"],
            bridge_eta=bridge["eta"],
            mean_audio=segment[:, -1].mean(),
        )
        assert abs(bridge["cents"]) <= 12
        rows.append(stats)
        assert abs(stats["cents"]) <= 12 and abs(stats["mean_audio"]) < 0.002
    e.update(windows=rows, mean_audio=y[:, -1].mean())
    for key in ("rms", "bridge_rms"):
        rms = [row[key] for row in rows[1:]]
        e[key + "_drift_db"] = db(max(rms) / min(rms))
        assert e[key + "_drift_db"] < 2
    # Independently verify the declared observation on the full stream.
    from scipy.signal import lfilter

    c = np.exp(-2 * np.pi * 20 / fs)
    ref = lfilter([c, -c], [1, -c], y[:, 5]) * 0.85
    assert np.max(abs(y[:, -1] - ref)) < 1e-12


def dynamic(fs, hz, kind):
    e, y = render(fs, hz, kind=kind, listening=True)
    low, high = (150, 290) if hz == 196 else (335, 650)
    fund = sosfiltfilt(
        butter(3, [low, high], fs=fs, btype="bandpass", output="sos"), y[:, 5]
    )
    phase = np.unwrap(np.angle(hilbert(fund)))
    h = round(0.006 * fs)
    observed = cents(
        (phase[2 * h :] - phase[: -2 * h]) * (fs / (2 * h * 2 * np.pi * hz))
    )
    t = (np.arange(len(observed)) + h) / fs
    score = (t >= 1) & (t < (1.45 if kind == "bend" else 3))
    t, out = t[score], observed[score]

    def truth(lag_ms):
        return trajectory(t - lag_ms / 1000, kind)

    # Correlate on a fixed scored interval for all candidate lags.
    lag = max(range(46), key=lambda ms: np.corrcoef(out, truth(ms))[0, 1])
    rms = lambda err: np.sqrt(np.mean(err**2))
    step = round(0.012 * fs)
    slew = (out[step:] - out[:-step]) / (step / fs)
    e.update(
        lag_ms=lag,
        unaligned_cents=rms(out - truth(0)),
        aligned_cents=rms(out - truth(lag)),
        min_hz=hz * 2 ** (out.min() / 1200),
        max_hz=hz * 2 ** (out.max() / 1200),
        min_slew_cents_s=slew.min(),
        max_slew_cents_s=slew.max(),
        max_error_cents=np.max(abs(out - truth(lag))),
        max_sample_step=dict(
            zip(
                ("B", "N", "pickup"),
                np.max(abs(np.diff(y[: 3 * fs, [2, 3, 5]], axis=0)), axis=0),
            )
        ),
    )
    assert lag <= 40 and e["aligned_cents"] <= 15 and e["max_error_cents"] < 600
    if kind == "vibrato":
        # Frozen 3–8 Hz / .01 Hz grid, constant plus sine and cosine.
        tt, yy = t, out

        def fit(freq):
            phase = 2 * np.pi * freq * tt
            design = np.array([np.ones(len(tt)), np.cos(phase), np.sin(phase)]).T
            coef = np.linalg.lstsq(design, yy, rcond=None)[0]
            return np.mean((design @ coef - yy) ** 2), np.hypot(*coef[1:])

        _, freq, depth = min(
            (err, i / 100, amp) for i in range(300, 801) for err, amp in [fit(i / 100)]
        )
        e.update(modulation_hz=freq, depth_cents=depth)
        assert abs(freq - 5) <= 0.2 and 25 <= depth <= 42
        assert max(e["aligned_cents"], e["unaligned_cents"]) <= 6
    # Retain the measured and aligned instantaneous frequencies, not just extrema.
    path = OUT / (Path(e["raw"]).stem + "-phase.npz")
    np.savez_compressed(
        path,
        time_s=t,
        observed_cents=out,
        input_cents=truth(0),
        aligned_cents=truth(lag),
    )
    e.update(phase=path.name, phase_sha256=digest(path))


RESULTS = {}
if __name__ == "__main__":
    build(BINARY)
    sources = SOURCES + [
        ADAPTER,
        PROBES,
        Path(__file__),
        Path(__file__).with_name("guide_audit_build.py"),
    ]
    manifest = dict(
        revision=git("rev-parse", "HEAD"),
        source_status=git("status", "--porcelain"),
        source_sha256={p.name: digest(p) for p in sources},
        raw_format="row-major binary64 little endian",
        columns=COLUMNS,
        gain="M5 20 Hz DC blocker, .85 FS/(m/s), positive polarity, unit gain; float32 mono WAV",
        controls="constant V, k=R*V/1.65; lift k=0 from 1 s; 1.0-1.4 s 200-cent triangle; 5 Hz +/-35-cent sinusoid",
        cases=RESULTS,
    )
    try:
        grid = {}
        for fs, hz, ratio in product(RATES, NOTES, RATIOS):
            grid[fs, hz, ratio] = static_case(fs, hz, ratio)
        for fs, hz in product(RATES, (110, 329.63)):
            static_case(fs, hz, 1.8)
        lo, hi = [grid[48000, 440, r] for r in (1.2, 2.4)]
        assert abs(hi["normal_force"] / lo["normal_force"] - 2) <= 1e-6
        assert hi["band_1500_6000"] - lo["band_1500_6000"] >= 0.01
        slow, fast = [static_case(48000, 440, 1.8, v) for v in (0.06, 0.16)]
        assert fast["pickup"]["rms"] / slow["pickup"]["rms"] >= 1.8
        for hz in [65.41, 82.41, 440]:
            decay(48000, hz)
        stationarity(48000)
        for kind, hz in [("bend", 196), ("vibrato", 196), ("vibrato", 440)]:
            dynamic(48000, hz, kind)
        manifest["passed"] = True
    finally:
        (OUT / "manifest.json").write_text(
            json.dumps(manifest, indent=2, allow_nan=False) + "\n"
        )
    print(len(RESULTS), "actual Rust renders passed")
