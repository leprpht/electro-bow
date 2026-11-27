# Stage 08 · I1 source identity specification and offline evidence

**28 November 2026. Awaiting independent Review 8.** One coupled, carrier-driven I1 proposal passes the frozen stationary engineering tests. Safety reduces some held source differences almost to zero, and the prescribed vibrato receives neutral O2 throughout. Natural source identity, musical approval and production implementation remain held.

## Revision and scope

Entry was `ddf77bd26543e2549daca88b074de8dc473fab85`, on `feature/bowed-string-engine`, aligned with its remote-tracking branch, with empty porcelain status. Ignored historical builds and traces were present. The specification, offline tools and first replay are at `f98e9086118f56dbd9a7663f9884061348b1e7d3`. The O2 tooling revision `bcc1ea7cf668c7dee5fef76a196d660fe23c4463` binds the independently reviewed 156-stream digest inventory and checks the complete qualification disposition. The final executable revision is `ffa05af2aca3cea28db450cabca998a03aedbf5d`, including independent per-window support checks and measured pressure/identity mutations. The final handoff revision is the commit introducing this file, retrievable with `git log -1 --diff-filter=A --format=%H -- docs/implementation/status/STAGE_08_I1_SPECIFICATION.md`. No remote publication was performed.

| Changed tracked files | Purpose |
|---|---|
| `tools/validation/o2_bench.py`, `o2_coverage.py`, `o2_qualification.py`, `o2_replay.rs`, `fixtures/o2_replay_sha256.tsv` | Capture actual returned factory sample, measure/verify electrical taps, preserve reference alignment, complete evidence and failed reports. The additional TSV transcribes hashes from independent Review 7, including all four extra physical cases. |
| `tools/validation/i1_clock.py`, `i1_safety.py`, `i1_study.py`, `i1_acceptance.py`, `i1_repeatability.py` | Event oracle, bounded gain law, measurements and listening, exit inventories/mutations, controlled calibration and withheld scoring. |
| New I1 specification, acceptance matrix, source evidence and this report | Proposed architecture, frozen protocol and measured disposition. |
| Acoustic output specification, acoustic migration/validation plans, ledger and decision log | Point future work at Review 8 and preserve previous authority. |
| Stage 08 listening checksums | Reproducible asset inventory, original- and matched-gain copies and archive. |

The diff has no production Rust, C/C++, FFI, JUCE parameter, preset, UI, physical model or shipping-render edits. [I1 specification](../../architecture/ELECTROBOW_I1_SOURCE_IDENTITY_SPECIFICATION.md) · [Acceptance matrix](../I1_ACCEPTANCE_MATRIX.md) · [Source evidence](../../research/I1_SOURCE_REPEATABILITY_EVIDENCE.md)

## Review 7 maintenance handoff

| Finding | Reproducer in the maintained runner | Observed disposition |
|---|---|---|
| R7-1, omitted evidence | Remove each of local timing, high band, onset/release, inputs and replay from a complete result before grading. The independent inventory includes 11 electrical preparations, 3 depths, 165 stimulus/depth rows, 330 frequency probes, 33 full long steps, 66 lifecycle rows, 35 common-band diagnostics, 60 transient windows, 51 neutral V17 rows, 9 expected stress failures, 3 sweeps and 384 gain corners. | Each removal rejected with its missing section. Electrical values are checked against analog reference, reconstructed l/a and q, actual impulse moments and repeat Rust measurement. The acoustic aggregates are remeasured from the required hashed input taps. |
| R7-1, corrupted probes | At 48 kHz/factory set cutoff complex coordinates to 123−456i, then independently set phase to 999°, load magnitude to −100, or eligible Nyquist phase/delay to 900°/100 s. Inject the voiced bridge .5 sin(2π196t)+.6(−1)ⁿ with clean 196 Hz wet. | Each corrupt probe rejected for complex response, phase or load magnitude. The bridge row records a failed anchor, null spectral report and named reason in strict JSON. A simulated build error replaced a pre-existing successful report with neutral_candidate_passed=false, an exception reason and missing inventory. |
| R7-2, final wet | Provide off only, factory only, differently tagged traces, two jointly shifted clocks, wrong epoch, reference digest, non-depth setting or window. For the real subtraction control add .1 sin(2π6151t) to the factory wet over a unit 196 Hz sine while both uncolored reference arrays are identical. | Partial pairs fail the paired-wet requirement. Joint or individual mismatches fail comparison against the reference clock. The colored residual measures −20 dB and fails; local O2 subtraction remains −240 dB. |
| R7-3, timing | Replace the recorded terminal h of every active subnormal batch at 48 kHz with zero. The other workloads and measured durations stay intact. | Active subnormal predicate rejects. All five batches of all five workloads were present at each rate. Silence had zero h and the active workload had positive subnormal magnitude. Median subnormal/silence ratios were .9971/.9988/1.0018 at 32/48/192 kHz against the ≤2 bound. |
| R7-4, WAV provenance | Run the factory diagnostic lane and actual return-wrapper lane on the same O1 observation at every tick of the maintained physical replay. Compare q bits and write the return value. | All 156 streams completed and all 27 retained O2 streams matched their independent hashes. The original 81 WAVs and existing Stage 07 package inventory reproduce. |

The runner performed 46 named negative controls. The 15 compiled O2 mutants were also rejected. The qualified O2 production module kept its entry blob. These are submitted tooling corrections; independent acceptance remains with Review 8. [Actual O2 report](../../../build/stage-08/o2-reviewed/results.json) · [Completed data inventory and runtime controls](../../../tools/validation/o2_coverage.py) · [Interrupted-run report](../../../build/stage-08/interrupted/results.json)

## Reproduction of the blocked design

The original .30 s cosine DI with H1=.01, H2=.2, H3…12=.08/k gave 51 accepted old-clock updates. On the actual returned Rust carrier the neutral and unsafe H2/H1 were −1.324794 and +9.237247 dBr; H1/H2 requests were −4.991482/+4.991482 dB. The dominant estimate was 391.836711 Hz. The clipped requests do not equal the realized transfer.

| 48 kHz, V=.10, R=1.8 | 82.41 Hz | 196 Hz | 440 Hz |
|---|---:|---:|---:|
| Unsafe all-up level over original M5, dB | +3.633 | +3.594 | +3.532 |
| Unsafe all-down, dB | −4.823 | −5.060 | −5.335 |
| Corrected all-up, dB | +1.952 | +1.897 | +1.541 |
| Corrected all-down, dB | −2.606 | −2.751 | −2.670 |
| Corrected inversion strongest Hz | 82.4035 | 195.9184 | 439.9825 |

All three historical inversion cases fail fundamental. The two level vectors fail at each pitch, giving the original nine named failures. Their engineering-negative role and the old thresholds are preserved.

## Realized output and timing

The ray law measures a two-column energy matrix and four complex/real frequency projections from each recent eight-cycle block. It grades every proposed scale against original M5, neutral O2, dominance including residual energy and Nyquist, and the actual paired factory difference. A second set of observations monitors the output after both smoothers. Candidate admission uses tighter margins and two-window recovery. Review 8 should specifically examine how much source information that margin costs.

- Across 36 stationary cases at three depths, neutral levels spanned −2.761…−.811 dB relative to M5. Neutral H2/H1 ranged from −4.162 to −1.167 dBr. The actual 440 Hz neutral H6 and H8 were −28.90 and −35.38 dBr. No fixed margin or inversion of a spatial null was assumed.
- The normalized engineering exit passed 13 I1 negative controls and eight clock controls. The 108 corrected depth rows gave −2.816…+1.952 dB over M5. Minimum factory η was .99660. The largest factory/off residual was −36.143 dB. The corrected injected weak-H1 case gave H2/H1=−.646 dBr. The nine independent complex-impulse comparisons had worst error 2.34e−13. The largest peak in the 15 actual frozen 196 Hz electrical/correction sweeps was +2.845 dB. The seven-rate high-note RMS spreads were 1.132/1.124/1.117 dB at off/factory/full.
- At fixed identity the >6 kHz power fraction increased .0004894 between the two pressures, versus neutral .0004905. Speed contrast was +8.4713 dB versus neutral +8.4720 dB. The corrected 384-corner pre-protection peak reached 2.656 FS. S6R still owns actual stereo mix, trim, linked limiter and recovery. Lift RMS fell from .1722 FS to 2.07e−5 FS on the 82.41 Hz tail, with exact zero after the separately prescribed 100 ms release.
- The 12 s held coefficient stress remained colored after startup. The recorded bend vetoed 16.2% of windows with .00606 FS RMS correction. The 5 Hz/35-cent vibrato vetoed every window and had zero correction. The changing speed/pressure trajectory also spent roughly 75% of samples neutral. A passing stationary gate does not remove this loss of identity during motion. The imposed support outage drove the correction down and recovered under the two-observation policy. No future-transient spectral guarantee follows.
- Clock snapshots carry integer input support, doubled center, job owner, completion/due tick, published/held vector and target. Causal prefix and partition checks, including empty calls, agree exactly. Wrong owner/launch epoch, delayed completion, publication order, future input and invalid refresh are rejected. The source-clock trace joins actual sample gain/veto and wet/carrier samples to a played DI. [Clock replay](../../../build/stage-08/clock.json) · [Joined source/output clock](../../../build/stage-08/source-clock.json) · [I1 results](../../../build/stage-08/i1-results.json)

Estimated maximum input projection is 84 order/indices per lane/host tick. The conservative four-carrier layout has 96 complex section updates per lane/tick, at most one grid trial per tick during 17 scheduled ticks, a maximum 42,248 bytes for prepared stereo I1 and a rate-specific combined analysis/control estimate up to 62,608 bytes. Direct transport latency is zero by construction. Actual prepared sizes, allocation counters and callback percentiles for **production I1** are unavailable.

## Listening and remaining evidence

The two-second bend and veto transition and the same-pitch held/DI comparisons are in the compact package. Engineering WAV gain is one; matched WAVs use the documented scalar to reach .10 FS RMS on the specified anchor. DI uses [60,220) ms, the one-second held tones use the full excerpt, and moving clips use [.5,1) s. Listen to the matched DI, choose between held A/B, compare neutral, then consult the key. Attribution travels with the package. Both historical listening packages were preserved. The reproducible archive SHA-256 is `90222fb6bd1590713f287527607771d1db7d1f36122da5d0020bad70d96c9809`. No human audition, owner verdict or controlled pickup/string/guitar holdout is recorded. [Stage 08 audio](../../../build/stage-08/listening.zip) · [Listening key](../../../build/stage-08/listening-key.json) · [Repeatability results and smallest recording request](../../research/I1_SOURCE_REPEATABILITY_EVIDENCE.md)

## Commands and regression results

From repository root, on macOS arm64 with the pinned Rust toolchain and existing scientific Python environment. The existing Stage 03 guide manifest/traces and canonical WAVs are prerequisites; regenerate with `guide_qualification.py` if absent. The O1 study also reads Stage 04 observation diagnostics. Stage 08 replay outputs are ignored. The final O2 and I1 replays set `ELECTROBOW_O2_OUT=build/stage-08/o2-reviewed` with identical inputs.

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
export OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 PYTHONPATH=tools/validation
export ELECTROBOW_O2_OUT=build/stage-08/o2
py=build/spec-reconciliation/py/bin/python
mkdir -p build/stage-08

cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test --workspace --release --locked --offline
cargo test --workspace --release --locked --offline --target x86_64-apple-darwin
cargo test --release --lib isolated_read_cost --locked --offline -- --ignored --nocapture
shasum -a 256 -c tools/validation/fixtures/acoustic/SHA256SUMS
rustc --edition 2024 -O tools/validation/contact_replay.rs -o build/stage-08/contact
"$py" tools/validation/contact_contract.py --rust build/stage-08/contact
"$py" tools/validation/ring_oracle.py
"$py" tools/validation/guide_negative_controls.py
build/stage-03/guide-render --stress
"$py" -c 'import o1_qualification as o; from pathlib import Path; o.OUT=Path("build/stage-08/o1"); o.main()'
"$py" tools/validation/o1_state_parity.py
"$py" tools/validation/o2_qualification.py
"$py" tools/validation/o2_mutations.py
(cd "$ELECTROBOW_O2_OUT" && shasum -a 256 -c ../../../tools/validation/fixtures/stage07_audio_sha256.tsv)
"$py" tools/validation/i1_clock.py
"$py" tools/validation/i1_study.py
unzip -tq build/stage-08/listening.zip
(cd build/stage-08 && shasum -a 256 -c ../../tools/validation/fixtures/stage08_listening_sha256.tsv)

cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
codesign --verify --deep --strict build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
"$py" tools/validation/plugin_baseline.py
```

Formatting and strict Clippy passed. Debug, native release and Rosetta release each passed 86 Rust unit and three integration tests. The separately invoked ignored O1 timing test passed. All 25 fixture checksums passed. O1 reconstructed 152 cases and 46,242,000 observations to 4.441e−16 m/s, and 13,820 historical internal state rows matched. The guide stress completed 28.8 million ticks with 600 launches and zero faults. Contact had 84,886 finite comparisons, maximum nine probes. Ring and guide scorer controls passed. The O2 renderer covered 46,818,000 ticks, 156 physical/replay inputs and all eleven electrical rates. Native CTest passed 4/4 and the signed VST3 verified. The 13-rate plugin baseline produced zero tail peaks, worst neutral error 2.98023e−8 FS, ABI/plugin wet difference 1.17129e−38 FS. Native Intel hardware, Windows/Linux, an integrated I1 callback and owner listening were outside the available evidence. [Rust and fixture log](../../../build/stage-08/rust-fixtures.log) · [Physical checks](../../../build/stage-08/physical-regression.log) · [Package baseline](../../../build/stage-08/package.log)

**Proposed next authorization.** Submit the increment for independent Review 8 of the tooling, complex response, admissibility/clock contract, and the loss of identity under motion and in the weakest source pair. A separate decision is required before private Rust I1. Final A2, live tracking, amended gesture/S4b, supported S5, integration, host and release qualification retain their gates. Baseline injected S4a retains its independent authorization.
