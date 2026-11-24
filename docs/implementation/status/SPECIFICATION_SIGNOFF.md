# ElectroBow specification sign-off · 2026-11-24

**Implement the isolated Rust contact solver and fractional-delay ring next.** The waveguide can follow their respective oracles. A1 conditioning and A2 difference measurement can proceed separately. Production pitch selection and everything requiring dependable live Hz remain on hold. The two reported octave-down errors have a measured correction. A separate octave-up failure persists on a 110 Hz harmonic input.

Reviewed `91b58c5c1d32ee20406aac3a0ec73d478008a0d6`, on `feature/bowed-string-engine`. The tree was clean at entry. The diff from `20d4ee56` consisted solely of the three Prompt 0 status documents. The current JUCE factory, handle, processing/aliasing, rate guard and state reader were checked against the existing source. The processing chain is JUCE → eb_instrument_process → Rust Instrument. Public handles, automation IDs, EBOW v5/v1–v5 compatibility, bypass, GUI and audio-to-audio architecture continue under the baseline contract. [Processor](../../../Source/PluginProcessor.cpp#L45) · [Active ABI](../../../rust/dsp/src/ffi/instrument.rs#L45) · [Rust render](../../../rust/dsp/src/instrument/mod.rs#L83) · [Baseline reconciliation](SPECIFICATION_RECONCILIATION.md)

## Independent implementation boundaries

| Work | Authorization | Evidence needed for the next boundary |
|---|---|---|
| Rust nonlinear contact | **Proceed**, isolated S2a | M3 branch vectors, force/flux oracle, bounded lookup count and pre-commit fault status in Rust V02 |
| Fractional-delay ring | **Proceed**, independently at S2a | Read-before-write impulse/wrap and interpolation. S2b verifies circuit phase and lazy seed/FIR preload including retune against eager V03 |
| Continuously excited guide | **Proceed in isolation after contact and ring pass** | S2b pure guide driven by prescribed continuous Hz. V03–V11 clauses assigned to the guide, including nominal zero faults and decay after lift |
| Rust guitar analysis | **Proceed** with A1, decimation/clock, A2 differences/rebase, normalization, fitted-depth and raw-d measurement tests. **Hold** final candidate ordering and production pitch tracking. | Repeat frozen failed cases and the complete pitch matrix on a bounded rule. Preserve coverage, cents/false-lock budgets, causal timing and the RT budget |
| Musical articulation | **Proceed** with S4a injected frames/commands, per-lane timers, cache and sample-indexed fades. **Hold** S4b exit using raw guitar through the tracker. | Synthetic frame tests can establish state behavior. Real acquisition, musical wet launch and full trajectory claims require S3 and the guide |
| Body coloration | **Proceed** with S5 crossover parity and synthetic/captured bridge input. Live guide comparison follows S2b. | Independently measure V17 outside the feedback path and retain original fallback regression |
| Production Rust/FFI integration | **Hold** step 6 routing | Passing S2–S5 including final candidate selection, combined Rust before host, active allocation/reset tests. Keep current ABI |
| JUCE integration | **Proceed only with H** as a future separate defensive host change and native regression. **Hold** the new-sound host integration and 1.25 s tail. | H has a complete rate contract; musical host changes require S6R and the measured release |
| Final qualification | **Pending** integrated binary | S6P/7 deadline distributions, stress, real host, state replay and V22 human decisions |

The [validation dependency matrix](../ELECTROBOW_MIGRATION_PLAN.md#validation-ownership) assigns individual V01–V22 clauses to modules, fixtures, first observable boundary and blocking stage. A post-limiter waveform cannot clear a junction guard test. A pitch frame cannot clear a wet-launch test. The 21 passive V03 rate/frequency pairs had a worst one-circuit phase-delay error of .001771 samples against the .003-sample bound. The S5 audibility observations still require measured body output. S7 still needs a labeled bright/warm pickup pair, spatial stereo material, a live playing session and packaged deadline distributions. The repository DI coverage below supplies no listening verdict. [Passive phase oracle](../../../tools/validation/ring_oracle.py) · [Outstanding qualification fixtures](SPECIFICATION_RECONCILIATION.md#validation-matrix)

## A2 investigation

### Reproduction and explanation

The unmodified ledger pitch block was extracted and executed first. All 112 named stationary cases passed. It returned 200/200 and 199/199 octave errors at the two reported points, with zero rejects. A separate direct-sum experiment constructs the same six-harmonic, peak-.35 input with 100 ms of digital silence, applies the causal 30 Hz HP and normalized 32D+1 Blackman FIR, and reads host samples D−1,2D−1,… . Direct FIR and decimated convolution agreed within 1e−13 FS. At every eligible hop the squared W-sample differences were summed for all lags and normalized by the cumulative mean. An independent sliding recurrence with one rotating rebase per hop differed by at most 6.13e−14 FS² on the two tones (2.85e−16 on noise). All frequencies below use raw-d refinement and the A2 endpoint check. [Fixture and scoring contract](../DSP_VALIDATION_PLAN.md#frozen-analyzer-fixtures-and-scoring) · [Rust selection experiment](../../../tools/validation/pitch_candidates.rs)

| Host/input Hz | D, Fa, W, K, H | First scored analysis/host indices | Integer-first accepted/eligible, median / p95 absolute cents | Fitted-first accepted/eligible, median / p95 absolute cents |
|---|---|---|---|---|
| 48000 / 1230.77 | 6, 8000, 124, 125, 40 | 8839 / 53039 | 200/200, 1200.033 / 1200.636 | 200/200, .0054 / .6224 |
| 34300 / 1318.51 | 4, 8575, 132, 133, 43 | 9459 / 37839 | 199/199, 1199.847 / 1200.447 | 199/199, .9461 / .9461 |

Both periods are about 6.5 analysis samples. For a stationary sinusoidal component, the mean squared difference at lag t is proportional to 1−cos(2πft/Fa). Harmonics sharpen the dip. Integer sampling misses the first zero, while twice the period is close to integer 13. At the first scored 48 kHz hop, normalized depths at lags 6, 7 and 13 were .151746, .172693 and 4.54e−11. At 34.3 kHz they were .151997, .170011 and 3.44e−5. The old strict .15 test selects 13 before raw interpolation ever sees lag 6. The fitted lag-6 depths were .074301 and .074850. The change in rank accounts for the recovered octave.

For a dip with neighbors a,b,c, p(u)=b+(c−a)u/2+(a−2b+c)u²/2. Solving p′(u)=0 gives the minimizing offset; the fitted depth grades the trough and Q=1−depth. Raw-d interpolation independently locates its period. A2 now specifies curvature units/floors, offset and ordinate clamps, tie/order behavior, threshold equality, range tolerance, guard neighbors and rejects. Fitting the CMND ordinate before selection and using raw d for the period are from YIN step 5. The bounds and confidence policy are ElectroBow decisions. The vendored aubio implementation thresholds the integer array first. [A2](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a2-bounded-causal-pitch-and-confidence) · [YIN §II.E](https://www.ee.columbia.edu/~dpwe/e6820/papers/deChevK02-yin.pdf) · [aubio active function](../../../ThirdParty/aubio-src/src/pitch/pitchyin.c#L136)

### Coverage, error and rejected proposals

The study scored each frequency/timbre separately. The finite sweep spans every integer period in range at offsets 0, .25, .5 and .75 plus exact 65 and 1320 Hz endpoints. It includes all seven qualification rates and three nonstandard rates. The table counts initiating-depth candidates on the fixed keep-level population. Both initiating and established-track results with their own rejection counts are in the replay CSV. “Gross” is absolute error ≥600 cents. Empty accepted sets cannot pass. [Per-case checks](../../../tools/validation/check_pitch_results.py)

| Cohort | Eligible | Old accepted / gross | Fitted-first accepted / gross | Fitted-first cases outside cents/octave criteria or empty |
|---|---:|---:|---:|---:|
| 336 named sine/six/pluck cases at .03/.35 FS | 66,890 | 66,890 / 0 | 66,890 / 0 | 0/336 |
| Six harmonics, 1/k, 10-rate fractional sweep | 39,600 | 39,600 / 60 | 39,600 / 0 | 0/4,950 |
| V12 weak-fundamental weights, extended across 48 kHz sweep | 3,760 | 3,760 / 64 | 3,760 / 16 | 5/470 |
| Dominant second [.2,1,.1,.25], 48 kHz sweep | 3,760 | 3,736 / 3,561 | 3,728 / 3,576 | 454/470 |
| Six equal harmonics, zero phase, 48 kHz sweep | 3,760 | 3,760 / 125 | 3,760 / 0 | 2/470 |

The integer-sampling failure reaches down to 372.09 Hz for the bright distribution. Fitting fixes those gross errors but raw-d fine bias still gives median 12.66 and 14.51 cents at 1185.19 and 1280 Hz. At 110 Hz the V12 weak case [.05,1,.6,.35] passes (.044 cent median at .35 FS). The **different** dominant-second fixture locks at 220.05 Hz on 200/200 frames at each peak .003, .03 and .35. At the first .35 FS hop its half-period trough at 36 already has integer depth .081305 and fitted depth .079469. The true trough at 73 has integer depth .001252 and fitted depth zero. At half a period the even harmonics repeat exactly and only the odd harmonics contribute mismatch. Strong even energy makes the first trough admissible even when both candidate periods are sampled accurately. This failure also occurs near the low endpoint. At .003 FS all 200 windows on the dominant-second and weak fixtures were eligible; on the bright distribution 120/200 met the level gate and all 120 were accepted.

**Decision.** Approve fitted depth as a bounded measurement. Reject integer-first and fitted-first as complete production selectors. Choosing the deepest fitted trough globally with the same .15/.26 and raw-d/range checks was also measured: it produced 30,292 gross errors on the named cohort and 9,818 on the six-harmonic sweep (39,568 accepted). Multiples of a stable period give equally plausible or deeper minima. A production correction must distinguish early half-periods from later multiples and address fine error on bright upper-register inputs. A simple change of interpolation depth, confirmation count or an unexamined threshold increase does not establish that. No replacement ordering passed this investigation. This finding blocks final A2 and its consumers only.

On the labeled 196 Hz bend and 196/440 Hz vibrato the fitted estimator accepted 90/90, 400/400 and 400/400 frames. Emission-time RMS errors were 11.61, 9.48 and 8.40 cents; aligned RMS errors were .82, .82 and 1.65 cents at 13, 12 and 11 ms lag. The two fitted modulation rates were 5.00 Hz with depths 34.72 and 33.71 cents. On low-E plucks the first candidate in the post-.033 s scoring window was at 34.979 ms with and without an 8 ms noise pick (266/266 eligible). These are analyzer observations. The 60/80 ms wet-launch and output-trajectory requirements retain their combined-engine owners. All 10,200 scored windows across silence, clicks, white noise at both levels and separate 50/60 Hz hum produced zero start candidates even with the level gate open.

| Repository DI (44.1 kHz) | Eligible hops | Integer / fitted accepted | Fitted median Hz |
|---|---:|---:|---:|
| 1a-Low-E | 3096 | 3081 / 3081 | 82.10 |
| 4a-G | 922 | 910 / 910 | 196.05 |
| 6a-High-E | 606 | 596 / 598 | 329.00 |
| 6c-High-E-24th | 411 | 408 / 409 | 1312.14 |
| Bend-Slide G | 1326 | 1301 / 1301 | 328.82 |
| GStaccato | 1565 | 1310 / 1312 | 292.13 |
| Hammer-Pull High E | 2088 | 1889 / 1900 | 778.78 |

These recordings have no independent frame labels. Their coverage and median describe estimator behavior. Listening and tracking accuracy on a real performance require different evidence.

## Other decisions

| Finding | Approved contract and evidence | Normative changes |
|---|---|---|
| V02 | Continuity gives B=a+j, N=b+j, v=a+b+j. Impedance gives F=2Zj and r=V−v=d−j. Net wave power is 2Zjv and FV−Fv=Fr≥0. For d=.14,k=.10 the admissible static force .280 N produces r=0; applying the sliding equality gives .025 m/s error. From Slide(+) at the same load j=.054477760665, r=.085522239335 and F=.10895552133 N. 80,500 independently enumerated graph/flux checks passed, including signed sweeps, loaded knots and reversal. | M3b validation and vectors; M6 flux; V02 conditional predicates, negative/nonfinite handling, nine-segment cap and separation of Fault from audible recovery. [Math](../../architecture/BOWED_STRING_MATHEMATICS.md#m3-force-and-scattering-at-a-massless-point) |
| V08 | At fault n₀ immediately invalidate that lane, emit cached wet product and gain scaled by (N−1−i)/N for i=0…N−1, N=max(1,floor(.005fs+.5)). No second E multiplication. Sample n₀ already decrements, n₀+N−1 is zero. 44.1 and 44.15 kHz both give 221 steps, final zero offset 220. The nominal duration remains 5 ms, with nearest-sample rounding. The other lane advances through linked output normally. | A1 durations and phase, A3 Release, M6 fault procedure and V08. [V08 and ownership](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria) |
| H · extreme rate | Current prepare admits 1e−307 Hz and the tail query evaluates 1200/fs=+∞. JUCE maps an infinite seconds value to infinite tail. The first binary64 rate yielding finite seconds is 0x1.2c00000000001p−1014; its predecessor overflows. Approve a future shared JUCE-only predicate and processor regression. Preserve existing supported rates, fallback control behavior, sentinel and ABI. | A4 host contract, V20 and migration H. Required invalid prepare clears resources/output; the new positive-underflow-rate tail query returns zero. This round exercised source and arithmetic, not a modified host. [Processor](../../../Source/PluginProcessor.cpp#L194) · [A4](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a4-mix-and-host-contract) |
| Staged gates | Separate module math, raw guide, analyzer, injected controller, combined Rust, active ABI, native JUCE, packaged performance and human review. Every final criterion retains an owner. | Guide rule 6, migration steps 1–7 and validation ownership table, V18/V19 host vs ABI wording. [Migration](../ELECTROBOW_MIGRATION_PLAN.md#validation-ownership) |
| Fixture freeze | Independent clocks, waveform recipes, hashing, eligibility, lost frames and error denominators now accompany V12. Scripts reproduce both the old successes and the newly exposed failures. | A1–A2, validation measurement conventions / frozen analyzer fixtures, research YIN attribution and status ledger. [Fixtures](../DSP_VALIDATION_PLAN.md#frozen-analyzer-fixtures-and-scoring) |

## Replay and provenance

Run from the repository root with the pinned Rust toolchain and Python 3 with NumPy, SciPy and SoundFile. The run used the Prompt 0 environment (Python 3.9.6, NumPy 2.0.2, SciPy 1.13.1, SoundFile .13.1) on arm64 Apple M2 Pro. For a fresh offline-test environment create a venv and install those versions on a compatible Python; Pedalboard .9.17 is used only for the optional existing macOS packaged baseline. No plugin dependency is added.

```sh
. "$HOME/.cargo/env"
py=build/spec-reconciliation/py/bin/python
mkdir -p build/spec-signoff
rustc --edition 2024 -O tools/validation/pitch_candidates.rs -o build/spec-signoff/pitch_candidates
"$py" tools/validation/ledger_pitch.py
"$py" tools/validation/pitch_study.py
"$py" tools/validation/check_pitch_results.py --reference-hashes
"$py" tools/validation/contact_contract.py
"$py" tools/validation/ring_oracle.py
"$py" tools/validation/string_oracle.py
cargo test --workspace --locked --offline
```

The pitch study writes build/spec-signoff/pitch.csv with all three methods, both eligibility modes, timing, counts and per-case errors. The checker writes inputs.tsv with every canonical f64 input hash. The reference manifest contains 6,725 fixture entries, SHA-256 `08c856c64164aaf2e7b38f7ff01f8d476a990adc9320a6704cddca2f27acf798`. The 48 kHz and 34.3 kHz counterexample inputs hash to `5fa5f6597c85bcfeabfa92b797d205b1aeb520d64a5590f1c3de2349fcb12c00` and `18a7cab905ca791a8cd11572fc9f466288d6575ae0ce949617097bbb6ba3e135`. The 110 Hz .35 FS dominant-second fixture hashes to `c869831c9c479c97d1cacb5d86458d9019c25d08cb6dbf1a129c8f2011837fdf`. Use --reference-hashes on the reference stack to enforce the input digest. On a different math-library stack the default checker reports the digest and still enforces frame-count and numerical assertions. [Implementation ledger](IMPLEMENTATION_LEDGER.md)

**Next implementation prompt.** Implement only M3a/M3b as an isolated Rust contact module under rust/dsp/src/instrument, with ten prepared knots, per-voice branch memory, signed correction, bounded upper-root lookup and explicit Fault. Test the V02 sweep, static/reversed/flat-fold/tail/lift vectors, force residual, flux and invalid inputs. Run the locked offline Rust suite and compare against the independent contact oracle. Keep EbInstrument production routing intact. The ring and preloaded waveguide history are the next independently reviewable increment.
