"""Offline, injected-pitch comparison. No live pitch or note-state implementation."""

import json
import subprocess
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

import numpy as np
from acoustic_observation import (
    OUT,
    RAW,
    bands,
    db,
    digest,
    partials,
    pickup,
    rms,
    tracking,
)
from guide_qualification import trajectory
from scipy import signal
from scipy.io import wavfile

FS = 48000
Q = Path("ThirdParty/q/test/audio_files")
FIXTURES = Path(__file__).parent / "fixtures/acoustic"
KNOTS = [120, 450, 1400, 3500]


def audio(path):
    fs, x = wavfile.read(path)
    if np.issubdtype(x.dtype, np.integer):
        x = x.astype(float) / (-np.iinfo(x.dtype).min)
    return fs, x.astype(float)


def analysis_stream(x, fs):
    d = int(fs // 16000)
    fa = fs / d
    c = np.exp(-2 * np.pi * 30 / fs)
    hp = signal.lfilter([c, -c], [1, -c], x)
    fir = signal.firwin(32 * d + 1, 0.34 * fa, fs=fs, window="blackman")
    return signal.lfilter(fir, [1], hp)[d - 1 :: d], fa


def feature(window, fa, f, noise_floor=0.00032, broad=False):
    # Frozen pitch, four complete periods; relative rather than absolute levels.
    count = min(12, int(0.28 * fa / f))
    a = abs(partials(window, fa, f, count))
    k = np.arange(1, count + 1)
    eps = max(3 * noise_floor, 1e-5, 0.015 * max(a))
    reliability = np.clip((a - eps) / (3 * eps), 0, 1)
    if np.count_nonzero(reliability) < 2:
        return None
    levels = db(k * np.maximum(eps, a))
    centered = levels - np.average(levels, weights=reliability)
    if not broad:
        gains = np.zeros(12)
        gains[:count] = np.clip(0.45 * centered * reliability, -5, 5)
        return gains
    hat = np.array(
        [np.interp(np.log2(k * f), np.log2(KNOTS), row) for row in np.eye(4)]
    )
    weights = hat * reliability
    return np.clip(0.45 * (weights @ centered) / (1 + np.sum(weights, axis=1)), -4, 4)


def capture(x, fs, f, stop, start=0.025, broad=False):
    z, fa = analysis_stream(x, fs)
    length = int(4 * fa / f + 0.5)
    first = max(length, int(start * fa) + length)
    held = np.zeros(4 if broad else 12)
    valid = 0
    hop = int(0.005 * fa + 0.5)
    for end in range(
        ((first + hop - 1) // hop) * hop, min(len(z), int(stop * fa)) + 1, hop
    ):
        target = feature(z[end - length : end], fa, f, broad=broad)
        if target is not None:
            held += (1 - np.exp(-hop / (fa * 0.040))) * (target - held)
            valid += 1
    return held, valid


def render(hz, speeds, ratios, lift=None, return_m5=False):
    f = np.full(len(speeds), hz)
    k = ratios * speeds / 1.65
    if lift is not None:
        k[lift:] = 0
    raw = subprocess.run(
        [str(RAW / "guide-render"), str(FS)],
        input=np.stack([f, speeds, k], axis=1).astype("<f8").tobytes(),
        capture_output=True,
        check=True,
    )
    trace = np.frombuffer(raw.stdout, dtype="<f8").reshape(-1, 12)
    if return_m5:
        return trace[:, 11].copy()
    return pickup(trace, FS, f, speeds[0]) / np.sqrt(2)


def resonant_bands(x, fs, f, fa):
    # Fixed pitch reference to the specified two cascaded contractive complex rotations.
    radius = np.exp(-2 * np.pi * 0.22 * f / fs)
    bank = []
    for k in range(1, 13):
        pole = radius * np.exp(2j * np.pi * k * f / fs)
        z = signal.lfilter([1 - radius], [1, -pole], x)
        z = signal.lfilter([1 - radius], [1, -pole], z)
        taper = np.clip((0.28 * fa - k * f) / (0.03 * fa), 0, 1)
        bank.append(2 * taper * z.real)
    return np.array(bank)


def moving_carrier(kind, db_gains):
    """Keep a measured source feature fixed while the actual N trace retunes."""
    raw = RAW / f"{kind}_48000_196_R1.8_V0.1_3s.f64le"
    trace = np.memmap(raw, dtype="<f8", mode="r").reshape(-1, 12)
    t = np.arange(len(trace)) / FS
    f = 196 * np.exp2(trajectory(t, kind) / 1200)
    q = pickup(trace, FS, f, 0.1) / np.sqrt(2)
    k = np.arange(1, 13)
    gains = 10 ** (np.array(db_gains) / 20) - 1
    z1 = np.zeros(12, complex)
    z2 = z1.copy()
    output = np.empty_like(q)
    for n, sample in enumerate(q):
        rho = np.exp(-2 * np.pi * 0.22 * f[n] / FS)
        freq = k * f[n]
        pole = rho * np.exp(2j * np.pi * freq / FS)
        z1 = pole * z1 + (1 - rho) * sample
        z2 = pole * z2 + (1 - rho) * z1
        taper = np.clip((0.28 * 16000 - freq) / (0.03 * 16000), 0, 1)
        output[n] = sample + np.dot(gains * taper, 2 * z2.real)
    delivered = output[: 3 * FS]
    if not np.isfinite(delivered).all():
        raise ValueError("nonfinite retuned identity transfer")

    return {
        "peak_FS": max(abs(delivered)),
        "rms_FS": rms(delivered),
        "correction_rms_FS": rms(delivered - q[: len(delivered)]),
        "tracking": tracking(output, FS, 196, kind),
    }


def distances(a, b):
    # Centering removes the overall level, with a common relative floor for weak orders.
    def relative(h):
        v = np.maximum(db(abs(h) / max(abs(h))), -50)
        return v - v.mean()

    return rms(relative(a) - relative(b))


def save_pair(name, clips, seed, anchor):
    order = np.random.default_rng(seed).permutation(len(clips))
    table = {}
    gains = [0.10 / rms(x[anchor]) for _, x in clips]
    headroom = min(1.0, 0.9 / max(g * max(abs(x)) for g, (_, x) in zip(gains, clips)))
    for i, index in enumerate(order):
        label, x = clips[index]
        gain = gains[index] * headroom
        filename = f"{name}-{chr(65 + i)}.wav"
        listening = x * gain
        if name != "attack":
            edge = (1 - np.cos(np.linspace(0, np.pi, FS // 200))) / 2
            listening[: len(edge)] *= edge
            listening[-len(edge) :] *= edge[::-1]
        wavfile.write(OUT / filename, FS, listening.astype("f4"))
        wavfile.write(OUT / f"engineering-{name}-{label}.wav", FS, x.astype("f4"))
        table[filename] = {"source": label, "gain": gain}
    return table


def reference_check():
    # One calibration second, two withheld seconds of the same performance.
    trace = np.memmap(
        RAW / "steady_48000_82.41_R1.8_V0.1_3s.f64le", mode="r", dtype="<f8"
    ).reshape(-1, 12)
    orders = np.arange(1, 7)
    arm = abs(partials(trace[-FS:, 3], FS, 82.40347, 6))
    outputs = [audio(OUT / f"82.41-{key}.wav")[1] for key in ("M5", "S5", "pickup")]
    responses = [abs(partials(y[-FS:], FS, 82.40347, 6)) for y in outputs]
    scores = {}
    for t in (8, 10, 22):
        rate, window = audio(FIXTURES / f"vaisvil-{t}.wav")
        mag = abs(np.fft.rfft(window * np.hanning(rate), 8 * rate))
        f = (np.argmax(mag[80 * 8 : 85 * 8]) + 80 * 8) / 8
        a = np.array(
            [
                np.linalg.norm(mag[int(8 * (k * f - 2)) : int(8 * (k * f + 2)) + 1])
                for k in orders
            ]
        )
        target = db(a / a[0])

        def error(v, target=target):
            return rms((db(v / v[0]) - target)[1:])

        # Estimate effective position from low orders in the ideal fixed-string limit.
        err, s = min(
            (error(arm * abs(np.sin(np.pi * orders * s))), s)
            for s in np.arange(0.145, 0.221, 0.001)
        )
        scores[t] = {
            "f": f,
            "target": target.tolist(),
            "best_position": s,
            "fit_error": err,
            "actual_error_M5_S5_pickup": [error(h) for h in responses],
        }
    return scores


def main():
    results = {}
    for line in (FIXTURES / "SHA256SUMS").read_text().splitlines():
        expected, filename = line.split("  ", 1)
        if digest(Path(filename)) != expected:
            raise ValueError(f"fixture hash mismatch for {filename}")
    files = [
        "1a-Low-E.wav",
        "gk Low E.wav",
        "4a-G.wav",
        "1c-Low-E-24th.wav",
        "6a-High-E.wav",
        "6c-High-E-24th.wav",
        "Bend-Slide G.wav",
        "Attack-Reset.wav",
        "GStaccato.wav",
        "Hammer-Pull High E.wav",
    ]
    for name in files:
        path = Q / name
        rate, x = audio(path)
        results[name] = {
            "sha256": digest(path),
            "rate": rate,
            "seconds": len(x) / rate,
            "peak": np.max(abs(x)),
        }
    results["geometry_calibration"] = reference_check()
    rate, excerpt = audio(FIXTURES / "vaisvil-8.wav")
    fgrid = np.arange(80, 86, 0.02)
    f = fgrid[np.argmax([abs(partials(excerpt, rate, f, 1)[0]) for f in fgrid])]
    results["reference_8_to_9s"] = {
        "hz": f,
        "peak": np.max(abs(excerpt)),
        "rms": rms(excerpt),
        "harmonics_dBr": db(
            abs(partials(excerpt, rate, f)) / abs(partials(excerpt, rate, f, 1)[0])
        ).tolist(),
    }
    for path in sorted(FIXTURES.glob("*.wav")):
        rate, x = audio(path)
        results[path.name] = {
            "sha256": digest(path),
            "rate": rate,
            "seconds": len(x) / rate,
            "peak": np.max(abs(x)),
        }
    gate = {}
    raw = audio(OUT / "196-M5.wav")[1][:FS]
    selected = audio(OUT / "196-pickup.wav")[1][:FS] / np.sqrt(2)
    gate.update(
        save_pair("timbre", [("M5", raw), ("pickup", selected)], 12, slice(FS // 5, FS))
    )
    time = np.arange(FS) / FS
    sources = [
        ("strings", files[3:5], 329.63),
        ("lowE", files[:2], 82.41),
        ("synthetic", ["A", "B"], 196),
    ]
    sources.extend(
        (f"techs-{t}", [f"techs-P{i}-{t}.wav" for i in (1, 2)], 196)
        for t in (60, 132, 204)
    )
    techs_spectra = []
    for label, notes, f in sources:
        wet = render(f, np.full(FS, 0.1), np.full(FS, 1.8))
        broad_bank = bands(wet, FS)
        broad_spectra = []
        old_outputs = []
        trials = []
        stimuli = []
        within = []
        direct_refs = []
        for idx, name in enumerate(notes):
            if label == "synthetic":
                rate = FS
                x = sum(
                    (1 if k <= 2 else [0.2, 1.6][idx])
                    * np.cos(2 * np.pi * k * f * time)
                    / k
                    for k in range(1, 13)
                )
            else:
                rate, x = audio((FIXTURES if label.startswith("techs-") else Q) / name)
                event = np.flatnonzero(abs(x) > 0.01)[0]
                x = x[event:]
            x = x * (0.08 / rms(x[int(0.06 * rate) : int(0.22 * rate)]))
            old_outputs.append(
                render(f, np.full(FS, 0.1), np.full(FS, 1.8), return_m5=True)
            )
            broad_gain, _ = capture(x, rate, f, 0.30, broad=True)
            broad_wet = 10 ** (broad_gain / 20) @ broad_bank
            broad_spectra.append(partials(broad_wet[FS // 2 :], FS, f))
            bank = resonant_bands(wet, FS, f, FS / (FS // 16000))
            observed = []
            for stop in (0.16, 0.22, 0.30):
                gain, count = capture(x, rate, f, stop)
                observed.append((gain.tolist(), count))
            gains = np.array(observed[-1][0])
            out = wet + (10 ** (gains / 20) - 1) @ bank
            src = partials(x[int(0.08 * rate) : int(0.23 * rate)], rate, f)
            sustained = partials(out[FS // 2 :], FS, f)
            trials.append((observed, src, sustained))
            earlier = wet + (10 ** (np.array(observed[0][0]) / 20) - 1) @ bank
            within.append(distances(partials(earlier[FS // 2 :], FS, f), sustained))
            stimuli.append((str(idx + 1), out))
            direct_refs.append(
                signal.resample_poly(
                    x[: int(0.4 * rate)],
                    FS // np.gcd(rate, FS),
                    rate // np.gcd(rate, FS),
                )
            )
        if not np.array_equal(*old_outputs):
            raise ValueError("equal controls changed M5")
        a, b = trials

        def difference(h1, h2):
            vectors = [np.maximum(-50, db(abs(h) / max(abs(h)))) for h in (h1, h2)]
            return vectors[0] - vectors[1]

        delta_in = difference(a[1], b[1])
        delta_out = difference(a[2], b[2])

        def notch_delta(values, k):
            return values[k - 1] - (values[k - 2] + values[k]) / 2

        results[label] = {
            "contrast_correlation": np.corrcoef(delta_in, delta_out)[0, 1],
            "h6_notch_contrast_dB": [notch_delta(d, 6) for d in (delta_in, delta_out)],
            "hz": f,
            "captures": [a[0], b[0]],
            "input_distance_dB": distances(a[1], b[1]),
            "M5_equal_controls_dB": distances(
                *[partials(old, FS, f) for old in old_outputs]
            ),
            "broad_ablation_dB": distances(*broad_spectra),
            "held_distance_dB": distances(a[2], b[2]),
            "within_take_wet_distance_dB": within,
            "within_take_gain_rms_dB": [
                rms(np.array(t[0][0][0]) - t[0][-1][0]) for t in trials
            ],
        }
        if label.startswith("techs-"):
            techs_spectra.append([t[2] for t in trials])
        if label == "techs-60":
            gate.update(
                save_pair(
                    "identity",
                    [(name, x[FS // 3 :]) for name, x in stimuli],
                    11,
                    slice(FS // 6, None),
                )
            )
            for idx, refsignal in enumerate(direct_refs):
                wavfile.write(
                    OUT / f"identity-reference-{idx + 1}.wav",
                    FS,
                    (refsignal * 0.1 / rms(refsignal)).astype("f4"),
                )
    # Across separately played G3 events (different string/fret contexts).
    results["techs_within_player_repeats"] = [
        [
            distances(techs_spectra[a][i], techs_spectra[b][i])
            for a, b in [(0, 1), (0, 2), (1, 2)]
        ]
        for i in (0, 1)
    ]
    for kind in ("bend", "vibrato"):
        results["frozen_identity_" + kind] = moving_carrier(
            kind, results["techs-60"]["captures"][0][-1][0]
        )
    # Floor and notch discrimination using the same causal estimator.
    fa = 16000
    hz = 200
    t = np.arange(round(4 * fa / hz)) / fa

    def notch(sixth):
        x = sum(
            0.15 * (sixth if k == 6 else 1) * np.cos(2 * np.pi * k * hz * t) / k
            for k in range(1, 13)
        )
        return feature(x, fa, hz)

    results["sixth_gain_normal_supported_dip_missing_dB"] = [
        notch(a)[5] for a in (1, 0.35, 0)
    ]
    if (
        feature(np.zeros_like(t), fa, hz) is not None
        or feature(np.cos(2 * np.pi * hz * t) * 1e-5, fa, hz) is not None
    ):
        raise ValueError("unsupported source feature refreshed")
    # Isolate the gesture. Both sides have the same acoustic transfer, E, final
    # speed and ratio. Lift at 0.7 s and apply the same 100 ms release.
    duration = 0.020 + 0.060 * (1 - 0.4)
    u = np.minimum(time / duration, 1)
    g = u * u * (3 - 2 * u)
    envelope = np.minimum(time / 0.050, 1) * np.clip((0.8 - time) / 0.1, 0, 1)
    base = render(196, np.full(FS, 0.1), np.full(FS, 1.8), int(0.7 * FS))
    gest = render(196, 0.06 + 0.04 * g, 1.2 + 0.6 * g, int(0.7 * FS))
    clips = [("level", envelope * base), ("gesture", envelope * gest)]
    gate.update(save_pair("attack", clips, 41, slice(FS // 4, FS // 2)))
    for name, x in clips:
        results["attack_" + name] = [
            db(abs(h) / abs(h[0])).tolist()
            for i in (0, 0.03, 0.40, 0.71)
            for h in [partials(x[int(i * FS) : int((i + 0.025) * FS)], FS, 196, 8)]
        ]
    (OUT / "identity.json").write_text(
        json.dumps(results, indent=2, allow_nan=False) + "\n"
    )
    (OUT / "listening-key.json").write_text(json.dumps(gate, indent=2) + "\n")
    with ZipFile(OUT / "listening-comparisons.zip", "w", ZIP_DEFLATED) as bundle:
        for filename in [*gate, "identity-reference-1.wav", "identity-reference-2.wav"]:
            bundle.write(OUT / filename, filename)
        bundle.write(FIXTURES / "vaisvil-8.wav", "bowed-reference.wav")
        bundle.write(FIXTURES / "ATTRIBUTION.md", "Recording-credits.md")
        bundle.writestr(
            "Listen.txt",
            """Keep playback volume fixed. Three short comparisons.

1. Play bowed-reference, then timbre A/B. Which sounds more like a bowed electric
guitar? Which buzzes? Flag any objectionable noise or resonance.
2. Hear identity-reference 1 and 2, then held identity A/B. Match each held tone
to its source, and name the cue. Both are the same pitch with identical bow controls.
3. Play attack A/B. Which onset sounds more naturally bowed? Is the release usable?

Indistinguishable and neither are valid answers. Record the playback route,
date and any timestamped fault before checking the separate listening key.
The WAVs are listening copies. The physical reference is an excerpt by Vaisvil.
The two DI references are level-matched excerpts from Guitar-TECHS by Pedroza,
Abreu, Corey and Roman. See Recording-credits.md for sources and licenses.
""",
        )
    print("generated three level-matched comparisons and original-gain stems")


if __name__ == "__main__":
    main()
