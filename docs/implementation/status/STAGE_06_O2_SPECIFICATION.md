# Stage 06 · Electrical pickup specification and offline evidence

**27 November 2026 · Awaiting independent Review 6.** Retain the 20 Hz blocker, 3200 Hz Butterworth, .85 FS/(m/s) conversion and fixed 1/√2 delivery trim as one reproducible candidate. The neutral candidate meets its isolated level, fundamental and factory-residual predicates. Extreme injected identity gains violate V17. Electrical calibration to a particular instrument, nominal linked-limiter use and the owner's listening verdict remain open. [Rust implementation contract](../../architecture/ELECTROBOW_O2_PICKUP_RESPONSE_SPECIFICATION.md) · [Acceptance matrix](../O2_ACCEPTANCE_MATRIX.md) · [Listening package](../../../build/stage-06/listening.zip)

## Authority and revision

Entry `85b0aa10c5ee3b19f09637c0d78d8d5023bdc9d2`, clean index and working tree, branch `feature/bowed-string-engine` at its remote tip. The specification and fixture commit is `36b85e5218a4541506b6d308c9ca495af44ba656`. Final executable evidence revision `a515db76285e3dbc507d0bc483d2ccf9116ea8c1`. The submission ends with a documentation-only ledger/report commit, identifiable with `git log -1 --diff-filter=A --format=%H -- docs/implementation/status/STAGE_06_O2_SPECIFICATION.md`. The program records its checked-out revision and tool digests on each replay.

Review 5 authorized five exits. (1) Name the SI, common-gain, delivery, sign/G, E, A4, Out and protector taps with one application of each gain. (2) Freeze denominators, harmonic eligibility and sweep criteria while preserving original V17. (3) Specify nominal .35 FS pluck, linked active frames, gain corners and lane-local cached-product fault recovery. (4) Seek a documented pickup/load measurement and evaluate response through 192 kHz. (5) Compare high-band, onset/lift and same-source detail, and record the owner's gain, Noise and listening decisions. Review 5 also called for separating independently authorized baseline S4a from the held stroke and correcting near-silent reporting. The handoff preserves the approval boundary for each. [Exact open requirements](REVIEW_05_O1_OBSERVATION.md#open-findings-and-next-authorized-task) · [Reconciled migration](../ACOUSTIC_MIGRATION_PLAN.md)

## Electrical and physical references

**Direct electrical evidence.** Helmuth Lemme's *Pickup Measuring Technique*, Fig. 2 and p. 31, reports a 1972 Fender Stratocaster bridge pickup, inductance 2.2 H. External capacitance is 470 pF (cable-equivalent load), resistance 200 kΩ (250 kΩ volume pot in parallel with 1 MΩ amp, no tone pot). The specific cable, winding resistance and intrinsic capacitance of this specimen are unspecified. A swept transmitting coil excites the pickup and the analyzer measures its AC output. The graph shows digital level in dBFS versus Hz over roughly 100 Hz–10 kHz. It rises above the low/mid plateau to a resonance near 4.5 kHz. The field drive and display reference supply no absolute string m/s→FS sensitivity. Use the published measurement for qualitative comparison. No trace was digitized or redistributed, and we found no open license for the figure. [Lemme, Sustain, 2013](https://www.planetz.com/wp-content/uploads/2014/10/Pickup_Measuring_Technique.pdf)

**Physical interpretation.** The Review 4 work by Horton–Moore and Paiva–Pakarinen–Välimäki distinguishes flux-gradient/velocity sensing, aperture and the loaded electrical circuit. A coil's inductance and total capacitance permit a resonance whose height depends on losses and external resistance. The measured resonance is within the useful comparison band, whereas the candidate is already −3.0103 dB at 3.2 kHz and descends monotonically thereafter. The study supports treating treble response as load-dependent. It supplies no basis for fitting this exact cutoff, Q, 20 Hz corner or digital sensitivity to the reference guitar. No electrical band was fitted or used as a held-out validation band. The entire published band was a qualitative check after the existing constants were fixed. The earlier low-E fit used H2–H6 (through ~494 Hz) for geometry, with other windows from the same performance. [Horton and Moore](https://guitarelectroscience.com/wp-content/uploads/2020/07/Horton.pdf) · [Paiva et al., §§1–5](https://aaltodoc.aalto.fi/bitstreams/92759095-3b64-4a0f-bf29-8a3617b9d92c/download)

**Comparable-pitch playing reference.** Vaisvil describes using a viola bow on a Stratocaster copy with Lace pickups. We extended the existing CC BY 4.0 fixture to one low-E phrase at 22.125–23.875 s of the verified 44.1 kHz stereo MP3 preview. The checked-in left-channel float WAV preserves source gain. First .125 s / middle [.375,1.375) / last .125 s give .0381/.1173/.0267 FS RMS, with an 82 Hz strongest middle bin. This supplies an audible rise, sustained low E and level fall at the guide's pitch. The recording has continuing motion, an unknown pickup switch/load, gain/EQ, unmarked bow reversals, codec and surrounding notes. Absolute onset time, lift time and passive decay from a known bow action remain unavailable. The independent Richard McCormick bowed Stratocaster pack documents release samples and three round robins, but requires login to download. We did not obtain raw comparable-pitch samples with synchronized bow-contact annotations. The engineering envelope has no physical timing fit. [Vaisvil original and license](https://freesound.org/people/vaisvil/sounds/103565/) · [Preview](https://cdn.freesound.org/previews/103/103565_1735967-hq.mp3) · [Fixture provenance](../../../tools/validation/fixtures/acoustic/ATTRIBUTION.md) · [McCormick recording](https://www.pianobook.co.uk/packs/bowed-electric-guitar-with-release/)

## Electrical bench

All seven V20 rates plus 32000⁺, 44150.5, 48100 and 192000⁻ Hz passed the normalized 1e−9 complex-gain criterion against the independent warped analog prototype. Here ⁺/⁻ are adjacent binary64 values toward the interior. The maximum cutoff error was 2.28e−12 Hz or less. The largest terminal impulse magnitude/state across cases was below 4.50e−31 in the specified window. The 60 s unit step left at most 1.70e−13 m/s blocked residual and finite load states, with previous input still exactly 1 m/s. At 48 kHz the step's maximum loaded output was .617155 FS. Each rate/depth has its own DC, passband, stopband, Nyquist, phase and group-delay record. [Per-rate strict JSON](../../../build/stage-06/results.json)

| Hz | DC pole | Load pole radius | Max impulse FFT complex error, all depths |
|---:|---:|---:|---:|
| 32000 | .996081 | .642496 | 8.57e−15 |
| 44100 | .997155 | .724683 | 1.36e−14 |
| 48000 | .997385 | .743821 | 1.88e−14 |
| 88200 | .998576 | .851137 | 2.15e−14 |
| 96000 | .998692 | .862354 | 3.09e−14 |
| 176400 | .999288 | .922566 | 4.78e−14 |
| 192000 | .999346 | .928627 | 5.86e−14 |

At 48 kHz load gain magnitudes at 440/3200/6000/12000 Hz are .999831/.707107/.254649/.045134. Measured cascade delay at 20/82.41/196/440/3200/12000 Hz is 4.048/.512/.152/.087/.073/.0066 ms. The loaded branch has zero theoretical Nyquist gain. Normalized Nyquist magnitude at Noise .5/1 was .012484/.024967. The 65–4000 Hz neutral electrical sweep had a maximum of −.0206 dB relative to the converted excitation and no >+6 dB peak. DFII-T cost and memory are estimates. No O2 Rust or new callback measurement was needed.

## Headroom, level and detail

At 48 kHz/440 Hz, V=.16 m/s, R=1.8, σ=+1, G=1, neutral identity and detail, the three-second O1-derived **measured** common peak is 1.005418823 FS and the delivery peak .710938468 FS. Setting E=C=1, G=1.4 and internal Out=2 **implies** 1.990627709 FS and 6.155279 dB instantaneous demand against .98. This is scalar arithmetic on the measured waveform. The actual protector and its gain history enter at S6R.

At V=.10/R=1.8, G=1, unity Out and factory detail the 196 Hz wet peak was .436459 FS. The table uses the final second and compares fixed engineering gains. The first column after pitch uses physical velocity. The remaining comparisons use equal units on both sides. η is harmonic power fraction.

| Pitch Hz | v/N dB | l/a dB | Common U/M5 dB | Delivery q/M5 dB | Factory difference/off dB | η |
|---:|---:|---:|---:|---:|---:|---:|
| 82.41 | +2.067 | −.017 | +2.190 | −.820 | −53.42 | .99921 |
| 196 | +2.042 | −.090 | +2.089 | −.921 | −48.02 | .99949 |
| 440 | +1.988 | −.176 | +1.954 | −1.056 | −44.35 | .99918 |

All 42 neutral stationary rate/control/depth rows passed their applicable original-denominator probes. The 384 corner rows cross bow speeds, pressures, captured gains, C, Out, detail and injected gain vectors. The maximum at C=0 was .700 FS and at Out=0 exactly zero. The largest implied values by vector are below. The matrix records all pre-protection peaks and instantaneous demand, including zeros. These corner estimates use binary64 waveforms and settled controls. S6R measures the actual f32 mix, trim slew and linked gain.

| Fixed vector | Largest pre-peak FS | Demand dB |
|---|---:|---:|
| Neutral | 1.996430 | 6.181 |
| All +5 dB | 3.315087 | 10.585 |
| All −5 dB | 1.699587 | 4.782 |
| H1 −5 / H2 +5 dB | 2.125602 | 6.725 |

**Original V17 failures.** At 82.41/196/440 Hz the all-up vector moves the fixed-delivery wet/M5 RMS by +3.63/+3.59/+3.53 dB. All-down gives −4.82/−5.06/−5.34 dB. Relative to neutral q those changes reach +4.58 and −4.29 dB. The inversion vector makes 164.81/391.84/879.97 Hz dominant, even though its RMS remains inside the original bound. A global trim cannot fix a change in dominant order. The all-up and all-down level intervals cannot both fit a ±3 dB budget by changing one scalar either. Retain the existing fixed trim for neutral O2 and expose the injected failures to the identity owner. No vector constitutes an I1 implementation or approval.

At 440 Hz Noise 0/.5/1 gave .439121/.438621/.438121 FS peak, .167069/.166869/.166676 FS sustained RMS, H2/H1 −1.422/−1.428/−1.434 dB, ≥6 kHz power .0327/.0292/.0260%, and nonharmonic fraction .0822/.0816/.0810%. The factory subtraction was .001013 FS RMS. Doubling depth doubles the same-source difference. The parallel return and phase of the load can **reduce** energy in a particular upper band despite admitting carrier signal at Nyquist. The measured effect is small, bounded, silent at reset and decays with the carrier. Its perceptual usefulness is open.

## High note and transients

At 1318.51 Hz the 192 kHz reference was rendered with the same V=.10/R=1.8, including actual new Rust O1 and original M4 seed at that rate. The 96 kHz reference required by V17 was also decimated. The table uses the common 0–14 kHz analysis band, native 32 kHz or interior polyphase-resampled window, neutral load. Percentages are fractions of band power times 100.

| Source rate | Raw ≥6 kHz % | Loaded ≥6 kHz % | Loaded nonharmonic % | Loaded centroid Hz | Factory-detail centroid Hz |
|---:|---:|---:|---:|---:|---:|
| 32 kHz | .762 | .0176 | .3380 | 1783.5 | 1776.9 |
| 48 kHz | 1.338 | .0259 | .0826 | 1860.5 | 1852.5 |
| 96 kHz | 2.927 | .0895 | .0581 | 1955.3 | 1944.9 |
| 192 kHz | 4.174 | .1202 | .0364 | 1976.2 | 1965.2 |

Native 32 kHz loaded H1–H4 relative to the 192 kHz reference were −.43/−1.58/−3.99/−8.03 dB. The 1% high-band diagnostic flags raw and blocked high notes at 48/96/192 kHz. All loaded high notes fall below it. Residual and redistribution vary with fractional interpolation, rate-dependent loop loss, phase locking, projection leakage and analysis bandwidth. This experiment does not isolate spectral foldover. A low-pass outside the nonlinear guide cannot remove already folded in-band energy. Final V17 and V20 remain pending.

At unity E, untrimmed mechanics launch into an established M4 seed. After electrical conversion and fixed trim the first sample at both pitches is −.013089 FS. Compare the neutral delivery RMS below. Actual peak, mean, AC RMS, harmonic envelope/eligible phase, filter-input difference and same-source Noise difference for each interval are in the replay. The 440 Hz tail still has a physical trace even after relative phase becomes ineligible.

| Pitch | 0–10 ms | 0–50 ms | .5–1 s | First 25 ms after lift | 1.1–1.15 s | 6–7 s |
|---:|---:|---:|---:|---:|---:|---:|
| 82.41 Hz | .16633 | .17818 | .17194 | .16808 | .13941 | 2.071e−5 |
| 440 Hz | .17070 | .16519 | .16707 | .15557 | .04886 | 1.732e−22 |

The 440 Hz mean over [.5,1) s was −3.57e−7 FS. Initial filtering reduces the first sample and adds a short frequency-dependent transient. Within 10 ms the RMS is already close to its steady value. Lift at 1 s removes mechanical work, the string carries the long decay, and the load continues from its current states. Musical attack, envelope and release timing belong to M4 and the future controller. A natural bow attack has yet to be demonstrated.

## Listening and decisions requested

The zip contains three one-second M5/O2 pitch pairs, one off/factory detail pair at 440 Hz, a two-second low-E sustain with lift at 1 s, and the unity-gain physical phrase. `engineering/` contains M5 at its original .85 and O2 at common gain U, both as mono 48 kHz float WAVs. `matched/` uses a scalar targeting .10 FS RMS over [.2,.8) s, with common pair attenuation only if a peak would exceed .90. The included gains file and separate listening key document every multiplier. Matched files carry anonymous labels, and the ZIP has reproducible entry timestamps. The physical reference remains at 44.1 kHz and its original level. There is no EQ, compression, saturation or reverb. Listen at fixed playback gain, starting with the reference. Record preference, buzzing, objectionable frequencies/timestamps, detail usefulness and musical sustain/release before revealing the key. [Listening package](../../../build/stage-06/listening.zip) · [Assignments and gains](../../../build/stage-06/listening-key.json)

| Decision owner | Disposition on submission |
|---|---|
| Product owner, electrical reference and gain intent | Pickup-led reference already selected. Proposed bounded smooth load and fixed trim are defensible as engineering settings for comparison. A specific pickup voicing, absolute transduction, desired damping and headroom tradeoff still need owner acceptance. |
| Product owner, Noise and listening | Decide whether public Noise on the supported path may mean intrinsic carrier detail. Accept only if the factory/endpoint change is useful on the actual listening copies and the reference direction is preferred with usable sustain/release. Record listener, date, route and failed excerpts. No human judgment has been supplied. |
| Independent Review 6 | Review the equations, tap order, evidence, guards, V17 denominators, nominal V08 fixture and retained failures. If the owner and review accept the bounded reference, authorize only isolated Rust O2 and numerical parity against this contract. |
| Downstream owners | Integrated linked nominal use <1%, .980001 FS delivered limit, actual controller fault/epoch behavior, V17/V20 final wet and V18–V22 remain open. I1 is blocked. Baseline injected S4a retains separate permission. Amended A3, final A2, live S4b, supported S5 routing, FFI/host/UI/tail and release remain held under existing stage ownership. |

## Regression and reproduction

Apple arm64, macOS 26.5.1, rustc 1.100.0, Python 3.9.6, NumPy 2.0.2, SciPy 1.13.1. Existing qualification uses its ignored Stage 03–05 paths. New generated artifacts are under `build/stage-06/`. The program checks the 15 Review 5 digests and the nine extended trace digests whenever it uses the corresponding recordings. The O1 hash-list digest is `85ecdac330745653572bd43dd1f7eeebddf80f75511e10daa9a5dc14d0084464`, extended list `d956ed978cbf8a9e5f943e897ccf137dbac7034518e8e38bd9fe55adc1b6efc2`. Keep the attributed input phrase under version control, SHA-256 `645ed8f4298842e01c752a20b7a8e52653c7fa959879fbf3f954f9fe31a757b1`. [Input manifest](../../../tools/validation/fixtures/acoustic/SHA256SUMS) · [Extended traces](../../../tools/validation/fixtures/o2_sha256.tsv)

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1
py=build/spec-reconciliation/py/bin/python

cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test --workspace --release --locked --offline
cargo test --workspace --release --locked --offline --target x86_64-apple-darwin
cargo test --release --lib isolated_read_cost --locked --offline -- --ignored --nocapture

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
unzip -tq build/stage-06/listening.zip

cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
bundle=build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
codesign --verify --deep --strict "$bundle"
shasum -a 256 "$bundle/Contents/MacOS/ElectroBow"
"$py" tools/validation/plugin_baseline.py
```

- Cargo formatting and Clippy passed. Debug and release each passed 84 unit and three integration tests. Release x86_64 passed on this Apple silicon machine. The normally ignored isolated O1 timing test was executed separately. Native Intel hardware, Windows and Linux execution were unavailable.
- The maintained guide replay regenerated 152 cases and 12 original M5 WAVs. All twelve original binary64 columns of 46,242,000 observed ticks matched the unobserved guide bitwise, and the canonical raw/WAV digest list matched Review 3. Maximum O1 reconstruction error was 4.441e−16 m/s. The private-state comparison matched 13,820 rows including lifecycle. All 12 O1 mutants and 11 guide mutants were rejected. Ten-minute guide stress recorded 600 launches and zero faults. Stage 04 observation and identity replays completed, and all 25 input hashes verified. [Physical and acoustic replay](../../../build/stage-06/replay.log) · [Guide controls](../../../build/stage-06/guide-controls.log) · [Final O1 reporting replay](../../../build/stage-06/submission-replay.log)
- All 11 electrical rate rows passed. The separate O2 negatives rejected 17 defective computations/reports. Nine injected identity cases failed at least one musical predicate as reported above. A zero runner exit confirms the isolated neutral evidence and detects the expected inversion. The JSON explicitly reports `all_tested_vectors_passed=false`. [Full per-case results](../../../build/stage-06/results.json) · [Submission run](../../../build/stage-06/submission.log)
- Release CMake build and native CTest 4/4 passed. The 13-rate packaged baseline returned zero tail peaks, worst neutral error 2.98023224e−8 FS and wet ABI/package difference 1.1712931e−38 FS. Deep strict signing verified. Package executable SHA-256 `28dbe9a2c3209da8be9746d905577ff0ea7fd101147f8c051bf467a8ed45db78`. The qualified mechanics, observer and original M5, shipping render, ABI, parameters, presets, UI and fallback have zero source diff from entry. The package regression exercises the shipping effect. [Native, Rust and package log](../../../build/stage-06/regression.log)

The frozen padded nominal mono input contains 211200 f32 frames, SHA-256 `cfc6d9342aa505dd7a53bd0ae531f717bec007042918ada9949a2117342a3d09`. Its eligible 144000 frames exclude padding. Listening ZIP SHA-256 is `39dd496030d731b3d8bbf39c15dd420de61d116f89fa1bb6b61eb6483c497712`.

The owner listening gate, instrument-specific electrical calibration and integrated nominal-limiter test are pending. Submit the specification and evidence for independent Review 6.
