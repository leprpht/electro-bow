# Roadmap audit and CI repair

Audited baseline: `4913918`, including the Rust migration, Attack, Bow Character,
and integrated-effect commits. The supplied roadmap names four phases but only
specifies Phases 1–3; this audit does not invent a fourth phase. The later phases
already exist in merged PRs #11–13. This change repairs and validates that existing
implementation rather than starting another development phase.

## Requirements and evidence

| Requirement | Implementation and validation |
| --- | --- |
| Four parallel IIR bands, 200/800/3000 Hz | Rust `filter_bank.rs`: `L200`, `L800-L200`, `L3000-L800`, `1-L3000`. Complementary amplitude sum, independent channel histories, accessible band buffers. |
| Transparent unprocessed recombination | Complex band-response checks, impulse/tail cancellation, noise null, channel isolation and C++ reference parity in `FilterBankTests.cpp`, 4–384 kHz. |
| Mono/stereo, variable buffers, real-time operation | Preallocated Rust buffers; JUCE chunks oversized callbacks. Allocation-counting tests cover the complete path. No audio lookahead or reported latency. |
| Continuous Attack smoothness | Rust `attack.rs`: level-relative fast/slow spectral envelopes and smoothed per-band gains. Low band untouched; upper transients reduced. No note detector, pitch tracker, event trigger or full-band fade-in. |
| Playing dynamics, chords, picking, tremolo, sustain | Existing isolated-stage tests plus new complete-path Rust scaling/partition tests covering eight fixtures, five sample rates, and minimum/intermediate/maximum settings. |
| Coordinated Bow Character | Rust `character.rs`: envelope-dependent spectral balance, input-multiplied filtered random friction, bounded middle-band nonlinearity and irregular spectral movement, coordinated by one smoothed control. |
| No MIDI, pitch detection, voice separation, synthesized strings, STK Bowed | None in the production chain. aubio/STK remain only in legacy component tests behind `BUILD_TESTING`. |
| Recognizable bowed-string sound | **Perceptual acceptance remains unverified.** Synthetic fixtures demonstrate signal properties, not recognition by listeners on recorded/live guitar. Existing offline comparison generator is reproducible. FL Studio listening/stress validation remains pending. |

The filter bank alone reconstructs transparently. The integrated effect retains
the existing 0.98 FS sample-peak protector, which intentionally changes gain on
overload and during its recovery. Transparency and exact dynamics scaling for
the complete effect apply below protection activity. Nonzero Attack/Character
intentionally change spectral magnitude and phase. No new synthesis or DSP
language migration is necessary; Rust remains the production DSP language.

## Gaps fixed

- Non-finite floats in saved sessions could pass through `std::clamp`, including
  Attack=NaN, making the Rust process call reject every callback. State loading
  now validates the complete v1/v2 payload before changing any control. Regression
  tests exercise every field with NaN/+Infinity/-Infinity, truncated payloads,
  and actual audio processing after rejection.
- Host callbacks now snapshot Attack/Character once before chunking, avoiding
  different settings within one oversized callback when the UI updates.
- Previous CPU tests fed the processed buffer back repeatedly, unlike a host.
  The benchmark now refills fresh input for every warm-up and measured block;
  reported time includes that small buffer-copy cost.
- The complete path had broad finite-output tests but no explicit level-scaling
  and block-partition assertions across the combined stages and output protector.
  The new Rust regression closes that coverage gap.

## CI findings and repair

Windows [run 44612714780](https://github.com/leprpht/electro-bow/actions/runs/44612714780)
failed before compiling the plugin: a restored generated build tree referenced
`C:/Program Files/CMake/share/cmake-4.5`, while the running CMake used 4.4 modules.
Errors included missing `DartConfiguration.tcl.in` and `CTestScript.cmake.in`.
The cache key and broad restore prefix did not account for runner tool changes.
Both CI and release workflows now configure from clean build trees and cache
only compiler outputs. Cache keys advance per commit, allowing new compiler
results to be saved; compiler content is checked before reuse. Windows uses its
native Visual Studio build without installing an unused compiler launcher.

macOS [run 44612762908](https://github.com/leprpht/electro-bow/actions/runs/44612762908)
was stalled in the universal build. After cancellation, its logs showed both Rust
slices compiling successfully, many concurrent JUCE C++ compilations, then no
progress from 20:58:28 UTC until cancellation at 21:14:29 UTC. The same code passed
macOS in run 44612714780. `build.py` used bare `--parallel`, which permits unbounded
GNU Make concurrency. Resource contention is a plausible explanation, not a
proven OOM diagnosis: the available log contains no memory-kill report.

Build concurrency is now explicitly bounded: three jobs on hosted runners,
locally up to eight by default, with `--parallel`/`CMAKE_BUILD_PARALLEL_LEVEL`
overrides. Both workflows have job/build/test timeouts, report tool versions and
compiler cache statistics, and retain verbose CTest diagnostics. Superseded CI
pushes cancel automatically. Node and Python major/minor versions are explicit.
Release downloads select only VST3 artifacts, excluding diagnostic uploads.

## Local Release validation

Apple M2 Pro, macOS arm64:

- 14 Rust tests, formatting and Clippy with warnings as errors: passed.
- All four CTest suites, including actual editor open/reopen: passed.
- Release VST3 and strict final bundle signature verification: passed.
- Both workflow files: `actionlint` passed.
- Complete-path tenfold scaling maximum error: `7.450580597e-9` FS;
  17/257-frame partition error: exactly zero on this machine.
- Filter-bank peak reconstruction error: `5.96046448e-8` FS; worst noise null:
  `-153.250729` dBr; magnitude deviation below `1.86e-7` dB; phase deviation
  below `1.40e-6` degrees. Algorithmic buffering latency: zero samples.

Corrected full-path benchmark, stereo 48 kHz, both controls at maximum, median
of seven one-second batches (includes input refill, dispatch and checksum):

| Frames | One-core CPU | ns/stereo frame |
| ---: | ---: | ---: |
| 16 | 0.2246% | 46.80 |
| 64 | 0.2144% | 44.66 |
| 256 | 0.2156% | 44.91 |
| 1024 | 0.2130% | 44.38 |

These replace the benchmark method in `final-effect.md`; they are local
microbenchmarks, not host scheduling guarantees. CI results are linked from the
pull request; cross-platform success is required before opening it.

## Modified files

- `.github/workflows/ci.yml`, `.github/workflows/release.yml`: portable caches,
  bounded jobs/timeouts, runtime versions and retained diagnostics.
- `build.py`: explicit build parallelism with environment and CLI overrides.
- `Source/PluginProcessor.cpp`: finite session validation and callback snapshots.
- `rust/dsp/src/signal_path_tests.rs`: combined-stage dynamics and partition regression.
- `tests/FilterBankTests.cpp`: invalid-session regressions and corrected benchmark.
- `docs/roadmap-audit.md`: requirements, failure evidence and current measurements.
