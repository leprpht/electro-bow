"""Stage 04 offline observation of the actual Stage 03 Rust telemetry (repository root)."""

import json
from pathlib import Path

import numpy as np
from guide_qualification import OUT as RAW
from guide_qualification import digest, trajectory
from scipy import signal
from scipy.io import wavfile

OUT = Path("build/stage-04")
BETA = 0.127
CENTERS = [(102, 193), (315, 650), (1060, 1780), (2820, 4100)]
STATIONS = [0.162, 0.174, 0.186]


def rms(x):
    return float(np.sqrt(np.mean(x * x)))


def db(x):
    return 20 * np.log10(np.maximum(x, 1e-12))


def bands(x, fs):
    lows = []
    for f in (200, 800, 3000):
        g = np.tan(np.pi * f / fs)
        lows.append(signal.lfilter([g / (1 + g)] * 2, [1, (g - 1) / (g + 1)], x))
    return np.stack([lows[0], lows[1] - lows[0], lows[2] - lows[1], x - lows[2]])


def legacy(x, fs):
    summed = np.zeros_like(x)
    for band, pair, q, weight in zip(
        bands(x, fs), CENTERS, [1.6, 2.6, 4.5, 5.5], [1.6, 2, 2.25, 1.75]
    ):
        for f, mix in zip(pair, [0.55, 0.45]):
            w = 2 * np.pi * f / fs
            a = np.sin(w) / (2 * q)
            mode = signal.lfilter([a, 0, -a], [1 + a, -2 * np.cos(w), 1 - a], band)
            summed += weight * mix * mode
    return 0.85 * x + 0.15 * summed


def read_arm(history, delay, seed_period, seed_speed):
    """Eager evaluation independent of Rust's wrapped index and saturated count."""
    n = np.arange(len(history))
    lag = np.floor(delay).astype(int)
    u = delay - lag

    def sample(index):
        phase = index / seed_period + 1 - BETA
        seed = -(seed_speed / BETA) * (phase - np.floor(phase) - 0.5)
        return np.where(index < 0, seed, history[np.maximum(0, index)])

    return (1 - u) * sample(n - lag) + u * sample(n - lag - 1)


def spatial_velocity(data, fs, hz, speed):
    """Stage 04 spatial research observation in m/s, before loading and conversion."""
    period = fs / hz

    def read(d):
        return read_arm(data[:, 3], d, period[0], speed)

    # Use the nut's rigid reflection. Both directions share the same time origin.
    velocity = np.zeros(len(data))
    for s, weight in zip(STATIONS, [1 / 6, 4 / 6, 1 / 6]):
        velocity += weight * (
            read((s - BETA) * period / 2) - read((2 - s - BETA) * period / 2)
        )
    # Check that the long tap at the bow reproduces the actual signed nut return.
    if np.max(abs(read((1 - BETA) * period) + data[:, 0])) > 1e-12:
        raise ValueError("trajectory, seed or nut-return alignment mismatch")
    return velocity


def pickup(data, fs, hz, speed, noise=0):
    velocity = spatial_velocity(data, fs, hz, speed)
    c = np.exp(-2 * np.pi * 20 / fs)
    raw = 0.85 * signal.lfilter([c, -c], [1, -c], velocity)
    loaded = signal.sosfilt(signal.butter(2, 3200, fs=fs, output="sos"), raw)
    return loaded + 0.025 * noise * (raw - loaded)


def partials(x, fs, f, count=12):
    w = np.hanning(len(x))
    t = np.arange(len(x)) / fs
    return np.array(
        [
            2 * np.dot(w * (x - x.mean()), np.exp(-2j * np.pi * k * f * t)) / w.sum()
            for k in range(1, count + 1)
            if k * f < fs / 2
        ]
    )


def stats(x, fs, f, data, kind):
    tail = x[-fs:]
    power = abs(np.fft.rfft((tail - tail.mean()) * np.hanning(fs), 8 * fs)) ** 2
    freq = np.arange(len(power)) / 8
    k = np.rint(freq / f)
    mask = (
        (k >= 1)
        & (k * f < fs / 2)
        & (abs(freq - k * f) <= np.maximum(2, 0.001 * k * f))
    )
    total = power[1:].sum()

    def harmonics(window):
        h = partials(window, fs, f)
        return {
            "peak_FS": abs(h).tolist(),
            "dBr": db(abs(h) / abs(h[0])).tolist(),
            "phase_deg": np.angle(
                h * np.exp(-1j * np.arange(1, len(h) + 1) * np.angle(h[0])), deg=True
            ).tolist(),
        }

    h = harmonics(tail)
    cycles = np.flatnonzero((data[1:, 10] != 0) & (data[:-1, 10] == 0)) + 1
    cycles = cycles[cycles >= len(x) - fs]
    warps = [
        np.interp(np.arange(128) * (b - a) / 128, np.arange(b - a + 1), x[a : b + 1])
        for a, b in zip(cycles[:-1], cycles[1:])  # noqa: RUF007 - Python 3.9 replay environment
    ]
    peak = np.max(abs(x))
    values = {
        "first": x[0],
        "rms10": rms(x[: fs // 100]),
        "rms50": rms(x[: fs // 20]),
        "ac10": x[: fs // 100].std(),
        "ac50": x[: fs // 20].std(),
        "rms_steady": rms(tail),
        "ac_steady": tail.std(),
        "peak": peak,
        "above_ceiling": int(np.sum(abs(x) > 0.98)),
        "required_reduction_db": max(0.0, db(peak / 0.98)),
        "clipped": int(np.sum(abs(x) >= 1)),
        "centroid_Hz": np.dot(freq, power) / total,
        "hf_fraction": power[freq >= 6000].sum() / total,
        "residual_fraction": power[~mask & (freq > 0)].sum() / total,
        "saw7_db": rms(np.array(h["dBr"][:7]) + db(np.arange(1, 8))),
        "tilt_db_oct": np.polyfit(np.log2(np.arange(1, 8)), h["dBr"][:7], 1)[0],
        "harmonics": h,
        "attack50": harmonics(x[: fs // 20]),
        "cycle_residual": rms(warps - np.mean(warps, axis=0))
        / np.std(np.mean(warps, axis=0))
        if warps
        else None,
    }
    if kind == "lift":
        values["lift_windows"] = [
            harmonics(x[n : n + fs // 20])
            for n in (fs - fs // 20, fs, fs + fs // 10, 2 * fs)
        ]
    return values


def case(data, e):
    fs, f, kind = e["rate"], e["hz"], e["trajectory"]
    n = fs * e["seconds"]
    data = data[:n]
    hz = f * np.exp2(trajectory(np.arange(n) / fs, kind) / 1200)
    raw = data[:, 11]
    selected = pickup(data, fs, hz, e["speed"])
    return {"M5": raw, "S5": legacy(raw, fs), "pickup": selected}


def projection_diagnostics(data, fs, f, seed_hz):
    orders = np.arange(1, 11)
    z = np.exp(-2j * np.pi * orders * f / fs)

    def tap(d):
        return z ** int(d) * (1 - d % 1 + (d % 1) * z)

    P = fs / seed_hz
    Hb = 0.98 * (0.125 + 0.75 * z + 0.125 * z * z) * tap(BETA * P - 1)
    Hn = tap((1 - BETA) * P)
    Hp = tap(BETA * P / 2)
    B, N, p, j = [partials(data[-fs:, col], fs, f, 10) for col in [2, 3, 5, 6]]
    return {
        "eighth_B_N_p_j_dBr": [float(db(abs(x[7] / x[0]))) for x in (B, N, p, j)],
        "nut_numerator_7_8_9_dB": db(abs(1 - Hn[6:9])).tolist(),
        "return_residual": rms(abs((1 - Hb * Hn) * B - (1 - Hn) * j)) / abs(B[0]),
        "pickup_residual": rms(abs(p - Hp * B)) / abs(p[0]),
    }


def tracking(x, fs, f, kind):
    """Auxiliary acoustic phase diagnostic. guide_qualification.dynamic owns V10/V11."""
    limits = (150, 290) if f == 196 else (335, 650)
    fund = signal.sosfiltfilt(
        signal.butter(3, limits, fs=fs, btype="bandpass", output="sos"), x
    )
    phase = np.unwrap(np.angle(signal.hilbert(fund)))
    l = round(0.006 * fs)
    t = np.arange(l, len(x) - l) / fs
    mask = (t >= 1) & (t < (1.45 if kind == "bend" else 3))
    out = 1200 * np.log2(
        (phase[2 * l :] - phase[: -2 * l])[mask] * fs / (2 * l * 2 * np.pi * f)
    )
    t = t[mask]
    truths = [trajectory(t - ms / 1000, kind) for ms in range(46)]
    lag = np.argmax([np.corrcoef(out, y)[0, 1] for y in truths])
    return {
        "lag_ms": int(lag),
        "aligned_cents": rms(out - truths[lag]),
        "unshifted_cents": rms(out - truths[0]),
    }


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    cases = json.loads((RAW / "manifest.json").read_text())["cases"]
    results = {}
    for name, e in cases.items():
        fs, f, kind = e["rate"], e["hz"], e["trajectory"]
        if not (
            (
                fs == 48000
                and (
                    f in (82.41, 196, 440)
                    and e["seconds"] <= 7
                    and (f == 440 or e["ratio"] == 1.8)
                )
                and kind != "driven"
            )
            or (f == 1318.51 and e["ratio"] == 1.8 and fs in (32000, 48000, 96000))
        ):
            continue
        path = RAW / e["raw"]
        if digest(path) != e["raw_sha256"]:
            raise ValueError(path)
        data = np.memmap(path, dtype="<f8", mode="r").reshape(-1, 12)[
            : fs * e["seconds"]
        ]
        outputs = case(data, e)
        f0 = cases[f"steady_{fs}_{f:g}_R{e['ratio']:g}_V{e['speed']:g}_3s"]["pickup"][
            "hz"
        ]
        results[name] = {
            key: stats(x, fs, f0, data, kind) for key, x in outputs.items()
        }
        if (
            kind == "steady"
            and fs == 48000
            and f == 440
            and e["ratio"] == 1.8
            and e["speed"] == 0.1
        ):
            results[name]["scattering"] = projection_diagnostics(data, fs, f0, f)
        if kind in ("bend", "vibrato"):
            padded = np.memmap(path, dtype="<f8", mode="r").reshape(-1, 12)
            pitch = f * np.exp2(trajectory(np.arange(len(padded)) / fs, kind) / 1200)
            for key, x in [
                ("M5", padded[:, 11]),
                ("pickup", pickup(padded, fs, pitch, e["speed"])),
            ]:
                results[name][key]["tracking"] = tracking(x, fs, f, kind)
        if fs == 96000 and f == 1318.51:
            for key, x in [("pickup", outputs["pickup"]), ("M5", outputs["M5"])]:
                down = signal.resample_poly(x, 1, 3, window=("kaiser", 10))[-32000:]
                # Match analysis bandwidth and bin widths with the native 32 kHz run.
                alt = stats(down, 32000, f0, data[-fs::3], kind)
                results[name][key]["decimated_32k"] = {
                    k: alt[k]
                    for k in ("hf_fraction", "residual_fraction", "centroid_Hz")
                }
        if (
            kind == "steady"
            and fs == 48000
            and f in (82.41, 196, 440)
            and e["ratio"] == 1.8
            and e["speed"] == 0.1
        ):
            for key, x in outputs.items():
                wavfile.write(OUT / f"{f:g}-{key}.wav", fs, x.astype("f4"))
    fs = 48000
    impulse = np.r_[1.0, np.zeros(fs - 1)]
    # Level and delay of the complex legacy parallel blend, including its direct path.
    response = np.fft.rfft(legacy(impulse, fs))
    orders = np.arange(1, 8) * 440
    results["legacy_response_db"] = db(abs(response[orders])).tolist()
    results["legacy_extrema_db"] = db(
        [min(abs(response[65:])), max(abs(response[65:]))]
    ).tolist()
    # Relative to the N-wave emission, valid away from zeros and at frozen f.
    omega = 2 * np.pi * np.arange(fs // 2 + 1) / fs
    z = np.exp(-1j * omega)

    def tap(d):
        return z ** int(d) * (1 - (d % 1) + (d % 1) * z)

    P = fs / 440
    spatial = sum(
        w * (tap((s - BETA) * P / 2) - tap((2 - s - BETA) * P / 2))
        for s, w in zip(STATIONS, [1 / 6, 4 / 6, 1 / 6])
    )
    c = np.exp(-2 * np.pi * 20 / fs)
    _, load = signal.sosfreqz(signal.butter(2, 3200, fs=fs, output="sos"), worN=omega)
    h = spatial * load * 0.85 * c * (1 - z) / (1 - c * z)
    results["pickup_group_delay_ms_at_orders_1_to_7"] = (
        -np.gradient(np.unwrap(np.angle(h)), omega)[orders] * 1000 / fs
    ).tolist()
    (OUT / "observation.json").write_text(
        json.dumps(results, indent=2, allow_nan=False) + "\n"
    )
    print("measured", len(results) - 3, "guide fixtures")


if __name__ == "__main__":
    main()
