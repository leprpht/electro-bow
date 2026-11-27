# Decision log

## 2026-11-27 · Commit both guide arms, loss, branch and DC as one sample

The private guide receives prescribed Hz, V and normalized k. M4 installs the seed under zero wet gain. Each tick observes the actual midpoint pickup before solving, stages both writes and commits scalar state only when both tokens exist. The M5 DC result is available at unity captured gain for the future controller. A fault returns on the detection tick after invalidating both histories. The future controller owns the cached gain and wet fade.

Integrated absolute-delay and independent evolving-history measurements passed. For V05 retain the complete post-lift spectrum, including the sub-Hz envelope of the 440 Hz tail at 5.56e−22 m/s RMS. The eager model reproduces this tail sample for sample. Report retained pitch in the first second after lift and keep the specified late decay windows. Awaiting independent Review 3. [Report, investigation and evidence](STAGE_03_WAVEGUIDE_IMPLEMENTATION.md)

## 2026-11-27 · Preserve the oldest tap through commit and measure absolute arrival

Use M2's exact capacity and expose the oldest committed sample at integer D=C. At fractional delays below C, retain both neighbors through every read. Separating validation from infallible writes lets the future lane owner discard a complete prospective tick if either arm or any other M6 check fails. M4 epoch invalidation and relaunch operate in constant time.

Measure the first nonzero cut-loop impulse arrival and the phase of its compact residual support. This fixes the cycle ambiguity of the fundamental-only principal phase probe. The actual individual Rust responses and the offline reference circuit passed their respective gates. Actual Rust circuit delay and closed-loop seed parity remain S2b obligations.

For V02, use target-prepared coefficient bits, search for an actual stored coincidence in the bounded fold neighborhood, and adjudicate the floor on the independently selected intersection using the specified binary64 operation order inside the four-ULP band. Independent enumeration, exact-mode grading, slip tolerances and contact arithmetic are preserved. Awaiting independent Review 2. [Evidence and changed files](STAGE_02_RING_IMPLEMENTATION.md)

## 2026-11-26 · Admit every representably increasing M3b load span

Review 00.1 identified a width-dependent loss of the upper root. From Slide(+) at d=.12026541114915246 and k=.12029965498642688 m/s the old search and its oracle fell back to Stick. The two stored loads are distinct and enclose d, so their linear interpolant has a sliding intersection. V02 now compares selected mode and slip independently of force balance.

Use inclusive ordered endpoints. Divide the enclosed load offset by the positive span before scaling by the knot span. On coincidence choose the upper knot at the common load. A smaller positive width cutoff would lose roots at finer scales. Re-forming loads at higher precision would define another graph and would make the choice depend on the arithmetic backend. The remaining 1e−12 slip floor and post-search static margin serve their original branch rules. Near a fold, a one-ULP change in a stored load or incident mismatch can still move slip appreciably.

The isolated Rust solve selected q=.030000277554192455 m/s in six probes on the review vector. Its independent rational intersection comparison passed 80,500 standard and 4,374 fold cases. All three faulty policies passed the physical residuals on their respective fixtures and failed branch comparison. Across 108 paired eager waveguide renders every sample retained its previous branch result at published playing points. Contact implementation awaits Review 1. [Review](REVIEW_00_1_SPECIFICATION.md) · [Mathematics](../../architecture/BOWED_STRING_MATHEMATICS.md#m3-force-and-scattering-at-a-massless-point) · [Results and replay](STAGE_01_CONTACT_IMPLEMENTATION.md)

## 2026-11-23 · Use the production instrument at 20d4ee56 as the migration baseline

The source audit pins aa1be375. Its production, test and build inputs match 20d4ee56 byte for byte. On this checkout the rebuilt ElectroBow 1.6.0 VST3 renders the fixed non-neutral pluck like its own exported eb_instrument_process, with a maximum difference of 1.18e−38 FS (rounded up). The native automation test maps every host ID against that function. Future comparisons can use the input, control settings, bundle identity and waveform hashes recorded here. [Guide, authority map and rules 1, 4, 6](../AI_IMPLEMENTATION_GUIDE.md#rules-for-implementation) · [Migration step 1, lines 5–11](../ELECTROBOW_MIGRATION_PLAN.md#1-establish-the-production-baseline) · [A4, lines 85–87](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a4-mix-and-host-contract) · [Native automation test, lines 650–696](../../../tests/FilterBankTests.cpp#L650) · [Measured baseline](IMPLEMENTATION_LEDGER.md#observed-results)

## 2026-11-23 · Withhold tracker sign-off at the upper end of the playable range

The specified A1 conditioner and A2 integer-trough rule select a period twice the input period on two reproducible six-harmonic tones. At 48 kHz/1230.77 Hz this persists on all 200 eligible stationary hops, despite the named-grid checks passing. Carry the fixture forward and seek a design decision before implementing a revised selector or claiming full-range tracking. The reconciliation records a candidate evaluation and the two gate clarifications awaiting review. No threshold, equation, host contract or gate change has been approved by this audit. [A1 conditioning/decimation, lines 27–29](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a1-input-and-time-base) · [A2 candidate selection, lines 42–46](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a2-bounded-causal-pitch-and-confidence) · [Migration step 3, lines 23–27](../ELECTROBOW_MIGRATION_PLAN.md#3-implement-the-causal-performance-measurements) · [V12, line 73](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria) · [Reproducible numerical probes](IMPLEMENTATION_LEDGER.md#replay-the-audit-probes)

## 2026-11-24 · Approve fitted trough measurement, withhold final ordering

Use the clamped CMND quadratic ordinate for depth and quality, with a separate raw-d vertex for Hz, at the A2 measurement boundary. The two reported cases changed from 399 octave-down frames to zero with identical coverage. Fitted-first still returns approximately 220 Hz for the identifiable 110 Hz dominant-second fixture. Deepest-first produced 30,292 gross errors across the 336 named cases. Both complete candidate policies are rejected for production. A bounded ordering correction and fine-error evidence are required for S3 sign-off. [A2 trough measurement](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a2-bounded-causal-pitch-and-confidence) · [YIN §II.E](https://www.ee.columbia.edu/~dpwe/e6820/papers/deChevK02-yin.pdf) · [Executed comparison and counters](SPECIFICATION_SIGNOFF.md#a2-investigation)

## 2026-11-24 · Use the friction graph to grade V02

Use signed load-line/flux/force bounds on all finite results. Grade static, slide and lift on their own constitutive branches. Invalid inputs and an impossible solve return Fault before wave commit. This preserves the friction fit, the nine-segment search and existing numerical tolerances. The .14/.10 static load violates the sliding-only equality by .025 m/s despite exact force balance. [M3b and worked vectors](../../architecture/BOWED_STRING_MATHEMATICS.md#m3-force-and-scattering-at-a-massless-point) · [M6 flux](../../architecture/BOWED_STRING_MATHEMATICS.md#m6-energy-release-and-floating-point-limits) · [V02](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria) · [Graph oracle](../../../tools/validation/contact_contract.py)

## 2026-11-24 · Count the fault tick as the first decrement

Use A1 nearest-sample durations with positive half ties upward. Invalidate the affected wave immediately. Consume exactly N steps from the cached E and E·w, ending at zero on n₀+N−1. Preserve unaffected lane progression. This resolves the 220.5-sample case without adding a holding sample. [A1](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a1-input-and-time-base) · [A3 Release](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#transition-table) · [M6](../../architecture/BOWED_STRING_MATHEMATICS.md#m6-energy-release-and-floating-point-limits) · [V08 sample checks](../../../tools/validation/contact_contract.py)

## 2026-11-24 · Authorize the future host-only overflow guard

A4 shall exclude the tiny positive binary64 interval in which 1200/fs overflows. The measured least finite-quotient rate and its adjacent predecessor distinguish the two behaviors. Preserve all ordinary fallback rates and the finite nonpositive query sentinel. H is a separate JUCE preparation/query test and correction, independent of new audio routing. [A4 host contract](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a4-mix-and-host-contract) · [V20](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria) · [Arithmetic regression](../../../tools/validation/contact_contract.py) · [Existing native lifecycle tests](../../../tests/FilterBankTests.cpp#L315)

## 2026-11-24 · Freeze clock, fixture and scoring conventions

Adopt zero-based decimation and hop phases, hand-specified spectra, labeled trajectories, level eligibility and accepted/eligible denominators. Preserve separate coverage, octave and cents results and the original numerical budgets. Publish reference input hashes and scripts so that a rejected frame cannot silently improve the score. [A1 clock](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a1-input-and-time-base) · [V12 fixture definitions](../DSP_VALIDATION_PLAN.md#frozen-analyzer-fixtures-and-scoring) · [Numerical verification](../../../tools/validation/pitch_study.py) · [Regression assertions](../../../tools/validation/check_pitch_results.py)

## 2026-11-24 · Split validation at the first observable boundary

Guide rule 6 and the migration ownership matrix now assign each clause to its first executable stage. Contact/ring use physical oracles, frame-driven state work can proceed before full guitar tracking, and bypass/tail, packaged performance and listening retain their host/release owners. V03 observes phase on the passive returning wave after one circuit, with the contact removed and phase referenced to P. This authorizes S2a immediately. [Guide rules](../AI_IMPLEMENTATION_GUIDE.md#rules-for-implementation) · [Migration steps and clause matrix](../ELECTROBOW_MIGRATION_PLAN.md#validation-ownership) · [V03 phase oracle](../../../tools/validation/ring_oracle.py) · [Sign-off evidence and stage decisions](SPECIFICATION_SIGNOFF.md#independent-implementation-boundaries)
