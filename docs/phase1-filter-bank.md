# Phase 1 — four-band filter bank

Implemented on `feature/filter-bank`, 2026-11-15.

The DSP implementation was migrated to Rust on 2026-11-19. The topology and
measurements below are the original baseline; direct C++/Rust parity tests now
verify every band. See [Rust integration and validation](rust-dsp.md) for current
source ownership, build prerequisites, and migration results.

## Integration and scope

The previous processor mixed input to mono, ran polyphonic/pitch analysis, and
replaced both output channels with STK bowed-string synthesis. The active signal
path now splits the original input into four bands and sums those bands at unity
gain. Mono and stereo are preserved. No analysis or bow synthesis runs in this
path, and no new bow processing was added.

The existing analyzer, instrument classes, and their component tests remain in
the repository. The old processor test that required bowed synthesis was replaced
by filter-bank processor integration tests. Existing UI controls and version-1
session data are retained for compatibility; **bow controls currently have no
audio effect, and pitch/voice displays remain idle**. A new UI is outside Phase 1.

## Filter design

Three parallel first-order, bilinear-transform low-pass filters receive the same
input, with nominal cutoffs of 200, 800, and 3000 Hz. Define their outputs as
`L200(x)`, `L800(x)`, and `L3000(x)`:

| Index | Intended range | Output |
| --- | --- | --- |
| 0 | Low | `L200(x)` |
| 1 | Low-mid | `L800(x) - L200(x)` |
| 2 | High-mid | `L3000(x) - L800(x)` |
| 3 | High | `x - L3000(x)` |

Their transfer functions sum algebraically to 1. Thus reconstruction preserves
amplitude **and phase**, including the impulse's original position. There is no
dry bypass mixed into the output: recombination reads only the four band buffers.

For each cutoff `fc`, with `g = tan(pi * fc / sampleRate)`, the reference transfer
function is `L(z) = b * (1 + z^-1) / (1 + a * z^-1)`, where
`b = g / (1 + g)` and `a = (g - 1) / (g + 1)`. The implementation uses a
trapezoidal integrator with double coefficients/history and float band buffers.
Recombination accumulates in double before converting to float.

**Tradeoff:** these are broad, overlapping bands with gentle 6 dB/octave skirts
away from the bilinear warp near Nyquist. The two middle differences are IIR
band-pass responses. The specified frequencies are the low-pass design cutoffs,
not four isolated brick-wall band boundaries or exact equal-gain intersection
points. Individual bands have phase shift; cancellation removes it in the unity
sum. Unequal gains or future nonlinear processing will change that cancellation.
There is no steep Linkwitz–Riley crossover or all-pass phase compensation here.

At rates below approximately 6.67 kHz, all three cutoffs scale proportionally so
the highest remains at 45% of the sample rate. At the usual audio rates the
nominal cutoffs are unchanged. Preparation rejects nonpositive/nonfinite rates,
invalid channel counts, and invalid capacities.

## Audio-thread contract

- `prepare(rate, capacity, channels)` allocates band storage and resets histories
  off the audio thread. Each channel has three independent filter states.
- `split(inputPointers, count)` fills four independent channel buffers for a
  chunk no larger than the prepared capacity.
- `getBandData(band, channel)` provides writable samples for future band DSP.
  Only `getNumSamples()` samples are valid. Contents last until the next split;
  pointers last until preparation. Access is confined to the audio thread.
- `recombine(outputPointers)` sums the current bands. In-place host processing is
  safe because splitting consumes the input before recombination writes output.
- `processBlock()` chunks oversized host buffers, with continuous history and no
  buffer resize. Empty callbacks do no work. Channel layouts must be matched
  mono or stereo.
- `releaseResources()` resets histories and storage. Repreparation also resets
  them, including after sample-rate/channel-count changes.

Code inspection of the callback and split/recombine loops finds no allocation,
locks, I/O, or coefficient updates. All vector allocation occurs in preparation.
The processor uses `ScopedNoDenormals`; the bank also clears state magnitudes
below `1e-30` at chunk boundaries to prevent persistent denormal tails in
standalone use. That numerical floor is -600 dBFS. Memory for band samples is
`4 bands * 2 reserved channels * capacity * sizeof(float)` (8 KiB at 256 frames).

## Reconstruction measurements

Measured locally with Apple M2 Pro, macOS 26.5.1, Apple Clang 17.0.0, arm64,
CMake Release (`-O3`). Numbers below come from `ElectroBowFilterBankTests`.

Each rate uses a 32,768-sample impulse, evaluated at 129 logarithmic frequencies
from 10 Hz to 0.499 times the sample rate, plus DC, Nyquist, and all in-range
nominal crossover frequencies. Every band's **complex** response is also checked
against the independently evaluated transfer function. Noise uses 65,536 samples
per channel, seeded uniform noise in [-0.9, 0.9], and varying chunk lengths.

Noise null is `10 log10(sum((output-input)^2) / sum(input^2))`; no delay alignment
or gain correction is applied. Flatness and phase columns are the maximum
absolute deviations of the summed impulse response from unity and zero phase.

| Sample rate (Hz) | Noise peak error (FS) | Noise null (dBr) | Flatness error (dB) | Phase error (degrees) | Impulse peak error (FS) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 4,000 | 5.960e-8 | -153.290 | 1.687e-7 | 1.391e-6 | 1.490e-8 |
| 8,000 | 5.960e-8 | -153.251 | 1.767e-7 | 1.134e-6 | 1.118e-8 |
| 22,050 | 5.960e-8 | -153.852 | 1.091e-7 | 7.136e-7 | 7.451e-9 |
| 44,100 | 5.960e-8 | -153.749 | 1.179e-7 | 9.756e-7 | 7.451e-9 |
| 48,000 | 5.960e-8 | -153.856 | 9.369e-8 | 7.038e-7 | 1.863e-9 |
| 88,200 | 5.960e-8 | -154.288 | 1.857e-7 | 6.962e-7 | 6.519e-9 |
| 96,000 | 5.960e-8 | -154.457 | 1.148e-7 | 6.116e-7 | 6.519e-9 |
| 192,000 | 5.960e-8 | -155.499 | 1.220e-7 | 5.469e-7 | 5.122e-9 |
| 384,000 | 5.960e-8 | -156.772 | 1.063e-7 | 7.221e-7 | 3.027e-9 |

Across these rates:

- Absolute noise RMS gain deviation: at most `1.532e-9 dB`.
- Individual band complex-response error: at most `3.058e-8`.
- Reconstructed impulse peak: sample **0**, with no added algorithmic latency.
  The host reports **0 samples** latency and **0 seconds** tail.
- DC steps, discontinuities, Nyquist-alternating samples, `1e-20` input, silence,
  and levels up to ±4 pass reconstruction tolerances. Impulse tails cancel to
  the residuals above, preserving the original transient.
- Band outputs are identical across tested block partitions for seeded noise.
  A silent stereo channel has exactly zero samples in every band while the
  other channel is driven. Mono matches the corresponding stereo channel.
- Reset/reprepare, writable independent bands, and finite output checks pass.
- Processor integration passes at 8, 44.1, 48, 96, and 192 kHz in mono/stereo
  with callback sizes 0, 1, 17, 64, 127, 512, and 4097, against a prepared maximum
  of 64. A zero advertised maximum is safely treated as one sample.

Automated acceptance limits are peak noise/impulse error below `2e-7 FS`, noise
null below `-135 dBr`, summed magnitude and noise gain deviation below `1e-5 dB`,
summed phase deviation below `1e-4 degrees`, and individual complex band error
below `2e-6`. These allow modest compiler/platform rounding differences.

## CPU measurement

Stereo, 48 kHz, median of seven batches, each processing approximately one second
of audio after warm-up. Includes the headless processor callback, splitting,
float band storage, recombination, and benchmark checksum overhead.

| Callback size | Time per stereo frame | Fraction of one core |
| ---: | ---: | ---: |
| 16 | 9.28 ns | 0.0445% |
| 64 | 8.77 ns | 0.0421% |
| 256 | 8.66 ns | 0.0416% |
| 1024 | 8.64 ns | 0.0415% |

These are local DSP microbenchmarks, not DAW CPU-meter readings or worst-case
scheduling guarantees. They exclude host, editor, audio-device buffering, and
system scheduling. CPU figures are reported rather than asserted in CTest.
Zero algorithmic latency does not remove device/host latency.

## Release build and tests

Successful local **macOS arm64 Release VST3** build:

```sh
cmake -S . -B build -DJUCE_PATH="$PWD/JUCE" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build --config Release --parallel 8
ctest --test-dir build --build-config Release --output-on-failure
```

All three suites pass: `ElectroBowFilterBankTests`,
`ElectroBowPitchDetectorTests` (retained component regressions), and
`ElectroBowEditorSmokeTests` (actual embedded UI open/reopen).

Artifact: `build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3`.
The original build generated `moduleinfo.json` after its signing step, leaving the first
signature stale. Current CMake builds sign and verify after manifest generation. The final local bundle was signed again and strict verification
passed. On macOS, after building:

```sh
codesign --force --deep --sign - build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
codesign --verify --deep --strict build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
```

Windows, Linux, and Intel macOS builds were not run locally; existing CI builds
all targets and runs CTest, so the new tests are automatically included there.

Reproduce detailed measurements with:

```sh
ctest --test-dir build --build-config Release -R ElectroBowFilterBankTests --verbose
```

`python3 run_tests.py` also builds the new test target.

## Modified files

| File | Change |
| --- | --- |
| `Source/FilterBank.h` | New reusable four-band DSP, buffers, and lifecycle API |
| `Source/PluginProcessor.h` | Filter bank ownership; remove active analyzer/synth state |
| `Source/PluginProcessor.cpp` | Transparent split/sum callback, chunking, zero latency/tail, idle legacy telemetry |
| `tests/FilterBankTests.cpp` | New reconstruction, response, lifecycle, integration, and CPU checks |
| `tests/PitchDetectorTests.cpp` | Retain legacy component tests; move processor coverage to new suite |
| `CMakeLists.txt` | Register filter-bank source and CTest target |
| `run_tests.py` | Build the new suite in the test wrapper |
| `README.md` | Describe Phase 1 and link this report |
| `docs/phase1-filter-bank.md` | Design, measurements, reproduction steps, limitations, and file list |
