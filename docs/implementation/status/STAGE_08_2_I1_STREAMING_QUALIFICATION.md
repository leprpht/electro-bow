# Stage 08.2 · Bounded I1 streaming qualification

**Awaiting independent Review 8.2.** The offline sample transaction retained all 108 stationary V17 depth rows and all 700 registered vibrato cells on each of the off and actual delivery paths. Confirmation at elapsed tick 5760 expired. A poisoned phase and actual moment overflow entered the affected lane's M6 fade. The proposed prepared layout and worst-tick schedule are ready for independent inspection. Isolated Rust I1 authorization remains with the reviewer.

Entry was clean on `feature/bowed-string-engine` at `9a54cd721b604731d1af24b1092f2d7b7972ded5`. The main executable and fixture revision is `e7a42b2e30a17aa60d07dc59fd4aa351a9458f22`. Final executable revision is `2ad76cfbd8f85753816193f4f6e4d7f3e37ef2fa`, which adds fractional-rate expiry and further recovery/prefix checks. The fourth-path and numerical thresholds were written in the working-tree acceptance matrix before the replacement kernel was evaluated. The subsequent handoff commit contains the specification, plans and this record. Obtain its ID with `git log -1 --format=%H -- docs/implementation/status/STAGE_08_2_I1_STREAMING_QUALIFICATION.md`. The only Rust edit is in the offline replay adapter, which exports the actual fourth electrical return after checking its tap bits. The production Rust, Source, UI, public interface and licensed acoustic fixture trees match entry. [Review boundary](REVIEW_08_1_I1_REMEDIATION.md) · [Executable specification](../../architecture/ELECTROBOW_I1_SOURCE_IDENTITY_SPECIFICATION.md#stage-082-chronological-transaction)

The final I1 run `1daa78ef9ad04b3aae295d395b961ae7` passed against O2 run `98ef7bfce89a4d1e8c7dfbcad3fe4ab7`. Its independent output directory reproduced 11 reports and 105 evidence files. All 17 source mutants exited with their registered failure in distinct invocations. The listening fixture checks and ZIP integrity check also passed. [I1 qualification](../../../build/stage-08-2/qualified/i1/qualification.json) · [O2 qualification](../../../build/stage-08-2/qualified/o2/results.json) · [I1 audio checks](../../../build/stage-08-2/qualified/i1-audio.log) · [O2 audio checks](../../../build/stage-08-2/qualified/o2-audio.log)

## Entry counterexamples and submitted dispositions

| Finding | Entry execution and first incorrect predicate | Revised ownership or predicate |
|---|---|---|
| R8-1, R8-2 | Review 8.1 closed the sweep and single-run publication findings. | Retained producer sweep recomputation, −240 dB eligible floor, null semantics, strict JSON, running/failed/interrupted/passed lifecycle and atomic replace. All publication regressions rerun. |
| R8-3, shifted pair | Both wet arrays rolled by one with original tags passed `o2_records.check_pair`. Rolling forward gave −48.024 dB factory difference, rolling backward gave −48.020 dB. | Captured output record from the actual renderer, fixed physical record, setting, all final column hashes and exact index range. The complete scorer rejects at `synchronized wet producer samples`. |
| R8-3, M5 alias | With the same frozen Rust bytes and edited tap-11 sidecar, `producer_clock` accepted column 25 as M5. The resulting ratio was −0.004272 dB. | Reopen the 30-column trace and compare canonical column hashes. Reject at `canonical Rust tap schema`. |
| R8-4, timing | `o2_bench.measured_timing` and the complete reader accepted 32 kHz rows labeled 192 kHz, frames and ns multiplied by six, active terminal −5e−324, and a coherently rebuilt row. | Exact stdout in the invocation UUID directory, executed binary, all deterministic workload terminals and batch inventory. Substituting both TSV and report fails at `executed timing stdout`. |
| R8-4, negatives | `o2_coverage.grade_controls` accepted arbitrary nonempty strings for every control. | The complete reader re-executes defective inputs, verifies the registered failure and compares run and execution records. The substitution fails at `executed negative-control predicates`. |
| R8-5 | Review 8.1 accepted the independent neutral prerequisite. | Preserve all eight upstream negatives. Missing, expired, revoked or wrong-scope live permission withdraws the target. Runtime receipt issuance still needs the future controller. |
| R8-6, R8-7 | In the entry C++ reference, setting `result[3] = 0.0` immediately before output left all three sentinels correct. A fresh complete entry qualification passed with neutral fourth wet, run `e4ceda3d264e40fc9fe9c9702ae975ba`. | Constant off/factory/full actual wet parity, independent fourth-bank correction and direct automated-wet comparison, preregistered fourth motion and recovery. Both the original source mutant and an equivalent defect in the authoritative kernel reach behavioral rejection. |
| R8-8, expiry | At 48 kHz with pick 0, continuous supported 196 Hz, confirm plus zero-E launch at 5760 created epoch 1. In `clock_ticks` confirmation changed phase before the pending-age test. | Evaluate pre-confirm elapsed eligibility. Tick 5759 launched, 5760 and 5761 produced no commit. Timeout/reset/support loss and expiry during both job phases have direct assertions. |
| R8-8, duplicated gesture | Two `Gesture.HOLD` entries silently produced Hold in the old priority scan. | Validate cardinality before A3 priority. Duplicate HOLD, pick, confirm, dispose and launch are invalid frames. The normal physical tick continues. |
| R8-9, poisoned phase | `WetEpoch.tick` returned finite wet and stayed live with NaN phase and ordinary veto after closure. | Detect the bad stored operand at phase advance and enter lane-local recovery on that sample. |
| R8-9, reduction | 1,960 finite M5 rows of 1e153 overflowed the full-window reduction. The joined exception handler raised `unexpected transaction failure nonfinite monitor input` and stopped stereo. | Check each packed add. Detect on commit 180 for the original input. A second arrangement places the first overflowing addition on commit 1,960. Both preserve physical and O2 evidence, with independent stereo-twin parity. |
| R8-10 | Shifted 0/5/10/15-sample crops of two attacks passed entry preflight as eight takes with different decoded hashes. | Verify each cropped buffer against its original session interval and bound attack log. Chronological nonoverlap and minimum 300 ms separation reject the crops. No controlled physical cell was available. |
| Joined execution | At 48 kHz the old `WetEpoch` retained N=1960 throughout retune. On the actual fast-retune trace at 440 Hz from sample 49000 its next end was 50960, while `drive` closed at 49872. Its 17 candidates were computed together. | Both invocation shapes use one host-tick kernel. N=872 at that 440 Hz start. Trial index, timestamps, classification and delayed output are checked directly. |

The entry reproducers run the pinned modules and reopen the original producer evidence. The maintained control additionally checks the entry mutant's complete report and its actual fourth waveform. Current-run engineering evidence trusts pinned fixtures and schema, the executed producer, captured outputs and independent references. An actor replacing all of these changes the trust roots. [Pinned reproducers](../../../tools/validation/i1_review81_entry.py) · [Entry results](../../../build/stage-08-2/entry-reproduction/counterexamples.json) · [Original fourth-path mutant exit](../../../build/stage-08-2/entry-mutant/qualification.json)

## Sample order and numerical equivalence

The chronological order runs reset, typed input and elapsed eligibility, source FIR/support, confirmation, zero-E disposal/launch, bounded analysis and publication, then the physical commit, O1 and four O2 returns. The new epoch advances coefficients and four banks, guide phase and the scheduled retired candidate, applies the previously available target and computes actual wet. It accumulates prospective moments and decides the observed window after the wet sample exists. Finite conversion, product and mix checks precede wet cache publication. At any checked fault only the affected epoch retires. Stream reset during a fade clears both lanes' generation-bound work. A closing tick c first permits a changed target at c+18. [Authoritative order, types, pseudocode and dependencies](../../architecture/ELECTROBOW_I1_SOURCE_IDENTITY_SPECIFICATION.md#one-host-sample)

At 47,001 Hz, reset origin 19, the joined capture retained [1356,2378] FIR support, doubled center 3734, completion 2446 and publication/launch 2614. Removing either due stamp withheld the held correction. A running projection and a completed pending result each expired before launch. At 32,008.333333333336 Hz a binary64 intermediate had rounded the strict 120 ms interval down by one. The prepared integer-ratio ceiling admitted tick 3841 and rejected 3842. The source job advances bounded slices on the host clock. Direct full-window projection comparison covered 48 publications at 48,000 and 47,001 Hz, including the reset origin. Maximum gain difference was 9.55×10⁻¹⁵ dB. Old audible hold, early repick, fresh confirmed replacement, lift/mute and later fresh reacquisition use the same lane actor. Partition checks include single sample, fixed, random, empty, event-aligned and oversized partitions, an altered future DI suffix and a second clean lane. Replacing future committed Hz, M5, depth and fourth q left the entire earlier wet and control trace identical while changing the later output. [Source job](../../../tools/validation/i1_capture.py) · [Joined traces](../../../build/stage-08-2/qualified/i1/joined.json)

Each current/retired block contains 123 f64 words, 984 bytes. The extra 24 bytes beyond 960 are the delivery pre-slew power and qy/dy products. Original M5, all 78 projections, complete Gram and paired cross terms remain present. Cholesky and all 13 solves use the completed block. Candidate trials overlap new sample accumulation. A minimum 194-sample window permits a single retired owner and 17 trials without catch-up. Direct QR reconstructs the independent coefficients and measures discarded samples. The complete exit checks the delayed target, wet, grid index, window ownership and realized decisions against full-window evidence. For the six fitted cases the largest absolute errors were 2.55×10⁻¹⁵ for sample-normalized moments, 4.55×10⁻¹⁵ for fitted coefficients and 5.66×10⁻¹⁵ for residual power. Two inputs straddling the recovery level threshold by ±2×10⁻¹² in amplitude required exact Boolean agreement. A finite, poorly conditioned observation vetoes normally. [Packed field layout and arithmetic](../../architecture/ELECTROBOW_I1_SOURCE_IDENTITY_SPECIFICATION.md#packed-binary64-arithmetic) · [QR and fault results](../../../build/stage-08-2/qualified/i1/stream-checks.json)

## Stereo faults and retirement

All 16 arithmetic probes passed the lane-local fault checks. They cover stored/increment phase, complex state and prospective overflow, coefficients, trial overflow and nonfinite fitted coefficient, moment and completed normalization, Gram, material residual, wet, conversion, cache product and the two M5 accumulation arrangements. Invalid finite subtraction and overflow are classified at the actual checked operation. The trial's overflowing energy evaluation reaches the residual-energy guard. At 48 kHz the M5 additions failed at ticks 658 and 2438 with first commits at 479. Fade zeros occurred at 897 and 2677. The zero on the 240th emitted fade sample is inclusive.

The separate joined lifecycle checks include precommit and electrical faults, retained phase/bank state through ordinary veto, fractional cached product, repeated fault, disposal/replacement and reset during cached fade. A bank fault at the exact zero-E replacement launch retained new epoch 2 commit index 0 and emitted its zero preceding cache. A trial was canceled at tick 2442 on disposal or reset. Revocation at 7100, wrong epoch, expired and out-of-range grants kept output neutral after slew. Renewal at 8500 required recovery. The unaffected lane matched the clean execution; a genuine transport reset restarted both lanes. [Fault evidence](../../../build/stage-08-2/qualified/i1/stream-checks.json) · [Physical/electrical commit telemetry](../../../build/stage-08-2/qualified/i1/joined.json)

## Acoustic evidence

All 36 cases × three depths passed the original V17 measurements with corrected/M5 levels −2.816 to +1.952 dB, largest factory/off difference −36.143 dB and maximum high-note rate spread 1.132 dB. The nine historical unsafe rows still failed. Neutral/colored pressure high-band changes were +0.00049052/+0.00048942. Speed spans were 8.4720/8.4713 dB. Synthetic held bright/dark distance was 2.406 dB. The closest natural held pair remained 0.1104 dB. [Stationary inventory, original references and DI transfers](../../../build/stage-08-2/qualified/i1/stationary.json)

The seven-rate joined witness uses the registered 196 Hz, 5 Hz ±35-cent modulation, held alternating ±5 dB, V=.10, R=1.8 and the original [0.5,2.5) s gate. Both columns scored **100 of 100 active cells at each rate**, with zero longest neutral samples in the scoring interval and zero scored veto samples. Pitch is the existing unshifted diagnostic in cents RMS; all fitted lag estimates were zero. Peak is the maximum over the full joined delivery.

| Hz | Off correction RMS FS | Delivery correction RMS FS | Delivery peak FS | Off / delivery pitch cents | Physical commits |
|---:|---:|---:|---:|---:|---:|
| 32000 | .006609 | .006607 | .44376 | 4.168 / 4.170 | 96000 |
| 44100 | .006638 | .006636 | .43839 | 4.225 / 4.227 | 132300 |
| 48000 | .006643 | .006641 | .43935 | 4.130 / 4.131 | 144000 |
| 88200 | .006661 | .006659 | .43970 | 4.400 / 4.402 | 264600 |
| 96000 | .006661 | .006659 | .44036 | 4.395 / 4.397 | 288000 |
| 176400 | .006664 | .006662 | .44164 | 4.453 / 4.455 | 529200 |
| 192000 | .006665 | .006663 | .44191 | 4.425 / 4.427 | 576000 |

Each constant off/factory/full delivery matched the corresponding final sentinel bytes. A delivery-only contradiction caused four realized vetoes with healthy sentinels. In the gain-alternation/outage fixture permission recovered 4017 samples after the outage. Bend had no realized failures, fast retune had one, max-retune had none. Bow pressure/speed ramps still had 53 unavailable windows and 54 realized failures in 73 observations. Lift delivery RMS fell from .17267 to 9.45×10⁻¹¹ FS and the harness release reached exact zero. The compact audition includes unity engineering files, separately matched copies, anchors and gain manifest. Human listening and controlled pickup/string recognition remain pending. [Moving evidence](../../../build/stage-08-2/qualified/i1/motion.json) · [Listening archive](../../../build/stage-08-2/qualified/i1/stage-08-2-listening.zip)

## Evidence ownership and resource proposal

The O2 report retains the 156-stream carrier inventory, 46,818,000 commits, measured tap-derived sweep and nine downstream stress failures. Timing stdout is captured at process execution under the run UUID. The grader checks all five batches of five workloads per rate, exact deterministic terminal values, active subnormals and silent zeros, plus the bound of 2 on the near-denormal/silence median ratio. At the final O2 execution, the 32/48/192 kHz ratios were 1.00681, 1.00619 and 1.00721. All 54 electrical/measurement negative controls reached their registered rejection. The complete I1 exit also challenges the full O2 reader with coherent timing replacement and unexecuted negative claims. Single-run ordinary failure, grading and publication interruption, serialization failure, missing inventory and abrupt termination preserve unsuccessful status. [O2 publication regressions](../../../build/stage-08-2/qualified/publication-controls.json) · [O2 results](../../../build/stage-08-2/qualified/o2/results.json)

The I1 source-mutation suite runs the ordinary qualification entry in an isolated source copy. It puts the registered behavioral gate first, records the failed run ID, source edit, executable invocation, exit and predicate, and treats survival or an unrelated failure as a failed control. It covers 17 defective producers/scorers, including both fourth-path mutations, unsafe gain, wrong bank, slew bypass, disabled realized veto, phase/scheduling, expiry, duplicate gesture, stale publication, wrong epoch, poisoned-phase veto, stereo abort, double E on cached product, denominator and shifted pair. Fresh reports and sample digests are also compared in independent output directories. [Complete I1 run and executed mutant inventory](../../../build/stage-08-2/qualified/i1/qualification.json)

At the peak rate boundary, the proposed rate-specific rings, shared taps, fixed control/epoch state and workspace occupy **61,376 / 65,536 bytes**. Including ring, scratch and explicit stream/guide descriptor reservations gives **726,976 bytes** under the 0.8 MiB ceiling. These are exact proposed arena capacities. A future implementation still needs to measure actual prepared allocation and validate object sizes. Each live lane can execute 96 complex section updates, 48 weighted terms and 123 statistical updates. At closure add one 6×6 factorization and 13 forward/back pairs. Another lane may be closing or processing its one scheduled candidate. The capture/A2 FIR coincidence, job slices, launch, cache and fault paths are included in the specification's work bound. Integrated 192 kHz/1 performance at 5.21 µs and the remaining V21 budgets have no offline timing substitute. [Prepared offsets](../../../build/stage-08-2/qualified/i1/layout.json) · [Worst-tick schedule](../../architecture/ELECTROBOW_I1_SOURCE_IDENTITY_SPECIFICATION.md#prepared-storage-retirement-and-deadline)

## Commands and remaining authorization

From repository root, using the installed NumPy/SciPy environment and the retained qualified guide/O1 fixtures:

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export PYTHONPATH=tools/validation OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1
out=build/stage-08-2
py=build/spec-reconciliation/py/bin/python
export ELECTROBOW_O2_OUT="$out/qualified/o2" ELECTROBOW_I1_OUT="$out/qualified/i1"
"$py" tools/validation/o2_qualification.py
"$py" tools/validation/o2_publication_checks.py
"$py" tools/validation/i1_qualification.py
(cd "$ELECTROBOW_O2_OUT" && shasum -a 256 -c ../../../../tools/validation/fixtures/stage07_audio_sha256.tsv)
(cd "$ELECTROBOW_I1_OUT" && shasum -a 256 -c ../../../../tools/validation/fixtures/stage08_listening_sha256.tsv && shasum -a 256 -c ../../../../tools/validation/fixtures/stage082_listening_sha256.tsv && unzip -tq stage-08-2-listening.zip)

cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test --workspace --release --locked --offline
cargo test --workspace --release --locked --offline --target x86_64-apple-darwin
cargo test --release --lib isolated_read_cost --locked --offline -- --ignored --nocapture
shasum -a 256 -c tools/validation/fixtures/acoustic/SHA256SUMS
rustc --edition 2024 -O tools/validation/contact_replay.rs -o "$out/contact"
"$py" tools/validation/contact_contract.py --rust "$out/contact"
"$py" -c 'from pathlib import Path; import contact_mutations as m; m.OUT=Path("build/stage-08-2/contact-mutants"); m.main()'
"$py" tools/validation/ring_mutations.py
"$py" tools/validation/guide_mutations.py
"$py" tools/validation/o1_mutations.py
"$py" tools/validation/ring_oracle.py
"$py" tools/validation/guide_negative_controls.py
build/stage-03/guide-render --stress
"$py" -c 'from pathlib import Path; import o1_qualification as o; o.OUT=Path("build/stage-08-2/o1"); o.main()'
"$py" tools/validation/o1_state_parity.py
ELECTROBOW_O2_OUT="$out/qualified/o2-mutants" "$py" tools/validation/o2_mutations.py
cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
codesign --verify --deep --strict build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
"$py" tools/validation/plugin_baseline.py
```

For entry reproduction, extract `git archive 9a54cd7 tools/validation` into `$out/entry`. Copy that validation tree into `$out/entry-mutant-tools` and insert `result[3] = 0.0;` just before its existing output write in `i1_motion_bank.cpp`. Use the preserved entry O2 evidence or regenerate it using the entry tools. The original complete exit and the maintained counterexamples were run as follows.

```sh
PYTHONPATH="$out/entry-mutant-tools" ELECTROBOW_O2_OUT=build/review-08-1/o2 ELECTROBOW_I1_OUT="$out/entry-mutant" "$py" "$out/entry-mutant-tools/i1_qualification.py"
"$py" tools/validation/i1_review81_entry.py "$out/entry/tools/validation" "$out/entry-reproduction" build/review-08-1/o2 "$out/entry-mutant/qualification.json"
```

Cargo format and strict Clippy passed. Debug, native release and translated x86_64 release each passed 86 unit and three integration tests. The isolated read-cost check passed. The contact replay compared 84,886 results with at most nine probes. Guide stress recorded 600 launches and zero faults. O1 compared 46,242,000 observations over 152 cases and 13,820 retained state rows. The compiled contact, ring, guide and O1 mutation suites passed their named rejection checks. All 15 compiled O2 mutants rejected. Native CTest passed 4/4, the signed VST3 verified, and the shipping baseline retained zero tail peak, 2.980232×10⁻⁸ FS worst neutral difference and 1.171293×10⁻³⁸ FS ABI/plugin wet difference. Permanent fixture hashes were checked. [Cargo/fixture log](../../../build/stage-08-2/cargo.log) · [Carrier log](../../../build/stage-08-2/carrier.log) · [Native log](../../../build/stage-08-2/native.log)

Request independent Review 8.2 of the executable, behavioral gates, prepared layout and evidence ownership before any isolated Rust I1 work. Live A2/A3 issuance, integrated callback deadlines, native Intel hardware, Windows/Linux, controlled recording evidence and human listening remain open. Baseline injected S4a keeps the authorization granted independently of I1.
