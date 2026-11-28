# Stage 07 · Private Rust O2 electrical pickup response

**28 November 2026 · Submitted for independent Review 7.** The isolated Rust lane reproduced the Stage 06 delivered waveform on every sample of 27 retained fixtures. Across the 152 qualified guide cases, all twelve physical/M5 columns matched the original bit for bit. The 51 neutral stationary rows passed. The nine injected identity failures still hold downstream coloration. [Measurements](../../../build/stage-07/results.json) · [Physical parity and tap errors](../../../build/stage-07/parity.json) · [Listening package](../../../build/stage-07/listening.zip)

## Revisions and files

Entry was `c9e480a4b2b85b4cd23e936b60b477a345c6ae3f`, clean tracked and untracked inventory on `feature/bowed-string-engine` at the remote-tracking tip. Existing ignored builds, dependencies and caches were present. Rust and replay implementation began in `e7b7be5`. Final executable evidence revision is `ebe9e1369324d92bbb80196936ec53e601dfb070`. The final handoff commit is documentation-only and can be identified with `git log -1 --diff-filter=A --format=%H -- docs/implementation/status/STAGE_07_O2_IMPLEMENTATION.md`. The JSON records the executable revision and source digests.

| Changed files, relative to the repository | Role |
|---|---|
| `rust/dsp/src/instrument/{mod.rs,o2.rs,o2/tests.rs}` | Private module declaration, ordered sample transaction, allocator and state tests |
| `tools/validation/{guide_render.rs,guide_audit_build.py,o2_replay.rs,o2_lifecycle.rs,o2_probes.rs}` | Shared original telemetry writer, disposable instrumented builds, observed guide and electrical traces, counters, partitions, long step and timing |
| `tools/validation/{o2_audio.py,o2_bench.py,o2_coverage.py,o2_qualification.py,o2_response.py}` | Guide parity, electrical oracle, exact inventory, V17, audio, Stage 06 reference recurrence |
| `tools/validation/{o2_mutations.py,contact_mutations.py,o1_state_parity.py}` | Runtime rejection of compiled defects and historical O1 state replay |
| `tools/validation/fixtures/{o2_corner_peaks.tsv,stage07_audio_sha256.tsv}` | Stage 06 reference peaks for the 48 distinct wet stimuli crossed into 384 corners, and 101 generated audio/package/key checksums |
| `docs/implementation/{O2_ACCEPTANCE_MATRIX,ACOUSTIC_VALIDATION_PLAN,ACOUSTIC_MIGRATION_PLAN,ELECTROBOW_MIGRATION_PLAN}.md`, this report and the status ledger and decision log | Scoring contract, ownership and submission record |

Review 6 authorized the isolated provisional response. The shipping instrument's call graph, its C ABI, JUCE parameters and IDs, UI, presets, original M5 and legacy fallback have zero executable source changes. The only edit to the shipping instrument module file registers the private module. The qualified contact, ring, guide and observer files have the entry Git blobs. [Review 6 authority](REVIEW_06_O2_SPECIFICATION.md) · [Rust lane](../../../rust/dsp/src/instrument/o2.rs)

## Numerical and lifecycle contract

`O2Coefficients::prepare(fs) -> Result<Self, PrepareError>` takes finite binary64 rates from 32000 through 192000 Hz inclusive. Prepare caches six f64 values, checks finiteness, the blocker pole and the second-order Jury stability conditions. These fixed provisional engineering values await instrument-specific calibration. The exact ordered calculations are

```text
c = exp(-2π·20/fs)                  K = tan(π·3200/fs)
D = (1 + √2·K) + K·K
b0 = K·K/D                         b1 = 2·b0                    b2 = b0
a1 = 2·(K·K - 1)/D                 a2 = ((1 - √2·K) + K·K)/D

h  = c·((old_h + v) - old_v)       a = (0.85·h)/√2
l  = b0·a + old_z1
z1 = (b1·a - a1·l) + old_z2        z2 = b2·a - a2·l
q  = l + (0.025·depth)·(a - l)
```

`O2Lane::tick(&mut self, &coeff, v, noise) -> Result<f64, OutputFault>` consumes signed O1 transverse velocity in m/s and bounded injected detail. It checks v in [−8,8], depth in [0,1] and all four old histories for finiteness. It validates all six prospective values before storing v, h, z1 and z2 together. The return is q in FS. The common-gain engineering probe is √2·q. The preparation error and sample fault have distinct types. Fresh construction and `reset(&mut self)` set all four memories to positive zero. Preparation owns every exp/tan call.

The harness launches the physical guide, resets O2, then calls `Guide::tick_observed` once per frame. A live observation belongs to the just-committed physical sample. The harness feeds its `velocity()` to O2. A valid cleared tick emits positive zero and skips O2. A physical fault returns no observation, and the guide performs its own invalidation. A downstream O2 fault returns no delivered sample, retains the successful M5 commit as evidence, invalidates the guide once and resets the lane. At tick 700 physical commits and O2 calls both stayed at 653. At tick 923 both advanced to 777, the physical fault count stayed at one and the guide invalidation count advanced by one. The controller owns any future cached fade. [Harness](../../../tools/validation/o2_replay.rs) · [Lifecycle schedule](../../../tools/validation/o2_lifecycle.rs)

The 66 named lifecycle rows cover three rates. They include positive-zero prepare, repeated dirty reset, active replacement launch, sustained play, continuous retune, compatible continuation, physical lift, a separately labeled harness release interval and disposal, fault, cleared idle and fresh epoch. Coefficients are fixed for an entire rate. Invalid rate, input, depth, poisoned histories and prospective overflow receive typed errors. The overflow fixture preserves the bits of all four old values. For 1,536 frames at each rate, single-sample, 127-frame, deterministic random and event-aligned partitions produced identical sample bits, timestamps, disposition, state, M5 and counters. Empty calls left clocks and state intact. Full ABI/host V19 remains downstream.

## Independent electrical results

The independent oracle first differences physical velocity, then integrates those differences through an exponential pole. It maps a complex analog pole through the bilinear transform, forms the conjugate-pair impulse analytically, applies the double-zero numerator, normalizes at DC and convolves with the converted signal. This avoids the Rust DFII-T state update. The differential checks h in m/s and a, l, q in FS at every tick. A failure reports rate, fixture, tick, input, depth, previous Rust state and expected/actual tap. The fixtures cover impulse, constant, delayed step, sine, multitone and all rendered O1 streams. Across synthetic and guide inputs the maximum absolute errors for h/a/l/q were 1.49e−14 / 8.97e−15 / 1.40e−14 / 1.40e−14, within the 1e−9 limits. [Oracle and electrical bench](../../../tools/validation/o2_bench.py)

All seven standard rates and the inward binary64 neighbors of 32 and 192 kHz, 44150.5 Hz and 48100 Hz passed at depths 0/.5/1. The FFT uses the smallest power of two covering half a second. The independent frequency reference is the warped analog prototype times the analytic blocker. The cutoff is root-found from the ratio of the two actual Rust impulse taps l/a. Every depth includes complex gain, phase and moment-derived group delay at DC, 20, 82.41, 196, 440, 1000, 3200, 6000, 12000 Hz and Nyquist. DC and neutral Nyquist have unavailable phase/delay. Positive-depth Nyquist has finite gain. Load DC gain is one, the neutral numerator has a double Nyquist zero and every pole radius is below one.

| Measurement across all eleven rates and three depths | Worst observed | Limit |
|---|---:|---:|
| Normalized absolute complex-gain error | 5.87e−14 | 1e−9 |
| Load cutoff error | 2.10e−11 Hz | 1e−6 Hz |
| Eligible group-delay error | 5.52e−12 s | 1e−8 s |
| Terminal unit-impulse tap/state magnitude | 4.50e−31 | 1e−10 |
| Maximum delivered output after the first second of a unit step | 1.02e−13 FS | 1e−10 FS |
| Maximum absolute terminal blocker/load memory after the step | 1.70e−13 | 1e−10 in the corresponding units |

The 33 long steps used the actual Rust tick for all 178,731,093 frames, including every frame after the first second. The previous physical input ended at exactly 1 m/s in each row. Each tick enforced the state checks. Each row also exercised fresh exact-zero input, dirty reset to four positive zeroes and the first sample after reset. The JSON preserves the first/peak step sample, post-one-second maximum and all four terminal histories by rate and depth. [Per-rate electrical results](../../../build/stage-07/results.json)

## V17, negative controls and gain

At positive polarity, G=1, neutral identity and fixed delivery trim, the final stationary second uses uncentered RMS of delivered wet over the original converted M5 on the same tick stream. Direct, Out and protection are absent. Spatial v/N, l/a, detail insertion, common U/M5, coloration and local detail subtraction retain separate fields. Across 51 rows the wet/M5 range was −2.761 to −0.811 dB against ±3 dB. Minimum factory harmonic fraction was .996679 against .85. The largest factory wet residual was −36.424 dB against −28 dB.

* The scorer considers every positive FFT bin, preserves an endpoint maximum and interpolates eligible interior maxima. The M5 anchor and delivered strongest component must each be within two native bins or 15 absolute cents of target. The 48 kHz 196 Hz/Nyquist mixture selected exactly 24000 Hz and failed. The 15.04-cent-flat 440 Hz fixture and an invalid original bridge anchor also failed.
* An independent inventory checks 11 electrical rates × 3 depths, five differential stimuli per depth, all required frequency points, 51 stationary/witness rows, nine stress rows, three sweeps, 384 corners, 66 lifecycle entries and 29 scorer controls. Applicable fields must be finite. Missing and duplicate keys, missing applicable flags, nulls and unexpected failures reject the run. Removing the 48 kHz/196 Hz/.10/1.8/.5 row or inserting a +8 dB peak at 3 kHz failed the full measurement aggregate.
* The measured sweep maxima at depth 0/.5/1 were −.02064 / −.02330 / −.02557 dB at 530.00 / 493.43 / 465.14 Hz. All three widths are unavailable because the low-frequency boundary truncates the −3 dB interval. The scorer retains eligible endpoint maxima, contiguous widths and the peak list for the +6 dB hold.
* The local detail diagnostic subtracts returned q samples. The factory gate subtracts aligned final wet before RMS using the off-depth wet denominator. The injected 6151 Hz coloration left local q unchanged (reported −240 dB floor) while the measured final-wet residual was −20 dB and failed. A one-tick displacement and mismatched settings also failed. [Scorer](../../../tools/validation/o2_qualification.py) · [Aggregate and controls](../../../tools/validation/o2_coverage.py)

At 48 kHz, 440 Hz, V=.16 m/s, R=1.8, neutral detail and identity, Rust measured **1.005418823 FS common** and **.710938468 FS delivered** including launch. G=1.4 and internal Out=2 imply **1.990627709 FS** and **6.155279 dB** instantaneous demand relative to .98. The 384 corners use measured Rust returns, the frozen explicit identity vectors and settled downstream scalar arithmetic. All mute corners were zero and the dry maximum was .700 FS.

| Injected vector in H1…H12 order | Largest implied pre-protection peak, FS | Demand, dB |
|---|---:|---:|
| Neutral | 1.996430 | 6.180559 |
| All +5 dB | 3.315087 | 10.585378 |
| All −5 dB | 1.699587 | 4.782345 |
| −5, +5, then ten zeroes | 2.125602 | 6.725118 |

Both level vectors failed coloration and wet/M5 level at 82.41, 196 and 440 Hz. The inversion vector failed strongest-fundamental at all three pitches, measuring 391.84 Hz on the 196 Hz case. All nine failures are recorded by name. These are downstream identity stress observations. S6R owns the linked nominal-use measurement. Review 6 found that a single hypothetical 1.001 FS request at 48 kHz can leave binary64 limiter gain fixed at .9999999999998668 after 61,599 recovery updates. The exact g<1 count can therefore exceed 1,439 of 144,000 eligible frames with very few instantaneous requests. The integrated run must report the first request, recovery and both counts. [Review 6 finding](REVIEW_06_O2_SPECIFICATION.md#gain-and-headroom)

All 15 disposable O2 mutations compiled, then failed their nominated runtime checks. The first eleven use actual Rust taps to distinguish missing/wrong blocker, prewarp, cutoff, denominator sign, load state update, missing trim, duplicate conversion, duplicate trim, reversed detail and excess detail. The remaining four detect missing reset, partial publication, stale observation and a replacement guide tick after an output fault. The mutation report retains the first divergent quantity/tick or the exact lifecycle assertion. [Mutation results](../../../build/stage-07/mutations.json)

## Local real-time evidence

`tick` reaches the ordered arithmetic in `advance`, finite checks, inclusive range comparisons and fixed-size array iteration. Reset stores four zeroes. The sample path has no heap operations, locks, I/O, blocking, data-dependent loop bound, transcendental or coefficient preparation. Temporary taps are local values. The actual Rust types measure 48 bytes for six immutable coefficients and 32 bytes per lane, or 112 bytes shared across two lanes. The thread-local Rust allocator counted zero allocations, reallocations or deallocations over 60 s of processing at each rate extreme with 12 lifecycle cycles, changing depth, lift, both faults, repeated launches and idle. A further 30 s of alternating subnormal input ran with the counter armed.

Apple M2 Pro, macOS 26.5.1 arm64, rustc 1.100.0, optimized standalone build with the validation counters. Each timed workload warms up for one simulated second and runs five 30-second batches. O2 alone uses alternating ±.1 m/s and rotating 0/.5/1 depth. The two guide workloads sustain 196 Hz, V=.10 and R=1.8. The near-denormal workload alternates ±1e−310 m/s and uses identical depth changes to warmed silence. Tiny active blocker histories persisted. Analysis eligibility never writes DSP state. The table gives medians in ns/tick. The JSON and TSVs retain every batch and the measured terminal blocker values. These measurements cover the isolated paths. Full callback V18/V21 requires integration.

| Hz | O2 | Guide + O1 | Guide + O1 + O2 | Silence | Near-denormal | Near-denormal / silence |
|---:|---:|---:|---:|---:|---:|---:|
| 32000 | 7.571 | 53.532 | 60.487 | 7.553 | 7.539 | 0.998 |
| 48000 | 7.561 | 53.050 | 60.228 | 7.544 | 7.532 | 0.998 |
| 192000 | 7.547 | 52.915 | 59.931 | 7.546 | 7.533 | 0.998 |

Full repeated timings and terminal blocker histories are available in the per-rate TSVs. [32 kHz](../../../build/stage-07/timing-32000.tsv) · [48 kHz](../../../build/stage-07/timing-48000.tsv) · [192 kHz](../../../build/stage-07/timing-192000.tsv)

## Physical parity, audio and listening

The 152-case guide run regenerated all 12 canonical M5 WAV hashes. Guide-only, guide+O1 and guide+O1+O2 preserved every bit of the twelve physical/M5 columns over 46,242,000 ticks. All ordinary replay ticks had one commit and one primary electrical call, one launch and zero faults per epoch. The O1 independent reconstruction differed by at most 4.441e−16 m/s. The historical private-state probe matched 13,820 rows (SHA-256 `3565e329514d00d05e28272120b4ca0cae3a3d000ec5da959f6496439451a51f`). The state builder now loads the matching historical module registration when compiling the pre-O1 guide. The ten-minute guide stress completed 600 launches with zero faults. Post-commit fault injections have their own lifecycle fixtures. [Guide/O1 replay](../../../build/stage-07/final-evidence.log) · [State and licensed fixture checksums](../../../build/stage-07/state-and-hashes.log)

With the four extra corner streams the Rust audio differential covered 156 renders and 46,818,000 physical ticks. On all 27 retained fixtures the delivered waveform differed from the Stage 06 recurrence by exactly 0 FS at all three depths. The 81 standalone float WAVs preserve original M5 and delivered O2 off/factory for the pitch, pressure, speed, bend, vibrato, lift and high-note comparisons. The compact ZIP uses Stage 06's seeded assignments, engineering M5 and common-gain O2, separately matched copies, fixed per-clip scalars and original-gain licensed low-E reference. Matching targets .10 FS RMS over [.2,.8) s with common peak attenuation if needed. ZIP SHA-256 is `39dd496030d731b3d8bbf39c15dd420de61d116f89fa1bb6b61eb6483c497712`, identical to the Stage 06 archive. Owner listening is pending. [Listening ZIP](../../../build/stage-07/listening.zip) · [Key and gains](../../../build/stage-07/listening-key.json) · [File hashes](../../../tools/validation/fixtures/stage07_audio_sha256.tsv)

At 1318.51 Hz the common-band report includes all seven rates at 0/.5/1 detail. Whole three-second traces are polyphase-resampled to 32 kHz with Kaiser β=10 and zero extension. The interior [1.5,2.5) s window uses (0,14] kHz power, [6,14] kHz upper-band fraction and each rate's validated bridge anchor. The neutral upper-band percentages at 32/44.1/48/88.2/96/176.4/192 kHz were .0176/.0196/.0259/.0941/.0895/.1262/.1202. The JSON reports harmonic amplitudes, eligible phases, centroid, nonharmonic residual, differences from 192 kHz and the 96→32 diagnostic. These rate-dependent differences include propagation and interpolation effects. An output filter cannot remove previously folded in-band energy.

| 48 kHz seeded neutral O2, RMS FS | First 10 ms | First 50 ms | Steady .5–1 s | First 25 ms after physical lift | 1.1–1.15 s | 6–7 s |
|---|---:|---:|---:|---:|---:|---:|
| 82.41 Hz | .16633 | .17818 | .17194 | .16808 | .13941 | 2.071e−5 |
| 440 Hz | .17070 | .16519 | .16707 | .15557 | .04886 | 1.732e−22 |

These samples start from the M4 seed. The full onset/lift table retains uncentered/AC RMS, mean, peak, harmonic envelope, eligible phase and same-source differences for every specified window and depth. The lift at 1 s removes physical work while the electrical histories continue. Natural attack, musical release and amended gesture still require their own evidence.

## Reproduction

Use the repository root, pinned Rust toolchain, native build prerequisites and the existing Python environment (3.9.6, NumPy 2.0.2, SciPy 1.13.1, Pedalboard and SoundFile for the shipping comparison). Existing guide/acoustic runners own their ignored Stage 03–05 directories. New audio, traces, measurements and mutation builds live in ignored `build/stage-07/`. The package baseline uses its existing ignored directory.

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1
py=build/spec-reconciliation/py/bin/python
out=build/stage-07
mkdir -p "$out"
export TMPDIR="$PWD/$out"

cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test --workspace --release --locked --offline
cargo test --workspace --release --locked --offline --target x86_64-apple-darwin
cargo test --release --lib isolated_read_cost --locked --offline -- --ignored --nocapture

rustc --edition 2024 -O tools/validation/contact_replay.rs -o "$out/contact"
"$py" tools/validation/contact_contract.py
"$py" tools/validation/contact_contract.py --rust "$out/contact"
"$py" tools/validation/contact_mutations.py
"$py" tools/validation/string_oracle.py
"$py" tools/validation/ring_oracle.py
"$py" tools/validation/ring_mutations.py
"$py" tools/validation/guide_qualification.py
"$py" tools/validation/guide_negative_controls.py
"$py" tools/validation/guide_mutations.py
build/stage-03/guide-render --stress
"$py" tools/validation/acoustic_observation.py
"$py" tools/validation/o1_qualification.py
"$py" tools/validation/o1_mutations.py
"$py" tools/validation/o1_state_parity.py
"$py" tools/validation/acoustic_identity.py
shasum -a 256 -c tools/validation/fixtures/acoustic/SHA256SUMS
"$py" tools/validation/o2_qualification.py
"$py" tools/validation/o2_mutations.py
unzip -tq "$out/listening.zip"
(cd "$out" && shasum -a 256 -c ../../tools/validation/fixtures/stage07_audio_sha256.tsv)

cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
bundle=build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
codesign --verify --deep --strict "$bundle"
shasum -a 256 "$bundle/Contents/MacOS/ElectroBow"
"$py" tools/validation/plugin_baseline.py
```

Formatting and Clippy passed. Rust debug, release and Rosetta x86_64 release each passed 86 unit and three integration tests. The ignored O1 timing test ran separately. Contact and ring oracles passed. Three contact, six ring, eleven guide and twelve O1 mutants were rejected. The guide negative controls, acoustic/identity replay and all 25 licensed fixture digests passed. Native Release CTest passed 4/4. Deep strict signing verified the rebuilt VST3. The 13-rate package baseline reproduced the prior input, ABI and plugin wet hashes, with zero tail peaks, worst neutral error 2.98023224e−8 FS and ABI/package wet difference 1.1712931e−38 FS. The rebuilt executable has its own digest in the regression log. Native Intel hardware, Windows and Linux execution were unavailable. [Rust, native and package log](../../../build/stage-07/final-regression.log) · [Oracle and mutation regression log](../../../build/stage-07/final-controls.log) · [Scorer and file verification](../../../build/stage-07/submission.log)

The owner retains the pickup calibration, voicing, fixed gain/trim, public Noise semantics and perceptual decisions. Review 7 owns the isolated implementation decision. I1, final A2, live tracking, amended articulation/S4b, supported S5 wet routing, linked S6R headroom, ABI/host V19, V20/V21 and release remain gated. Baseline injected S4a retains its separate authorization. The larger proposed acoustic architecture retains its existing review status.
