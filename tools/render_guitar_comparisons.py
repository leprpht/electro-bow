"""Render dry/wet pairs through the built VST3. Optional: pip install pedalboard soundfile numpy."""
import argparse
from pathlib import Path

import numpy as np
import soundfile as sf
from pedalboard import load_plugin


RECORDINGS = Path(__file__).resolve().parents[1] / "ThirdParty/q/test/audio_files"


def excerpt(name, seconds, align=False):
    samples, rate = sf.read(RECORDINGS / (name + ".wav"), dtype="float32")
    start = 0
    if align:
        start = max(0, int(np.flatnonzero(np.abs(samples) > 0.025)[0]) - int(0.03 * rate))
    return samples[start:start + int(seconds * rate)].copy(), rate


def strum():
    # An open D voicing assembled from individual recorded strings, with
    # staggered attacks so the swell also encounters already-ringing strings.
    # Use distinct takes for D, A, D and F# (open, 12th, 12th and 24th fret).
    notes = ["3a-D", "2b-A-12th", "3b-D-12th", "-2c-F#-24th"]
    takes = [excerpt(note, 3.0, align=True) for note in notes]
    rate = takes[0][1]
    assert all(sr == rate for _, sr in takes)
    spacing = round(rate * 0.023)
    chord = np.zeros(rate * 3 + spacing * (len(takes) - 1), dtype="float32")
    for i, (take, _) in enumerate(takes):
        start = spacing * i
        chord[start:start + len(take)] += 0.35 * take
    return chord, rate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("plugin", type=Path)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    cases = [
        ("low-E-swell", excerpt("1a-Low-E", 4.0, align=True),
         dict(attack_ms=240, bow_character=0.68, spectral_movement=0.85)),
        ("staccato", excerpt("GStaccato", 5.0),
         dict(attack_ms=35, bow_character=0.72, noise=0.3, saturation=0.65)),
        ("layered-D-chord", strum(),
         dict(attack_ms=115, bow_character=0.8, saturation=0.7, spectral_movement=0.85)),
        ("riff", excerpt("riff_0_0", 5.0, align=True),
         dict(attack_ms=22, bow_character=0.85, noise=0.55, spectral_movement=1.0)),
    ]
    for name, (dry, rate), settings in cases:
        # Taper the edit and include silence for the body decay. Identical
        # input gain on both files preserves level differences for audition.
        fade = min(len(dry), int(rate * 0.015))
        dry[-fade:] *= np.linspace(1.0, 0.0, fade)
        dry = np.pad(dry, (0, rate))
        effect = load_plugin(str(args.plugin.resolve()))
        for parameter, value in settings.items():
            setattr(effect, parameter, value)
        wet = effect.process(dry, rate, buffer_size=127)
        assert dry.shape == wet.shape and np.isfinite(wet).all() and np.max(np.abs(wet)) <= 0.980001
        for label, audio in [("dry", dry), ("wet", wet)]:
            sf.write(args.out / f"{name}-{label}.wav", audio, rate, subtype="PCM_24")


if __name__ == "__main__":
    main()
