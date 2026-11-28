# Stage 08.1 · Moving-pitch identity and lifecycle remediation

**Awaiting independent Review 8.1.** The proposed guide-phase permission preserved the registered identity contribution during vibrato at all seven rates. The original stationary V17 gates passed on the revised 108 depth rows. Numerical faults, unsupported observations and source discrimination have separate dispositions below. Isolated Rust I1, empirical source recognition, owner perception and release qualification remain held.

## Revisions and scope

Entry was `ef0e63177ec11390a9604d46d7265a4ec39076cb` on `feature/bowed-string-engine`. Tracked, staged and untracked porcelain was empty. The entry checkout and previously reviewed ignored physical fixtures were available. The motion witness was committed separately at `1b0e4b511371de32ff1978dcad3b1cfef9aad323` before revised measurements. O2's final measured run identifies entry `c35d68b1246d5029902a9d865b932c7b23d3bcb6` and renderer SHA-256 `e02cf213a08ece491e7087d44285c45823b568f6e536555329021be6b8efe9c3`. The final qualification code revision is `c35d68b1246d5029902a9d865b932c7b23d3bcb6`. The I1 report binds the measured tool bytes and the O2 run identity. Resolve the final handoff commit with `git log -1 --format=%H -- docs/implementation/status/STAGE_08_1_I1_REMEDIATION.md`.

The changed Rust file is the private timing adapter. The entry and handoff share the Rust tree `e628b2c0c0e9f0b80be11232c5e815e25f04e5e0`, Source tree `e9e9a5eed1174e67b29da2b7257d38f308c99771` and UI tree `ad64eaeb94fbe29cebdc34d9e864a71692d454cd`. Contact, ring, guide, M4/M6, observation, electrical coefficients, shipping processing, C ABI, parameters, presets, UI and fallback retain their entry code. Source analysis and all I1 rendering in this increment are offline. [Independent entry review](REVIEW_08_I1_SPECIFICATION.md)

## R8 dispositions submitted for review

| Finding | Entry input, first failed protection and consequence | Revised evidence and proposed disposition |
|---|---|---|
| R8-1 | Replacing the factory maximum with −100 dB at 65 Hz, null width, empty peaks and zero complex error passed the scalar/threshold checks. | Replaying the actual electrical return yields −0.02329544 dB at 493.42857 Hz. Its half-power interval reaches a band edge. The complete grade rejects the substituted report at the tap-derived sweep comparison. Submit tooling correction. |
| R8-2 | A real full run interrupted at final grading persisted success without an error. The success bit had already been set. | Current UUID/revision marker, exclusive runner, unsuccessful running report and strict atomic success publication. Ordinary failure, bad anchor, serialization error, grading interrupt, interrupted rename, prior success and incomplete inventory exercised. A process exiting without cleanup leaves running; the previous UUID also rejects. Submit tooling correction. |
| R8-3 | The scorer rebuilt its clock from its caller. Rolling actual factory/off wet together and shifting all clock tags by one passed at −48.0197 dB against unshifted M5. | Replay-owned clock, original tap digests, source/control schedule and sample window now govern neutral/pair scoring. Missing sides, jointly moved caller clock, wrong source/settings/epoch/count/window and underlying factory bytes reject. Submit tooling correction. |
| R8-4 | An entire 32 kHz timing table relabeled 192 kHz passed, including an arbitrary positive subnormal. Eligibility alone supplied no invocation provenance. | Rust emits rate, batch, workload, frames, measured terminal and elapsed ns with the invocation ID. The complete grade reopens that rate's measured receipt and checks its executable and all 25 batches. Ratios at 32/48/192 kHz were 1.00679/1.01741/1.01533. Submit tooling correction. |
| R8-5 | On actual 48 kHz/196 Hz M5, three same-RMS 199 Hz carriers and +2 dB H1 request, the fixed monitor reached λ=1. Neutral and colored strongest pitch were both 199 Hz. | Neutral prerequisite retained as failed at all three fundamentals and factory η. The old correction RMS was .043369 FS; revised delivery is bit-identical to the detuned carrier and its correction RMS is zero. H2-dominant, missing/weak H1, Nyquist, invalid bridge, ineligible reference and invalid epoch also fail their own prerequisites. Submit conditional runtime contract. |
| R8-6 | The fixed rectangular-bin rule plus motion gate withheld the existing alternating held feature throughout prescribed vibrato. | One six-column host-time Gram projection, 17-point common gain ray, actual fourth electrical delivery, slew and realized-output veto. Old witness 0/100 cells per rate; revised 100/100 per rate. Submit mathematical proposal with its retrospective limits. |
| R8-7 | Old complete report accepted a coherent zero residual and −240 dB, empty harmonics, invented sweep, truncated timelines and false controls. | Freshly reconstruct the required reports, carrier and candidate sample artifacts in a second directory. Check expected inventory and sample coverage, then send forged reports and actual defective waveforms through the same exit. Wrong M5, paired wet, missing cases, normalization, bypassed slew, stale features, bad epochs/clocks and neutral motion reject. Submit evidence binding. |
| R8-8 | Pick at 0, timeout at 5760, launch at 23900 yielded 76 post-timeout publications and a nonzero held launch. Later pick/launch also overwrote a live epoch without disposal. | Typed pending/confirmed acquisition, explicit receipt expiry, separate wet epoch, zero-E disposal, new-epoch authorization, due stamps and joined delivery. Expired launch fails. Old held feature survives pending replacement until a valid disposal. Submit capture/lifecycle revision. |
| R8-9 | The old clock's fault action retired the source/voice without any sample transaction, commit fields, preceding product or other lane. | Joined chronological stereo replay injects numerical failures into prospective I1 operations after actual returned O2. Detection retains the committed fields and uses the previous finite E and E·w; one invalidation, no replay, exact terminal zero, independent right lane and fresh acquisition. Submit offline transaction evidence. |
| R8-10 | Eight WAV containers repeating one decoded performance per class passed DI discrimination and reached withheld wet scoring. | Manifest/protocol revision, original/decoded/segment digests, take IDs, alternating order, setting, common gain, chain and attack records are required. Same decoded bytes in rewrapped WAVs and identical post-onset capture with unique tails both reject before wet. Integrity correction submitted. Controlled source approval remains pending. |

The pinned entry reproducer runs the actual original grader and renderer and interrupts its final grade, then probes the surviving report. The nine historical all-up/all-down/inversion failures remain individually visible, alongside the 108 passing historical corrected rows. [Entry measurements](../../../build/stage-08-1/reproduced/entry.json) · [O2 complete report](../../../build/stage-08-1/qualified/o2/results.json) · [Publication controls](../../../build/stage-08-1/qualified/publication-controls.json)

## Motion, delivery and spectral results

The frozen witness uses the off-depth wet-minus-neutral waveform over [0.5,2.5) s. One hundred nonoverlapping 20 ms cells each need at least half of stationary correction RMS. Each rate needs at least 75 active cells and at most 60 ms consecutive inactive. The same-rate stationary reference must exceed 10⁻⁴ FS and have a qualified neutral carrier. The thresholds were fixed from the objective of retaining substantial stationary correction through ten full vibrato periods. They are engineering thresholds. [Preregistration](../I1_ACCEPTANCE_MATRIX.md#stage-081-motion-witness-registration)

| fs, kHz | Previous stationary correction RMS, FS | Revised vibrato correction RMS, full 3 s | Mean cell RMS / stationary | Old / new active cells | Longest inactive witness interval |
|---:|---:|---:|---:|---:|---:|
| 32 | .006602 | .006517 | 1.0008 | 0 / 100 | 0 ms |
| 44.1 | .006629 | .006545 | 1.0012 | 0 / 100 | 0 ms |
| 48 | .006652 | .006549 | .9984 | 0 / 100 | 0 ms |
| 88.2 | .006660 | .006568 | 1.0000 | 0 / 100 | 0 ms |
| 96 | .006665 | .006569 | .9993 | 0 / 100 | 0 ms |
| 176.4 | .006665 | .006573 | .9998 | 0 / 100 | 0 ms |
| 192 | .006667 | .006573 | .9994 | 0 / 100 | 0 ms |

Vibrato and bend had zero failed realized-monitor blocks at every rate. Fitted phase tracking on revised off wet gave 4.13–4.45 cents RMS for vibrato and 2.17–2.28 cents for bend, with zero fitted lag on the diagnostic grid. Startup permission takes two completed blocks plus the trial delay. The 48 kHz vibrato's longest full-fixture neutral interval was 3,899 samples, outside the scored motion interval.

The independent fourth returned q uses the actual O1 velocity and per-sample detail schedule. Constant factory delivery matches the factory sentinel bits. Ramps, seven-sample alternations during motion/outage and gain changes use their own bank state. The monitor records completed host-time moments for both the unslewed target and the delivered output, plus actual paired power. At zero scale both signed zeros, small and ordinary finite values survive exact passthrough, including held neutral through a retune. The old instantaneous neutralization at tick 143825 makes a −.1690313 FS correction step and fails the slew bound. [Motion arrays and measured block records](../../../build/stage-08-1/qualified/i1/motion.json) · [Transition and recording controls](../../../build/stage-08-1/qualified/i1/integrity.json)

| 48 kHz stress | Actual delivery correction RMS, FS | Max adjacent wet step across paths, FS | Whole-fixture paired residual, dB | Failed realized / total completed blocks |
|---|---:|---:|---:|---:|
| Vibrato | .006547 | .07777 | −48.39 | 0 / 73 |
| Bend | .006552 | .07930 | −48.02 | 0 / 74 |
| 196→440 Hz in 20 ms | .006382 | .07930 | −45.28 | 1 / 133 |
| 4 octaves/s to 1320 Hz | .013977 | .08725 | −40.58 | 0 / 291 |
| Speed and pressure ramps | .001972 | .06468 | −47.20 | 54 / 73 |
| Alternating gains, missing observation and recovery | .050780 | .07930 | −48.42 | 1 / 73 |
| Physical lift | .002579 | .07930 | −48.21 | 1 / 171 |
| Held identity | .006555 | .07930 | −48.00 | 0 / 73 |

The bow-control trajectory remained colored for 24.51% of samples. Its M5 monitor failed on 53 of 73 windows; the rule requests neutral through these intervals. The 20 ms retune and the outage show the latency of a retrospective veto. Recovery after the scheduled observation outage took 4,017 samples (83.69 ms). Physical lifted delivery decayed from .172666 FS to 9.45×10⁻¹¹ FS, with exact zero after the harness-only release envelope. These failures stay in the report. The qualification claim covers the registered motion witness and the response to measured contradictions. It provides no universal prospective safety or motion-color guarantee.

On the revised stationary grid the level range was −2.8156…+1.9516 dB over original M5. Minimum factory η was .99660, largest paired residual −36.1434 dB and largest measured corrected sweep +2.8455 dB. Pressure raised the high-band power fraction by .00048942 versus neutral .00049052. Speed retained 8.4713 dB of the neutral 8.4720 dB change. The largest seven-rate high-note spread was 1.1323 dB. The synthetic bright/dark held distance was 2.40595 dB. [Per-case stationary, sweep and source measurements](../../../build/stage-08-1/qualified/i1/stationary.json)

## Mathematical interpretation and transaction

With the guide-integrated phase φ, the real columns are unity, cosine/sine of φ and 2φ, and the real Nyquist alternator. Over the measured host samples solve the fixed 6×6 normal system for the q and fully overlapped d columns. The energy of q+λd is its exact 2×2 Gram quadratic. Subtracting the **full** fitted Gram energy yields squared orthogonal residual. Cauchy–Schwarz bounds correlation of the remainder with any unit orthogonal competitor. Separate group norms test fitted H1, H2 and Nyquist. The conditioning screen, negative-moment tolerance, scheduler and limits on inference are specified with the implementation interface. Original stationary V17 still observes the realized waveform with its independent one-second scorer. [Mathematics, proof, event table and proposed Rust interface](../../architecture/ELECTROBOW_I1_SOURCE_IDENTITY_SPECIFICATION.md#stage-081-permission-and-transaction-contract)

At 47,001 Hz and reset origin 19, I1 and A2 hops are 236 and 235. First raw FIR interval is [1356,2378], doubled center 3734, completion 2446 and publication 2614. Dropping either A2 endpoint 2603 or identity stamp 2614 cancels publication. In the joined reset-origin replay with support ending there, only the complete case colors the delivered wet after recovery. Both missing-stamp variants remain exactly neutral. At 48 kHz the expired acquisition and its unpublished work cannot launch at 23900. Attempts to replace epoch 1 at 8400–8402 fail with missing disposal, nonzero E or wrong old owner. A valid zero-boundary disposal allows epoch 2 at 8403. Prefix replacement and fixed/single/random/empty partitions preserve joined outputs.

The stereo oracle walks both lanes at each tick. Its physical arrays are prepared from isolated Rust launch/commit replays and their actual O2 returns. Detection applies a **logical** lane invalidation in the offline transaction. Pre-guide and post-O2 boundary failures are injected; the private Rust harness separately tests those real boundaries. I1 failures are finite-state checks exercised on actual candidate coefficients, bank state, moment inputs, wet arithmetic and float conversion range. Detection-tick fields remain available in the trace. At 48 kHz, Nf=240 and the terminal cached E and product are exactly zero at fault+239. The conversion fault at replacement tick 9911 retained E=.4973958333 and product=.0391504000 FS and reached zero at 10150. A second partition schedule reproduced the same faultful samples. Repeated faults during the cache interval do not restart the fade. The next valid launch uses fresh history. Shared source preprocessing and the healthy lane continue. [Stereo cases and physical transaction fields](../../../build/stage-08-1/qualified/i1/joined.json)

The stereo analysis/control estimate is 64,656 bytes (880 bytes below 64 KiB), and 720,016 bytes for the estimated complete prepared payload. Four banks require 96 complex section updates per lane/tick. A shared moving basis adds at most 120 statistic updates per lane/tick, a bounded 6×6 factorization with 13 RHS at window closure, five realized checks and one common-ray candidate per sample over the next 17 samples. The proposed fixed scheduling accounts for simultaneous FIRs, capture publication, stereo close, launch and fault handling. Exact Rust layout, allocator counts and B=1 callback time remain open.

An additional QR projection of actual factory q,d starting at tick 48000 on vibrato, fast retune and the maximum-rate retune agreed with the normal-equation residual at all 17 scales within 8.85×10⁻¹⁷ FS². The three Gram condition numbers were 2.025–2.111. A constant-phase degenerate basis failed the conditioning screen. [QR measurements](../../../build/stage-08-1/gram-audit.json)

## Recording and listening

No eligible controlled cell was available. The six natural player/chain examples still include a weakest held distance of .1104 dB. That result demonstrates limited transfer in a confounded pair. The smallest controlled pickup comparison remains eight documented alternating independent attacks, two calibration and two withheld per setting, one guitar/string/fret/chain and common gain. The integrity controls report pair measurements and rejection reasons before withheld rendering. [Recording contract](../../research/I1_SOURCE_REPEATABILITY_EVIDENCE.md#stage-081-controlled-cell-contract)

The nine engineering exports independently matched the source arrays after float32 conversion. Matched-copy error against the documented scalar was at most 3.154×10⁻⁸ FS. [Waveform audit](../../../build/stage-08-1/listening-audit.json)

The compact package contains neutral and revised vibrato, the previous stationary correction, revised bend, held source contrasts and a depth/veto/recovery clip. Engineering exports use unity gain. Matched exports use one scalar per clip to .10 FS RMS on the documented anchor. The archive includes the sample provenance, gains, attribution and owner protocol. SHA-256 `2a10269b8a9bcebccb2eac85a84802cbf73c376ad21e6d6122e2755fae82109d`. The original stationary package reproduced its frozen `90222fb6…` digest. No human listening, instrument calibration or controlled recognition decision was made. [Listening ZIP](../../../build/stage-08-1/qualified/i1/stage-08-1-listening.zip) · [Gain and source manifest](../../../build/stage-08-1/qualified/i1/listening-manifest.json)

## Commands and verification

Executed on arm64 macOS 26.5.1, Cargo 1.100.0, Python 3.9.6, NumPy 2.0.2 and SciPy 1.13.1. Use the existing Stage 03 manifest/physical fixtures, the pinned Rust compiler and the project's scientific Python environment. All generated qualification outputs are under ignored `build/stage-08-1/`. The original defect reproducer needs a checkout of its exact entry revision; its generated audio and reports go to a separate directory.

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export PYTHONPATH=tools/validation OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1
py=build/spec-reconciliation/py/bin/python
out=build/stage-08-1
export ELECTROBOW_O2_OUT="$out/qualified/o2" ELECTROBOW_I1_OUT="$out/qualified/i1"
git worktree add --detach "$out/entry-checkout" ef0e631
mkdir -p "$out/entry-checkout/build"
ln -s "$PWD/build/stage-03" "$out/entry-checkout/build/stage-03"
"$py" tools/validation/i1_review8_entry.py "$out/entry-checkout" "$out/reproduced"
"$py" tools/validation/o2_qualification.py
"$py" tools/validation/o2_publication_checks.py
"$py" tools/validation/i1_qualification.py
ELECTROBOW_O2_OUT="$out/o2-mutants" "$py" tools/validation/o2_mutations.py
(cd "$ELECTROBOW_O2_OUT" && shasum -a 256 -c ../../../../tools/validation/fixtures/stage07_audio_sha256.tsv)
(cd "$ELECTROBOW_I1_OUT" && shasum -a 256 -c ../../../../tools/validation/fixtures/stage08_listening_sha256.tsv)
(cd "$ELECTROBOW_I1_OUT" && shasum -a 256 -c ../../../../tools/validation/fixtures/stage081_listening_sha256.tsv)
unzip -tq "$ELECTROBOW_I1_OUT/stage-08-1-listening.zip"

cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test --workspace --release --locked --offline
cargo test --workspace --release --locked --offline --target x86_64-apple-darwin
cargo test --release --lib isolated_read_cost --locked --offline -- --ignored --nocapture
shasum -a 256 -c tools/validation/fixtures/acoustic/SHA256SUMS
rustc --edition 2024 -O tools/validation/contact_replay.rs -o "$out/contact"
"$py" tools/validation/contact_contract.py --rust "$out/contact"
"$py" tools/validation/ring_oracle.py
"$py" tools/validation/guide_negative_controls.py
build/stage-03/guide-render --stress
"$py" -c 'from pathlib import Path; import o1_qualification as o; o.OUT=Path("build/stage-08-1/o1"); o.main()'
"$py" tools/validation/o1_state_parity.py
cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
codesign --verify --deep --strict build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
"$py" tools/validation/plugin_baseline.py
```

Cargo format and strict Clippy passed. Debug, native release and translated x86_64 release each passed 86 unit and three integration tests. The isolated read-cost test passed separately. All 25 permanent acoustic checksums matched. Contact had 84,886 comparisons and at most nine probes. Ring and guide negatives passed. Guide stress covered 28.8 million ticks and 600 launches with zero faults. O1 matched 152 cases, 46,242,000 observations (worst 4.441×10⁻¹⁶ m/s) and 13,820 historical state rows. O2 matched 156 streams, 46,818,000 commits, all retained trace hashes and 81 rendered carrier WAV checksums plus the rest of the 101-entry O2 audio manifest. All 15 compiled O2 mutants rejected. The corrected O2 exit exercised 52 named negatives. Native CTest passed 4/4 and the strict signed VST3 check passed. The 13-rate shipping baseline had zero tail peaks, worst neutral difference 2.98023×10⁻⁸ FS and ABI/plugin wet difference 1.17129×10⁻³⁸ FS. [Cargo and permanent fixtures](../../../build/stage-08-1/cargo-fixtures.log) · [Carrier checks](../../../build/stage-08-1/physical.log) · [O1 parity](../../../build/stage-08-1/o1.log) · [Native and packaged baseline](../../../build/stage-08-1/native.log)

**Proposed next authorization.** independent Review 8.1 of the measured-data mathematics, motion witness, actual-depth oracle, capture/fault ownership, memory schedule and complete-exit controls. This submission does not authorize isolated Rust I1. Native Intel hardware, Windows/Linux, a production I1 callback, live controller/host integration, controlled recordings and owner audition remain outside the evidence. [Complete I1 disposition and artifact digests](../../../build/stage-08-1/qualified/i1/qualification.json)

The complete-exit comparison uses an independent invocation of the same maintained renderers, with separate output paths. It also checks the direct sample relation, authoritative carrier bytes, exact zero path, expected inventories and measured spectral gates. This is a reproducibility and evidence-binding test. Independent implementation review and Rust measurements retain their own gates.

The final O2 run `56774564d72b4badb362c6d93da45b2b` published a passed report with SHA-256 `bcd882f9c39707bee4e7ebb5de08a3b1e51621fc6240cbb76815b08bdc14d086`. I1 run `950655bb97124ee984f15331c42f67e0` referenced that invocation, matched all 105 artifacts and nine measurement reports against the fresh replay, and rejected 16 complete-exit controls. Its passed qualification report SHA-256 is `34590e8bcd7dd0af75c436ba7cb9b94448b3ff5d8210e0c66341503b1cdb50ee`.
