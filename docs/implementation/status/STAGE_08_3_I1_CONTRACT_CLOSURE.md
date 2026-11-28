# Stage 08.3 · I1 transaction contract closure

**28 November 2026. Submitted for focused independent Review 8.3.** The corrected offline Rust lane emits and grades pre-link A4, reaches exact neutral at the withdrawal deadline, and retires a corrupt phase at its owning operation. Its proposed stereo layout fits both existing budgets. The independent decision requested is whether to authorize a private, isolated Rust I1 implementation.

## Revisions and execution boundaries

Entry was `7de5fa4d54b2878ac2001db8f721a92e7c30e57a` on `feature/bowed-string-engine`, at its remote-tracking tip with clean staged, tracked and untracked status. The initial offline implementation was `05ded66471727e76541b130de7006a5fb4cf97ad`. Final executable revision `eee1a09ce36aa44ec28d650deb540b62db1443a4` adds the stereo reset entry, the A3 level-history fields and a public-reader rehash test. The handoff commit changes documentation. Qualification used Rust 1.100.0, release, `aarch64-apple-darwin`, with built-source SHA-256 `0a132bf88bd0ed4bc7d30e2f32598b4ae570e9dd92d2cedb37ec3a299d102b93`. The existing process-exec profile blocked Python in the qualification processes and their children. [Corrected producer provenance](../../../build/stage-08-3/final/i1-contract/run-3i0HhC/provenance.json)

| Boundary | Producer and independent evidence |
|---|---|
| Historical transaction | Rust loads the retained C++17 shared library through its size-checked epoch ABI. Each live call consumes fs, pitch, original M5, four q values, twelve coefficients and two eligibility flags. It emits normalized wet, corrections and monitor telemetry. The historical joined Python archive stops at wet/product/cache. The migrated Rust stereo harness additionally measures a converted mix. |
| Corrected transaction | Rust advances a separate four-bank packed epoch and one lane emission per live tick. Typed numerical errors retire only the failed lane. Actual-code replay adapters supply committed physical rows and four O2 histories. Direct, Character, signed gain, receipt and controller frame are injected. |
| Independent numerical reference | `identity::Bank` evolves its own complex states. `identity::paths::drive` fits the rectangular sample window with QR, grades all 17 candidates and reconstructs the permission and normalized wet trajectories. `identity::qr` reconstructs the packed fields, fits, residuals and masks. |
| Qualification | `stereo/closure.rs` grades actual emission and preceding cache. `deadlines.rs` executes the old and corrected recurrences. `layout/prepared.rs` owns arena offsets. `evidence_controls.rs`, `publication.rs` and the electrical aggregate consume dependent evidence. `transaction_mutants.rs` rebuilds defective sources and requires the nominated first failure. `qualification.rs` links fresh producer receipts and frozen comparisons. |

[Historical ABI](../../../tools/validation-rs/src/identity/epoch.rs) · [Historical kernel](../../../tools/validation/i1_stream.cpp) · [Frozen joined renderer](../../../tools/validation/i1_joined.py) · [Corrected candidate](../../../tools/validation-rs/src/identity/candidate.rs) · [Independent trajectory](../../../tools/validation-rs/src/identity/paths.rs)

## Findings and dispositions

| Finding | Historical reproduction / existing Rust detection | Corrected qualification |
|---|---|---|
| R82-1 | The old complete exit accepted a coherent half-product. Frozen producer streams contain 17,099 incorrect live products and caches. The first mismatch is at tick 480. No pre-link A4 field exists in those streams. | The producer returns normalized wet, signed wet, P, binary64 A4 and the checked f32. Compare these with independent source and carrier schedules on both lanes. Grade the preceding pair through the full M6 fade. |
| R82-2 | Native repeated addition misses the endpoint at 32, 44.1, 48, 96 and 176.4 kHz. Rust already measures these five misses. | Capture the origin on a target change and count ticks. All ten exercised rates land exactly on the deadline and copy the original q bits. |
| R82-3 | Historical native epoch 3,880 bytes. The 61,376 / 726,976 byte estimates group fields. Adding literal support and receipt yields 63,136 / 728,736 before reconciliation. | The proposed 64-bit arena has 449 named ranges with owner, type, count, byte offset, alignment and lifetime. The corrected compiled epoch is 3,912 bytes, including two 984-byte blocks. |
| R82-4 | The preserved probe records acceptance of O2's extra NaN and two independently loaded I1 layout snapshots containing NaN. The migrated Rust parser already rejects these tokens. | The active electrical and I1 consumers exercise those cases. Layout ingestion also checks required field types and values against the producing layout. Rehashed malformed layout evidence fails the public reader. |
| R82-5 | Native stored NaN and infinity reach PHASE. DBL_MAX reaches MOMENT after doubled-angle overflow. | Corrected stored NaN, infinity and DBL_MAX all retire at PHASE after both commits. Finite rank deficiency still yields an ordinary veto. |

[Review 8.2 counterexamples](REVIEW_08_2_I1_STREAMING_QUALIFICATION.md#findings-requiring-remediation) · [Historical reader probe](../../../build/review-08-2/evidence-probes.json) · [Corrected standalone evidence](../../../build/stage-08-3/final/i1-contract/run-3i0HhC/evidence.json)

### Emission and fault fade

Five 18,500-tick stereo runs cover idle, launch, moving pitch and depth, Hold, replacement, lift, release, a fault in either lane, completed cached fade, reset during fade and fresh acquisition. Gain includes positive and negative polarity and magnitudes .08 and 1.4. The two direct signals differ. Character takes both endpoints and automated intermediate values, including during retirement. A healthy lane's physical index and both commits advance on the same ticks as its independent oracle. For these fixtures the bank/QR normalized wet agrees bit-for-bit. The grader requires exact signed wet, product, pre-link mix, converted output and cache. The files store each converted f32 losslessly widened to f64 alongside the pre-conversion y. During an inactive or failed proposal the normalized and signed wet columns contain zero. The emitted E and P describe the pair actually mixed. Fault site, fade index and commit flags distinguish it from a successful live proposal. [Emitter](../../../tools/validation-rs/src/identity/delivery.rs) · [Grader and sample column definitions](../../../tools/validation-rs/src/identity/stereo/closure.rs)

| Actual producer boundary | 48 kHz left sample |
|---|---|
| Tick 480, live | E=.0026041666666666665, normalized wet=−.061948860458945806, signed gain=1, P=−.00016132515744517135 FS. With Character=.5078125 and direct=.2065601654138988 FS, A4=.20620508145634397 FS, converted=.2062050849199295 FS. Historical mutant P=−.00008066257872258568 FS. |
| Tick 9820, stored DBL_MAX | Epoch 2, physical index 101, both commits present. Previous E=.26041666666666663 and P=−.0364087110897791 FS. The first 239/240 decrement emits E=.2593315972222222 and P=−.03625700812690502 FS. Current direct=.04113400662517315 and Character=1 give A4=−.005790349139987494, converted=−.005790349096059799 FS. |
| Tick 10059, completed fade | Index 239 emits zero wet product, clears the cache and leaves one invalidation. Between detection and terminal zero there is no further guide/O2 commit on the retired lane. The peer follows its own uninterrupted trajectory. |
| Reset collision | At tick 9860, before either lane processes the frame, transport reset discards the partial fade and live peer. Both follow a fresh generation starting from idle. |

The expected previous P is formed from the previous scheduled E, captured gain and independent normalized wet. For every fade tick the grader scales both saved values exactly once, then uses the current direct sample and Character. It never reads the producing cache to construct the expected pair. The ordinary and damaged streams have their own schema 3 artifact digests. [Trace inventory](../../../build/stage-08-3/final/i1-contract/run-3i0HhC/receipt.json)

### Withdrawal and packed fit

At each target change capture the last realized level O. For elapsed decrement/increment k use k/N, clip the directed distance to the target, and directly assign the endpoint. Realized-veto scheduling and receipt-denial priority are unchanged. Interruptions restart from the emitted binary64 value. Monotonic rounding gives a conservative consecutive-sample bound of 1/N + 2·f64::EPSILON. The executable checks this bound, all 17 grid origins, partial reversals and recovery. The differential checks every selected target, trial and permission against the qualified schedule. [Derivation](../../architecture/ELECTROBOW_I1_SOURCE_IDENTITY_SPECIFICATION.md#permission-and-phase)

| Rate (Hz) | Revocation tick | N | First exact neutral |
|---:|---:|---:|---:|
| 32,000 | 3918 | 160 | 4077 |
| 44,100 | 5400 | 221 | 5620 |
| 48,000 | 5880 | 240 | 6119 |
| 88,200 | 10800 | 441 | 11240 |
| 96,000 | 11754 | 480 | 12233 |
| 176,400 | 21600 | 882 | 22481 |
| 192,000 | 23508 | 960 | 24467 |
| 47,001 | 5754 | 235 | 5988 |
| 32,008.333333333336 | 3918 | 160 | 4077 |
| 95,999.5 | 11754 | 480 | 12233 |

At the deadline all four q words, including negative zero, pass bit-identically. Historical 48 kHz permission was still 2.301978052621223e−15 at tick 6119. In seven packed/full-window cases the largest normalized moment error was 2.09e−15, fit error 5.45e−15 and residual error 6.16e−15. Every finite mask and scheduled trial matched. The deliberately degenerate case stayed vetoed. The 18 arithmetic probes include stored-state corruption, both genuine finite-M5 overflows, bank overflow, trial/residual and conversion failures. A separate stereo case covers stored infinity. [Packed and fault evidence](../../../build/stage-08-3/final/i1-contract/run-3i0HhC/evidence.json)

### Prepared payload and strict ingestion

The maximum is immediately below 192 kHz. The shared block is 4,672 bytes, each lane 27,760 bytes, and the exclusively borrowed prepared work arrays occupy 744 bytes. Analysis/control totals **60,936 bytes**, leaving 4,600 under its ceiling. Physical rings and host scratch occupy 655,360 bytes and explicit guide/stream descriptors occupy 1,088. Total **717,384 bytes** leaves 121,476 under the stereo ceiling. Literal support and receipt storage, tagged completed results and the captured slew origin have named owners. Replacing generic reservations with named fields and reusing the work arrays across serial lanes accounts for the smaller total. The rate scan covers both sides of every decimator change. Within an interval the ring and FIR capacities are monotone. The proposed temporary values occupy 424 bytes. The offline renderer allocates carrier, oracle and trace buffers. Production stack frames and allocator overhead require measurement against an actual stream implementation. [Byte map and lifetimes](../../architecture/ELECTROBOW_I1_SOURCE_IDENTITY_SPECIFICATION.md#prepared-stereo-layout) · [Executable field checker](../../../tools/validation-rs/src/identity/layout/prepared.rs)

Strict parsing occurs before comparing JSON values. The inspector reopens receipts and provenance, hashes the actual inventory, checks run/suite/result ownership and revalidates the two layout contracts. The complete exit also reads its run-local electrical aggregate and compares it with the strict child measurements. O2 rejects an extra top-level NaN. Both I1 layout readers reject NaN, Infinity, −Infinity, 1e999, malformed syntax, duplicate results and a string substituted for the byte count. The integration probe rewrites the current and terminal receipt digests to match each damaged file, so a hash mismatch cannot earn the parser rejection. Valid optional allocation measurements remain null. [Consumer checks](../../../tools/validation-rs/src/identity/evidence_controls.rs) · [Public-reader rehash test](../../../tools/validation-rs/tests/publication.rs) · [Publication test run](../../../build/stage-08-3/final-cargo.log)

### Compiled defective implementations

The mutation driver rebuilt each edited producer, ran its CLI and required exit 1 with the named behavioral predicate. The two complete mutations also verified the failed parent receipt and its failed i1-contract child. Neither the compiler result nor an artifact digest mismatch can satisfy these controls.

| Source edit | First failed predicate |
|---|---|
| P = 0.5 × E × w | `P == E*(signed_gain*wet)`, complete parent `run-9YPE5R`, child `run-qhOuno` |
| Repeated binary64 accumulation | `exact neutral on Nth decrement`, complete parent `run-oGdOUe`, child `run-HTPFeu` |
| Omit gain or discard its sign | `signed gain times normalized wet` in each executable |
| Omit Character on the wet mix term | `live A4 mix` |
| Multiply the live product by E twice | `P == E*(signed_gain*wet)` |
| Scale the cached wet term again in A4 | `cached-fade A4 mix` |

The nine earlier compiled transaction/source controls also retain their prior first failures, including the historical half-product at joined column 10. The independent frozen signed comparison identifies the 17,099 affected samples even though that old complete producer reported success. [All 16 edits, executable digests and receipts](../../../build/stage-08-3/final/qualification/run-LV9vbA/children/i1-transaction-mutants/run-jeQkQv/mutations.json)

## Historical equivalence and intentional differences

Four dispositions apply to the complete exit.

| Disposition | Evidence rule |
|---|---|
| Historical equivalence | Run the retained native epoch, frozen joined producer streams, capture clocks, carrier witnesses and original artifact gate. Preserve all sample hashes and the existing comparisons. |
| Expected intentional correction | A captured-origin ramp changes binary64 scale during some transitions. For every stationary and moving tick, compare the new scale to a separate clipped-distance calculation driven by the qualified target clock. Require each corrected wet word to equal the corresponding q plus corrected scale times qualified d, with the exact q-copy branch at zero. DBL_MAX changes its site from MOMENT to PHASE at the same post-commit fault boundary. |
| New candidate qualification | The signed and pre-link sample stream has a new artifact identity. Every value is graded at its producer, including the live cache proposal and each emitted cached pair. The added A3 level ring, receipt and result fields have explicit lifetimes and offsets. |
| Unexpected divergence | A changed bank correction, phase, trial index, target, gate or physical return fails its predicate. An unaccounted historical artifact difference fails complete qualification. |

The canonical historical Stage 08 ZIP remains `90222fb6bd1590713f287527607771d1db7d1f36122da5d0020bad70d96c9809`. The sole authorized Rust counterpart remains `0f6a5e212c3d751702ae6e3c16bf8d43db45fc1095b2ef1d05e3a9c42dd5a89c`. Its techs-P2-60-held gain is 0.48111582544669923 versus the historical 0.4811158254466993, one binary64 ULP lower. The 17-member archive checks, exact 14 WAVs and other six gains retain their existing predicates. Stage 08.2 and O2 retain exact archive equality. [V1.1 disposition](REVIEW_V1_1_RUST_VALIDATION_CLOSURE.md#complete-qualification)

## Numerical regression and listening evidence

All 108 corrected stationary V17 depth rows pass. The nine historical unsafe cases still fail their level/coloration or fundamental predicates. Across the 36 stationary schedules the corrected slew changed 6,859 binary64 wet words. Across the 27 motion/stress schedules it changed 3,269. All of those changes match the independent reconstruction from the qualified targets and bank. **Zero normalized wet words changed after binary32 conversion** in either inventory. The existing engineering/listening excerpts retain the historical archive checks. The five new stereo transactions supply engineering output at the newly qualified signed A4 boundary. [Stationary measurements](../../../build/stage-08-3/final/qualification/run-LV9vbA/children/i1-stationary/current.json) · [Motion measurements](../../../build/stage-08-3/final/qualification/run-LV9vbA/children/i1-motion/current.json)

The seven-rate joined capture and delivery run scored 100/100 cells at each rate on both off and actual delivery, for **700/700 on each path**. It preserved the registered fixed-bin stationary denominator, physical indices and the realized veto/selection schedule. The original expiry/priority controls, independent fourth-bank tests, finite-rank vetoes, numerical fault retirements, stereo M6 and recording-integrity controls ran in their original suites. The frozen joined comparison checked 62,088 fields and the historical signed comparison reproduced the 17,099 wrong products. [Registered motion](../../../build/stage-08-3/final/qualification/run-LV9vbA/children/i1-joined-motion/current.json) · [Frozen joined comparison](../../../build/stage-08-3/final/qualification/run-LV9vbA/children/i1-stream-compare/current.json) · [Historical signed comparison](../../../build/stage-08-3/final/qualification/run-LV9vbA/children/signed-compare/current.json)

The standalone O2 exit ran seven producers, rejected all 54 electrical controls and passed receipt reinspection. A copied passing O2 aggregate with an extra top-level NaN in its electrical report and coherently rehashed parent receipts exited 1 in the public inspector at the JSON parser. [O2 receipt](../../../build/stage-08-3/final/o2-qualification/run-Y6aFCO/receipt.json) · [Rehashed electrical probe](../../../build/stage-08-3/final-o2-corrupt-reader.json)

Native Release passed all four CTests, strict bundle signature verification and the VST3 factory plus nineteen public ABI export checks. Both Cargo workspaces passed formatting, strict all-target Clippy and locked offline debug and release tests. Each profile passed 89 production and nine validation tests. The separate packaged recorded-audition test passed with the frozen eight PCM24 containers and exact saved-state recall at the two alternate block sizes. [Cargo log](../../../build/stage-08-3/final-cargo.log) · [Packaged report](../../../build/stage-08-3/final/package/run-Gglhg5/evidence.json) · [Recorded audition](../../../build/stage-08-3/final-packaged-audition.log)

### Complete exits

| Final release command | Passed run | Result |
|---|---|---|
| i1-contract | `run-3i0HhC`, 2.32 s | Five stereo emissions, ten deadline rates, seven QR cases, 18 arithmetic cases and the arena assertions. |
| o2-qualification | `run-Y6aFCO`, 220.00 s | Seven fresh electrical producers and 54 rejected controls. |
| regression | `run-tpYoUs`, 945.97 s | All 44 fresh producers passed, including native Release and the packaged plugin. |
| qualification with frozen reference | `run-LV9vbA`, 1,000.83 s | All 44 producers and 14 frozen comparisons passed. The unresolved artifact map is empty. |

The complete artifact inventory has 105 entries. Seventy-three match byte-for-byte, 31 retain the measured fields with their actual invocation-local renderer identity, and one has the pinned Stage 08 archive disposition above. All 40,350 historical pitch rows compare, including the existing A2 misses. The exit verifies fifteen I1 producer links, all sixteen historical report/waveform negative controls, actual mutation execution, clock and capture expiry, the original source-recording checks, and the corrected output predicates. Native/package testing measured a worst neutral difference of 2.98023224e−8 FS, a wet ABI/plugin difference of 1.17129312e−38 FS and zero saved-state partition error. [Qualified comparisons](../../../build/stage-08-3/final/qualification/run-LV9vbA/comparisons.json) · [Inspected complete receipt](../../../build/stage-08-3/final-qualification-inspect.json) · [Inspected regression](../../../build/stage-08-3/final-regression-inspect.json) · [Inspected O2](../../../build/stage-08-3/final-o2-qualification-inspect.json) · [Recorded WAV and recall receipt](../../../build/validation-migration/packaged-audition/run-wtGRjp/comparison.json)

Two preliminary native-host invocations encountered the shared local CMake cache during checkout preparation and concurrent configuration. The verification checkout reused byte-identical native source files, and the final aggregate runs serialized the native/package configurations. The final receipts above are the successful runs for the submitted executable revision. Their elapsed times describe the offline test harness.

From the repository root with the configured Release package and existing comparison data, run the following commands sequentially. The macOS review runs also wrapped the executable in `sandbox-exec -f build/review-v1/no-python.sb`.

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export ELECTROBOW_VALIDATION_OUT=build/stage-08-3/final
m=tools/validation-rs/Cargo.toml
cargo build --manifest-path "$m" --release --locked --offline
v=tools/validation-rs/target/release/electrobow-validation
"$v" i1-contract
"$v" o2-qualification
"$v" regression
"$v" qualification build/validation-migration/reference
"$v" inspect o2-qualification
"$v" inspect regression
"$v" inspect qualification
cargo test --manifest-path "$m" --release --locked --offline --test packaged_recordings -- --ignored --nocapture
```

## Independent decision requested

Authorize a private isolated Rust I1 implementation only if Review 8.3 accepts these five dispositions and the complete exits below. Its implementation gate should measure actual prepared allocations, stack and callback performance, and compare its emitted stereo samples and fault ownership to this offline contract. The physical and O2 replays here establish the post-commit boundary. Actual same-call invalidation, linked output, live A2/controller receipts, controlled natural identity and owner perception retain their prior decision owners. Baseline injected S4a retains its independent authorization. Shipping processing, ABI, JUCE controls, presets, UI, fallback and limiter retain the entry trees.

Require zero heap operations in tick, reset, replacement and oversized slices. Exercise dual closure, close/trial, coincident FIR/A2 work and late faults. At 192 kHz a one-sample deadline is 5.2083 µs. Collect p50/p99/p99.9/max, miss count and harness overhead for at least 60 seconds per rate/block/activity plus ten minutes of stress. Retain the 48/192 kHz stereo mean limits of 10%/20% at 64 frames, p99.9 below half a block for B≥16, and the active-subnormal/silence median ratio ≤2. [Isolated implementation exit](REVIEW_08_2_I1_STREAMING_QUALIFICATION.md#bounded-work-and-next-rust-exit)
