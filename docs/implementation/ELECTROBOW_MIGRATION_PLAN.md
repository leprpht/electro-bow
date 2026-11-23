# Ordered migration from the six-control instrument

Begin from the audited six-control `EbInstrument` graph. Each increment below has a buildable end state. The first production routing change is step 6. A temporarily unconnected module may be compiled under `#[cfg(test)]` until its consumer is ready. Keep the existing C ABI usable throughout. Sections M1–M7 refer to the [mathematics](../architecture/BOWED_STRING_MATHEMATICS.md), A1–A4 to the [control and integration contracts](../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md), and V01–V22 to the [validation cases](DSP_VALIDATION_PLAN.md).

## 1. Establish the production baseline

**Depends on** the [source audit](../research/EXISTING_DSP_AUDIT.md). **Files** `rust/dsp/src/instrument/tests.rs`, `rust/dsp/src/ffi/tests.rs`, `tests/FilterBankTests.cpp`, `tools/render_guitar_comparisons.py`. No production edits or new library.

- Render a fixed pluck, then zeros through `eb_instrument_process` and the built VST3. Capture input, wet output, controls, revision, rate, block partition and final .30 s. Run the existing 62 Rust tests and processor suite. Preserve raw DI gain. Pair the current fixture outputs with input SHA-256 in the test report; write renders into a build directory.
- Keep neutral reconstruction, valid/invalid buffer behavior, callback/reset allocation count, anti-phase stereo, six IDs and v1–v5 state recall as regressions. Record one original-graph CPU measurement with an active input. No extra live state or buffer changes.
- **Exit** when V01 and the existing suites pass, and the active production symbol is recorded. A test calling `eb_bow_process_panel` would characterize a different graph. Do not calibrate against a host render unless loading the intended binary is confirmed.

## 2. Implement the contact and passive waveguide in isolation

**Depends on 1.** Existing `rust/dsp/src/instrument/mod.rs` declares private submodules. Add `rust/dsp/src/instrument/{friction,waveguide}.rs` with local Rust tests. Preserve production render and all legacy processing.

- In `friction` precompute M3a's ten knots. Implement M3b's signed branch memory, exact linear-segment intersection, constant tail and fault response. Test force units, branch transitions and the finite residual before connecting any delay.
- In `waveguide` allocate the two histories at preparation. Implement read-before-write M2, fractional cutoff bounds, one-sample loss correction and the bridge tap. Use explicit mode for contact disabled. Include a test-only operation to initialize an impulse for passive timing. Implement M4 negative-time seed reads, two FIR initial samples and M6 guards; add exact inactive reset. Match a golden test rendering with eagerly filled histories, including the first wrap and a lengthening delay.
- Drive the isolated model from continuous Hz and physical controls offline, without analyzer or host. Capture wave velocities, force, branches and unguarded output. Check V02–V09 and model-only V10–V11. **Exit** when stable pitched output persists under continued bow work and decays after lift, independently of the body and output limiter. A passing passive delay pitch test does not clear the active pitch gate. If failures occur, inspect arm labels, extra samples, seed indexing and selection of the upper slip branch before changing coefficients.

## 3. Implement the causal performance measurements

**Depends on 1.** Add `rust/dsp/src/instrument/{analysis,pitch}.rs`, declare them in `instrument/mod.rs`. No live audio routing yet. Preserve current crossover, Attack and host meter.

- Implement A1's HP, power followers, pre-swell two-band brightness, decimation FIR, peak buckets and independent lane clocks. Prepare checked history arrays. Implement the A2 sliding difference with periodic one-lag recomputation, exact lag bounds, first admissible minimum, raw-d parabolic interpolation and quality. Return timestamped values even on unvoiced frames.
- Generate golden arrays by direct summation in test code and compare frame outputs on harmonic, noisy, bending and silent inputs. Stream the same signal with partitions of 1, 17, 127 and 1024. Measure acquisition time and independent pitch error on V10–V13 and V20. Record results on the existing Low E, high E and bend recordings.
- **Exit** when no warm-start false period, octave-folded fallback, stereo cancellation or block timestamp dependency is present in the deterministic fixtures. Exceeding the pitch budget on high strings calls for inspecting the actual trough and interpolation; exceeding it on low strings calls for examining the available period and quality. Do not hide errors by pinning frequency to an equal-tempered grid.

## 4. Implement the monophonic gesture controller

**Depends on 2 and 3.** Add `rust/dsp/src/instrument/voice.rs`, register in `instrument/mod.rs`, connect the isolated analyzer and guide in the Rust test harness. No host control or preset changes.

- Implement A3's Idle, Acquire, Bowed, Hold and Release states with absolute sample counters, two/three-frame confirmation and distinct fast mute versus slow loss behavior. Own the captured polarity, peak/dynamic gain, pending replacement and E,V,R ramps. Implement M2's Hz ramp and M4 launch once per deliberate event. A note change fades to zero before replacing history.
- Feed synthetic frame sequences with an injected onset just before a block boundary, an octave outlier, a real legato jump, a quiet decay, a strong mute and a repick at identical Hz. Then replay raw audio into the combined analyzer/guide. Plot or log event index, accepted Hz and contact-enabled state. Confirm hold expiry is measured from the last supported frame and cannot refresh from wet audio.
- **Exit** on V10–V16 with identical event times across partitions. A high-quality repeated note must open a second articulation; a single bad pitch must leave the current period alone. If quiet mutes continue for the full hold, compare to the specified ambiguity rule before adjusting thresholds. Silence should never arm the initial history.

## 5. Prepare the existing body and crossover for reuse

**Depends on 1 and 2.** Modify `rust/dsp/src/filter_bank/mod.rs`, `rust/dsp/src/instrument/body.rs` and their existing tests. Reuse the existing arithmetic and coefficient construction. Avoid a second heap-backed `FilterBank` for a single bridge sample.

- Factor a stateful one-sample crossover operation out of `BandSplit::split_channel_observing`; keep its planar storage, order of evaluation, input sanitization and chunk-end tiny-value handling. Preserve exact original-band parity. Expose independent filter histories for already finite synthesized velocity after conversion to FS.
- Refactor `Body` so frame modulation advances once, independent lane `pickup` and lane reset can be called, and legacy behavior can still run at fallback rates. On the synthesized branch use the M5 mid-band shaper, original formants, reduced scatter and .85/.15 blend. Test an impulse and a swept signal into the coloration with a fixed synthetic input. No connection to returning waves.
- **Exit** on existing filter/Mode tests and V17. Inspect neutral nulls, wet harmonic/image energy and relative body gain. If modal coloration dominates, compare isolated bridge and body captures before adjusting any bow physics.

## 6. Route analysis, string and mix through the actual plugin

**Depends on 2–5.** Modify `rust/dsp/src/instrument/mod.rs`, `rust/dsp/src/instrument/tests.rs`, `Source/PluginProcessor.cpp`, `tests/FilterBankTests.cpp`. Exercise `rust/dsp/src/ffi/instrument.rs` through the same exported ABI. Update header comments only if they describe changed sound/tail. Keep `Source/PluginProcessor.h`, layout, parameter list and editor unless a host contract demands a change.

- Prepare supported-rate lanes and per-lane raw/low/high pre-swell scratch of the advertised **chunk** capacity. Keep the old timbral path for other valid rates. Ingest all channels, then run direct Attack. For each frame process saved input and copied feature pairs, observe the frame if due, advance state, evaluate M7 and A4, and finally process linked output protection. Keep meter input, first control snapshot and JUCE dry/wet/bypass semantics. No second borrow of host inputs after writing starts.
- Update the host tail branch to 1.25 s at supported rates. Host-reported fixed latency remains zero because the direct branch and bypass are immediate. Leave v5 state bytes and v1–v4 conversion intact. A new plugin instance initialized with recalled controls should behave as a fresh instrument. No UI sliders or exposure of legacy MIDI fields.
- Update specific invalidated *production* assumptions: keep the current unpitched impulse test with direct fallback, replace its requirement for immediate resonant ringing at supported rates with causal transient/no-false-launch and held-note release tests; move active chord superposition expectations to a neutral or legacy path. Replace `testProcessor`'s tail <1 s check for supported rates. A dial test must include a confirmed pitched segment with nonzero wet gain. Keep the original assertions for compatible paths, and preserve all ABI invalid-input and bypass checks.
- **Exit** on V01 and V12–V21 through the exported C entry point and the built VST3. Run existing CTest and compare non-bypassed render with Rust fixtures at the same settings. Risks include stale analysis of post-swell data, shared reset affecting the other lane, an unbounded tail in silence, dry overwritten early, and wet-only tests accidentally hearing the direct note.

## 7. Calibrate and qualify the release

**Depends on 6.** Extend `rust/dsp/src/ffi/tests.rs`'s allocator hook, `tests/FilterBankTests.cpp`'s full-processor benchmark and `tools/render_guitar_comparisons.py`'s recorded fixtures. Add focused fixtures in `rust/dsp/src/instrument/tests.rs` or adjacent private test modules; name cases by the V-number they exercise. No change to the plugin distribution/build topology.

- Evaluate V01–V22 at all specified rates, controls, channel layouts and partitions including a note replacement in the smallest callback. Read true pre-limiter peaks, guard count, branch duty and pitch accuracy. Record maximum callback and percentiles separately from means. Exercise a stream lasting long enough to pass hold/release thousands of times. Confirm reset/process/fault scenes allocate zero bytes including deallocations.
- Render the DI set through the packaged binary, retaining dry, old version, bridge-only offline diagnostic, new full-wet and factory WAVs at original gain. Trim only in listening copies. Listen to the small set in V22. Save result metadata, error traces and failed examples with the benchmark environment. Confirm host plugin identity using a working non-neutral automation before accepting any DAW comparison.
- **Exit** when required numerical and real-time gates and listening checks pass. A failed listening comparison is a release issue even if the DSP is finite. Change one calibration dimension at a time (hold/mute thresholds, pressure ratio, contact-noise level or body blend), document the observed directional effect and rerun the affected isolated tests and full suite. If a reference fixture fails the model's regime or causal response gate, retain the prior shippable plugin while diagnosing it. Chord extraction and a new panel are separate projects.
