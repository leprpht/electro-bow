# ElectroBow integrated effect

Branch: `feature/effect-refinement`. This report supersedes earlier stage reports
where behavior or measurements differ. The JUCE VST3 interface, two visible
controls, plugin identifier, and version-1/version-2 session compatibility remain.

## Architecture

Input audio → four parallel complementary IIR bands → Attack → Bow Character →
band recombination → stereo-linked sample-peak protection → output.

All DSP, including final output protection, resides in `rust/dsp/src/`.
`eb_filter_bank_process` is the integrated C ABI entry point. C++ only owns the
handle, snapshots controls, and divides oversized host buffers into preallocated
chunks. No pitch detection, MIDI, voice separation, synthesized string voices,
or STK Bowed is involved. The input remains the source of musical content.

The first-order complementary crossover design remains at 200/800/3000 Hz.
Keeping its broad overlap avoids adding higher-order phase rotation or delay.
The existing independent complex-response and C++-baseline parity tests continue
to verify reconstruction. No evidence from these tests justified retuning the
boundaries; this is a retained design, not a claim of perceptual optimality.

At both controls' minimum and below the protection ceiling, the band sum
reproduces the original input within floating-point rounding. Active per-band
processing deliberately changes magnitude and phase. Stereo control is linked;
identical or opposite-polarity stereo signals retain that relationship. There is
no claim of linear-phase behavior with nonzero controls.

## Final controls

| Control | Range | Behavior |
| --- | --- | --- |
| Attack | 0–100% (legacy storage 0–500) | Sharp original onset to progressively slower spectral transient response. The slow envelope spans 5–150 ms; this is not a literal onset delay. |
| Bow Character | 0–100% | Attack-only smooth sound to dynamic middle-band body, mild nonlinear texture, input-multiplied friction, and irregular spectral movement. |

Attack preserves low-band content and reduces upper-band transient excess using
level-relative envelope control. Bow Character follows the Attack-processed
bands. Its maximum middle-band emphasis has been reduced from about 20%/14% to
12%/8% (plus small movement) to limit buildup and avoid counteracting Attack's
softening. Nonlinearity remains weak and band-selective. Neither stage attempts
to synthesize sustain after the guitar decays.

Attack caches its slow-envelope coefficient while the control is unchanged,
avoiding repeated exponentials on small blocks. Its existing gain smoothing and
Character's 20 ms parameter smoothing remain. Processing crosses the C ABI once
per chunk; all stages execute in Rust with fixed state and preallocated bands.

## Output protection

The final Rust stage computes a shared stereo gain for each frame. Above a
**0.98 sample-peak ceiling**, it reduces gain immediately and recovers toward
unity with a 50 ms time constant. It never boosts. Below the ceiling, without a
recent overload, it is transparent. Overload reduction intentionally changes
dynamics; exact level scaling is promised only outside protection activity.

This prevents sample clipping without lookahead or independent-channel image
shifts. It is not a true-peak limiter: intersample peaks remain possible, and
instantaneous overload reduction can introduce distortion. Keep sensible input
headroom; extreme input should not be treated as a desired sound mode.
Non-finite input samples are replaced by zero and extreme finite input is
clamped to ±16 FS before it can overflow float band storage or poison IIR state;
final non-finite output is also suppressed. Finite overrange audio is handled by
the linked gain stage. No extra audio delay buffer is introduced.

The host reports **0 samples processing latency**. With processing enabled it
reports a conservative **100 ms tail allowance** for exposed IIR decay; this
is not latency or synthesized resonance. Neutral settings report zero tail.

## Legacy dependency audit

Production source consists of the JUCE processor/editor, thin Rust adapter, UI
assets, and Rust library. No production include or link requires aubio/STK or
the old pitch/bowed-voice classes. Their CMake configuration and compilation are
now guarded by `BUILD_TESTING`; old component tests still require them, so their
source and vendored licenses are retained. A separate tests-off Release build
is configured with nonexistent aubio/STK paths to verify the plugin does not
require them. They were not deleted merely because the audible path changed.

## Automated final test material

Rust complete-path tests cover generated single notes, open-chord frequency sets,
dense chords, sequential arpeggio tones, alternate picking, tremolo picking,
sustain and quiet/loud dynamics. These are synthetic fixtures, not recordings
or a claim of listening validation. Tested rates are 8, 22.05, 44.1, 48, 96 and
192 kHz, mono/stereo, callback lengths 1/17/64/257, and five combinations of
minimum/intermediate/maximum controls. Earlier filter tests also cover 4–384 kHz,
empty blocks, and host callbacks larger than the prepared capacity.

Additional tests cover:

- 10+ seconds of continuous stereo with repeated control changes;
- overload, NaN/Infinity input, finite recovery and peak bounds;
- linked protection preserving channel ratio at neutral settings;
- opposite-polarity stereo without cancellation in envelope detection;
- silence, reset, sample-rate repreparation, gain scaling below protection;
- all original complex band responses, zero-delay impulse and reconstruction;
- a real-time allocation counter including the complete protected path;
- state migration and actual embedded editor open/reopen.

Reproduce:

```sh
cargo test --locked --offline
cargo clippy --all-targets --locked --offline -- -D warnings
cmake --build build --config Release --parallel 8
ctest --test-dir build --build-config Release --output-on-failure
ctest --test-dir build --build-config Release -R ElectroBowFilterBankTests --verbose
```

## FL Studio status: pending

FL Studio 20 is installed locally. macOS reports UI automation disabled, so no
reliable insert/playback/stress session could be executed. The user explicitly
requested leaving this validation pending. **Automated tests and successful
builds do not establish real-time stability in FL Studio.**

Manual acceptance checklist: scan the final VST3; insert on an audio track;
play single notes, open/dense chords, arpeggios, alternate/tremolo picking and
long sustains; sweep both controls; test 44.1/48/96 kHz and 32/64/256/1024-frame
buffers; test mono/stereo routing; save/reopen a project; bypass/re-enable;
check audible clicks, clipping, dropouts and host CPU over a sustained session.
Do not mark host validation complete without recording these observations.

## Final measurements and artifacts

Measured locally on Apple M2 Pro, macOS arm64 Release. Stereo 48 kHz with both
controls at maximum, median of seven batches, full protected signal path:

| Block frames | One-core CPU | Time/stereo frame |
| ---: | ---: | ---: |
| 16 | 0.2200% | 45.83 ns |
| 64 | 0.2143% | 44.64 ns |
| 256 | 0.2126% | 44.28 ns |
| 1024 | 0.2132% | 44.41 ns |

This includes processor dispatch and checksum overhead, excluding host/device
scheduling and GUI. It is not a worst-case scheduling guarantee. Output protection
adds cost compared with the previous unprotected chain; coefficient caching
reduces avoidable work rather than promising an overall speedup.

At neutral controls, existing measurements remain: peak reconstruction error
`5.96046448e-8 FS`, worst noise null approximately `-153.25 dBr`, summed response
deviation below `1.86e-7 dB`, and phase deviation below `1.40e-6 degrees`.
The direct C++/Rust filter reference comparison reports zero sample difference
on this machine. These apply below protection activity, not during limiting.

Final validation passed: **13 Rust tests**, Clippy with warnings as errors,
**all four CTest suites**, native macOS Release, and a separate **universal
arm64/x86_64 Release VST3**. Strict bundle signature verification passed.
Windows/Linux builds were not rerun locally. FL Studio remains pending above.

Universal build reproduction (requires both Rust macOS targets):

```sh
cmake -S . -B build/final-release -DJUCE_PATH="$PWD/JUCE" \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF \
  '-DCMAKE_OSX_ARCHITECTURES=arm64;x86_64' \
  -DAUBIO_SRC_DIR=/nonexistent -DSTK_SRC_DIR=/nonexistent
cmake --build build/final-release --config Release --parallel 8
```

The two nonexistent legacy path entries intentionally go unused in production.
Artifact: `build/final-release/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3`.
Packaged artifact: `build/ElectroBow-final-macos.zip`. These are local ad-hoc
signed builds, not notarized distribution releases.

The existing comparison generator now uses the complete protected path:

```sh
cargo run -p electrobow-dsp --release --example character_comparison -- build/final-audio
```

It produces dry/smooth/textured/rough synthetic guitar-like WAV comparisons.
After the reduced boosts, smooth/textured/rough RMS levels are respectively
0.05128494 / 0.05260335 / 0.05392612 at the same recording gain. Listening on
actual electric-guitar recordings is still recommended before accepting timbre.
