# Continuous attack processing

Implemented on `feature/attack-shaping`. DSP lives in `rust/dsp/src/attack.rs`;
JUCE invokes it between filter-bank split and recombination. The existing Attack
slider is active and displays 0–100% smoothing. Saved `attackMs` values retain
their 0–500 storage range; this is a response control, not a literal onset delay.
Bow Character and the other bow controls remain unimplemented/inactive.

## Algorithm selection

A global amplitude ramp would reduce the entire onset, including the fundamental,
and require a trigger or restart policy for fast playing. A fixed spectral
low-pass would also remove sustained brightness. The selected approach combines
continuous spectral-envelope control with smoothed transient-sensitive gains:

1. For each band, take the maximum absolute sample across the channels. A fast
   peak follower has instantaneous rise and a 3 ms decay constant.
2. A slower reference follows that envelope with a 5–150 ms time constant,
   selected by Attack. Positive relative excess is
   `clamp(1 - slow / fast, 0, 1)`; silence uses zero excess.
3. Smooth the requested amount and each band gain with a 1 ms time constant.
   Gain is `1 - 0.75 * bandWeight * amount * excess`, with weights
   `[0, 0.15, 0.65, 1]`. The low band is untouched; the high band's floor is 0.25.
4. Apply the same gain to both channels in each band, then recombine.

Minimum gives the sharp original response. Intermediate settings soften upper
transients moderately. Maximum produces stronger, longer spectral softening.
The envelopes run continuously: no pitch, note boundaries, articulation labels,
threshold-triggered events, lookahead, or future samples are used. Repeated
picking changes the current envelopes directly; there is no note admission or
retrigger lockout. Envelope history and gain history persist across blocks.

This is spectral attack shaping, not full-band fade-in. It deliberately changes
attack timbre and peak energy. Once fast and slow envelopes agree, gain returns
to unity. The detector is level-relative, so scaled input produces scaled output
apart from floating-point error and a negligible `1e-30` silence floor. It does
not normalize quiet playing up to loud playing. Stereo linking avoids independent
left/right gain pumping. Unequal band gains alter the crossover sum's phase;
perfect original reconstruction is only promised at minimum Attack.

## Validation

`cargo test --locked --offline -- --nocapture` covers:

- Single pluck, sustained chord, fast picking at approximately 16.7 Hz, 12 Hz
  tremolo, and a quiet-to-loud change, at minimum/intermediate/maximum Attack.
- Tenfold input scaling preserved within `1e-7` absolute sample error.
- Matching output for 17- and 257-frame block partitions.
- Monotonic onset softening from minimum through intermediate to maximum.
- A sustained 6 kHz test: maximum Attack onset energy ratio **0.175034** over
  samples 48–959; late sustained energy ratio **0.993536** over samples
  40000–47999, relative to minimum Attack at 48 kHz.
- The existing allocator test now includes maximum Attack during 1,000 stereo
  blocks: zero heap allocations, reallocations, or frees in processing/reset.

C++ integration tests exercise the actual Attack parameter through JUCE and the
Rust ABI at 8, 44.1, 48, 96, and 192 kHz with oversized 257-frame callbacks against
a prepared capacity of 64. They verify audible-path change, finite output, and
preservation of a 2:1 stereo channel ratio. Existing reconstruction, baseline
parity, and zero-latency tests explicitly select minimum Attack.

No delay buffer was introduced: reported and algorithmic buffering latency are
**0 samples**. The 1 ms gain smoothing is a control response, not an audio delay.
Without lookahead, the earliest samples of a pick cannot be anticipated; this
processor reduces the following spectral transient rather than guaranteeing
complete removal of every initial click.

Tests use synthesized signals. Recorded-guitar listening comparisons remain
necessary to tune the subjective amount, especially for very dense repeated
attacks. This design does not claim to preserve unmodified peaks while also
removing their pick transient.

Build and reproduce measurements:

```sh
cmake --build build --config Release --parallel 8
ctest --test-dir build --build-config Release --output-on-failure
ctest --test-dir build --build-config Release -R ElectroBowFilterBankTests --verbose
cargo clippy --all-targets --locked --offline -- -D warnings
```

The integration benchmark runs stereo at 48 kHz with maximum Attack enabled,
including filter splitting, shaping, recombination and processor dispatch. CPU
is a median-of-seven local microbenchmark, not a host scheduling guarantee.

Measured on Apple M2 Pro, macOS arm64 Release:

| Block frames | One-core CPU | Time/stereo frame |
| ---: | ---: | ---: |
| 16 | 0.1145% | 23.85 ns |
| 64 | 0.1100% | 22.91 ns |
| 256 | 0.1088% | 22.67 ns |
| 1024 | 0.1095% | 22.82 ns |

The macOS arm64 Release VST3 builds successfully, including final signature
verification. Cross-platform CI has not been executed locally for this change.
