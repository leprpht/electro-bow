# Prompt 0.1 specification review · 2026-11-26

**Prompt 1 is authorized to proceed with the isolated Rust nonlinear bow-contact solver.** Its inputs are the incident velocity mismatch, normal load, prepared friction curve and previous branch. All four can be supplied exactly in a unit test. One contact-specific correction is required before closing V02. A tolerance in M3b can discard the upper slip root. The independent ring may start now. The driven guide can follow passing contact and ring tests using prescribed continuous Hz. Production pitch selection and live-input integration remain blocked.

Reviewed the commit `bdc8072d2556dfc339ae864c2bd54e208b022bda` against its parent `91b58c5c1d32ee20406aac3a0ec73d478008a0d6`, from a clean worktree. The 18-file diff comprises ten documents and eight offline validation tools. This review records proposed corrections for their owners. The authoritative specifications remain at the reviewed revision.

## Findings

### BLOCKER · The proposed A2 selector picks the wrong octave and loses an entire previously accurate fixture

At 48 kHz, 110 Hz with weights [.2, 1, .1, .25] yields 220.055 Hz on all 200 eligible frames at **each** peak .003, .03 and .35 FS. The first scored hop has fitted CMND depth .079469 at lag 36, giving Q≈.921. The actual period near lag 73 has fitted depth zero. Both clear the starter threshold. Consecutive confirmation will accept the same false pitch. The published weak-fundamental 110 Hz fixture uses different weights and passes. On the weak fractional sweep, 1066.67 and 1230.77 Hz each produce 8/8 gross errors. On the bright sweep, 1185.19 and 1280 Hz have median errors of 12.66 and 14.51 cents against the 12-cent limit.

At 711.111111 Hz (48 kHz, dominant second, peak .35), integer-first accepted 8/8 with 3.37-cent median error. Fitted-first rejected 8/8 in both start and follow modes. At the first scored hop the lag-6 depth changes from .181259 to .107818. The raw fit gives 1417.09 Hz, beyond the inclusive upper limit of 1326.6 Hz. A2 discards the frame without trying the later trough. This one fixture accounts for the entire eight-frame coverage loss in that cohort. Four fitted cases have no accepted frames. Empty cases cannot satisfy V12.

**Remediation for S3 exit.** Keep both original counterexamples, the 110 Hz mixtures, the 711.111111 Hz loss, the weak and bright failures and the two modes in the regression set. Review a bounded ordering and frequency refinement using per-case coverage and truth. Recheck false starts, timing and the RT budget before changing A2, V12 or the production authorization. Deepest-first also fails, with 30,292 gross errors on the named grid. [Architecture A2, candidate ordering](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a2-bounded-causal-pitch-and-confidence) · [V12 scoring](../DSP_VALIDATION_PLAN.md#frozen-analyzer-fixtures-and-scoring) · [Rust selection](../../../tools/validation/pitch_candidates.rs#L24) · [Range check](../../../tools/validation/pitch_study.py#L169)

### MAJOR · M3b and its graph oracle discard a representably increasing slip segment

V02's **branch-specific residuals are correct**. Continuity gives `v=a+b+j`, impedance gives `F=2Zj`, and `r=V−v=d−j`. Thus the signed load line holds on every physical branch. With positive load, Stick needs zero slip and a force in the static interval. Slide needs the signed upper positive root of `q+kμ(q)=|d|`. Lift needs `j=0, r=d` and cleared memory. Expanding the two outgoing squared waves gives net flux `2Zj(a+b+j)=Fv`. Dissipation is `Fr≥0`. At `d=.14, k=.10, Z=1`, Stick gives .280 N and exactly zero slip. Testing the sliding equality there introduces a spurious .025 m/s residual. From Slide(+), the upper root is .085522239335 m/s and force is .108955521330 N. I enumerated intersections on ascending, descending and horizontal segments plus the analytic tail, then selected the largest physical slip under the previous-mode policy. It agreed with the submitted oracle on all 80,500 sweep cases, including loaded knots and both sweep directions. The separate reversal, lift, exact flat-fold and invalid-input vectors also passed.

The remaining defect is **which branch gets graded**. M3b line 54 and the oracle both skip every segment with load width ≤1e−12 m/s. Starting on Slide(+) in the following case, the upper root lies strictly inside an increasing segment. The graph's own verification accepts its erroneous Stick result. The load is attainable at V=.10 m/s and R≈1.985.

```python
from tools.validation.contact_contract import Q, MU, graph, verify
i = 3                                      # q = .02 to .04 m/s
k = (Q[i+1] - Q[i] - 5e-13) / (MU[i] - MU[i+1])
lo, hi = (Q[t] + k * MU[t] for t in (i, i+1))
d = (lo + hi) / 2
q = Q[i] + (Q[i+1] - Q[i]) * ((d - lo) / (hi - lo))
print(k, d, hi-lo, q, graph(d, k, 1), verify(d, k, 1))
```

On the reference stack, `k=.12029965498642688`, `d=.12026541114915246`, width `5.000028169277471e−13`. Interpolation between the stored f64 loads gives `q=.0300002775542` and `j≈.09026513`. The oracle returns `(j=d,r=0,Stick)`. Computing the segment load in 60-digit arithmetic from the prepared f64 knots also places the root in the interior at .03000064 m/s. The error in slip is about .03 m/s despite a tiny difference in load. Force and flux checks alone cannot distinguish these admissible static and sliding states.

**Remediation for Prompt 1.** Carry this signed regression into the same increment as the solver. Reconcile M3b with its upper-root rule by admitting distinct increasing endpoint loads enclosing the query. Compute the bounded interpolation fraction before scaling by the knot span. For coincident loads at the fold, select the upper endpoint deterministically. Check exact endpoints, both signs, previous Stick versus same-sign Slide, and the constant tail. Compare mode and reconstructed slip to an independently enumerated graph that does not share the width cutoff. Retain the ≤9 probes, finite intermediate checks and pre-commit Fault. Record the correction explicitly in M3b, V02 and its embedded reference, both oracles, and the decision/sign-off records before declaring contact parity. [M3b, lines 45–77](../../architecture/BOWED_STRING_MATHEMATICS.md#m3-force-and-scattering-at-a-massless-point) · [V02 and embedded reference](../DSP_VALIDATION_PLAN.md#executable-reference-for-the-low-e-energy-check) · [Graph oracle, lines 23–36](../../../tools/validation/contact_contract.py#L23) · [String oracle, lines 47–57](../../../tools/validation/string_oracle.py#L47)

### MAJOR · A green pitch checker can hide complete loss of maintenance coverage

The checker evaluates the named grid and six-harmonic sweep in start mode. It prints several other results and hashes the input manifest. As a negative control, I copied the fresh CSV, set every fitted follow row to zero accepted and all eligible rejected, and cleared its pitch metrics. This removed **130,915 accepted frames**. The unmodified checker still exited 0 with `--reference-hashes`. The original CSV produced the measured coverage below.

**Remediation.** In the checker, require each fixture/method/mode combination exactly once, verify eligible and rejected counts, and assert start **and** follow coverage and error gates on accepted baselines. Give the known failure fixtures explicit expected observations, including the lost 711.11 Hz case. Keep the qualification decision separate from a successful replay of expected failures. Add false-candidate and trajectory assertions for the metrics used to clear a boundary. Align the replay claims in V12 and sign-off with the assertions actually enforced. [Result checker, lines 29–98](../../../tools/validation/check_pitch_results.py#L29) · [V12 coverage contract](../DSP_VALIDATION_PLAN.md#frozen-analyzer-fixtures-and-scoring) · [Sign-off replay claims](SPECIFICATION_SIGNOFF.md#replay-and-provenance)

### MAJOR · V03's phase probe can pass a guide with zero delay

The scalar result for the specified M2 circuit is correct. Independently summing the integer delays, the two fractional phase delays and the one FIR sample gives worst error .001770987909 samples at 32 kHz / 1318.51 Hz across the 21 pairs. The executable predicate uses the principal argument of `bridge*nut*exp(iωP)` at the fundamental, where `ωP=2π`. Replacing the entire circuit transfer by the real constant .98 yields a maximum reported error below 5e−13 samples across the same pairs. The actual delay in that counterexample is zero.

**Remediation for S2b exit.** Keep the .003-sample V03 tolerance. Measure the actual Rust cut-loop response with an impulse or phase unwrapped continuously from DC at fixed delay. Establish the absolute circuit delay independently of the expected cycle count. Require a zero-delay control to fail. Extend the eager/lazy comparison to the full 100 ms of evolving waves and pickup across the downward retune. The current tap-only replay injects identical writes for 2,229 samples at 48 kHz (46.44 ms). The ring impulse weights and original guide feasibility results remain usable. [V03](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria) · [Phase calculation and history comparison](../../../tools/validation/ring_oracle.py#L24) · [Migration V03 observations](../ELECTROBOW_MIGRATION_PLAN.md#validation-ownership)

### MAJOR · Two final acceptance clauses need explicit observation points

All 22 V-numbers are present, and migration step 7 requires the full validation plan. Two matrix entries are insufficient as standalone exit tests.

| Clause and location | How a stage could pass prematurely | Required correction in the migration matrix |
|---|---|---|
| V10/V11, migration line 84 versus validation lines 71–72 | The row names full output but measures the isolated wet term. A correct internal wet signal can coexist with a disconnected wet bus or an erroneous final mix. | Keep the wave-only, candidate and wet diagnostics. Measure the delivered S6R output on the prescribed fixtures with declared controls, alignment lag, depth, RMS and launch counts. Retain V11's separate ≤6-cent model-only bound. |
| V19, migration line 98 versus validation line 80 | The active `eb_instrument_process` rejects before mutation. The legacy failed `eb_filter_bank_split` deliberately clears the valid-frame count. A test restricted to the active handle cannot observe the legacy requirement. | Name the legacy handle, failed split and subsequent valid-count query at S1/S6R alongside the active-handle no-mutation and S6H chunking checks. Keep the existing legacy regression in the release run. |

Amend the “every clause” claim in the sign-off with these assignments. The original final gates still govern release. [Migration ownership](../ELECTROBOW_MIGRATION_PLAN.md#validation-ownership) · [V10/V11 and V19](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria) · [Legacy public contract](../../../rust/dsp/include/electrobow_dsp.h#L294) · [Existing failed-split test](../../../rust/dsp/src/ffi/tests.rs#L70) · [Sign-off, staged gates](SPECIFICATION_SIGNOFF.md#other-decisions)

### MINOR · The measurement harness cannot establish rejected-frame confidence parity

With header `(rows=1,L=6,K=125)`, supply ones for every f64 difference except `d[7]=NaN` (the unused `d[0]` can be zero). Rust's `.max(0.0)` converts this NaN to zero. At Fₐ=8000 the fitted result is 1142.857 Hz with Q=1. Also, the binary returns one Q shared by both modes. At depth=.15 it emits zero start Hz, a valid follow Hz and Q=.85. Range masking is in Python and the CSV discards Q, so an out-of-range candidate's final `None,0` cannot be verified from it.

**Remediation for S3.** Exercise the final per-mode Candidate/Frame interface after all gates. Reject nonfinite differences or arithmetic before clipping. Preserve starter eligibility at depth=.15 and test zero quality for a rejected result in its evaluation mode. Add a range-failed first trough with a valid later trough to enforce the reviewed no-retry reference. Sanitized bounded audio in the replay produced finite differences. [A2 and A4 frame contracts](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a2-bounded-causal-pitch-and-confidence) · [Rust binary protocol and normalization](../../../tools/validation/pitch_candidates.rs#L61) · [Python scoring](../../../tools/validation/pitch_study.py#L168)

### MINOR · Parent and expanded counterexamples have distinct sample hashes

The parent evaluates the harmonic phase and peak normalization in a different floating-point order from `tone`. On the pinned stack the parent input hashes are `07484a8be351d56eceae1093562297af33222fe0ec8dfe7e99ae2fdfaa1afa7f` and `6b6b3890db861500320bafa9c9939983f4da43398f33d6cc6436e76f61dd57e6` (48 and 34.3 kHz). The expanded generator's hashes start `5fa5f659` and `18a7cab9`. Maximum sample differences are 1.22e−12 and 2.41e−12 FS rounded up. I fed the exact parent samples and scoring indices through the compiled selector as well. Both versions give the same decisions at all 399 hops.

**Remediation.** Label both input identities in the sign-off and fixture provenance or reuse the parent expression for byte-exact replay. Preserve the existing expanded manifest under its own identity. [Parent-equivalent generator](../../../tools/validation/ledger_pitch.py#L7) · [Expanded generator](../../../tools/validation/pitch_study.py#L44) · [Reported hashes](SPECIFICATION_SIGNOFF.md#replay-and-provenance)

## INFORMATIONAL · Measurements and contract checks

### Historical and expanded pitch replay

I executed the pitch block extracted directly from the parent ledger, compiled the reviewed Rust selector, and regenerated all 6,725 inputs, three policies and two evaluation modes in a fresh output directory. The 112 original named cases passed. Independent pair indexing on the parent samples gave the following results. Cents are absolute median / p95 over accepted frames.

| Host / input Hz | First analysis / host index | Eligible | Integer accepted / gross | Integer cents | Fitted accepted / gross | Fitted cents |
|---|---:|---:|---:|---:|---:|---:|
| 48000 / 1230.77 | 8839 / 53039 | 200 | 200 / 200 | 1200.033 / 1200.636 | 200 / 0 | .0054 / .6224 |
| 34300 / 1318.51 | 9459 / 37839 | 199 | 199 / 199 | 1199.847 / 1200.447 | 199 / 0 | .9461 / .9461 |

Both true periods are near 6.5 analysis samples. At the first scored 48 kHz hop the lag-6 integer CMND is .151746 and its fitted ordinate is .074301. The integer policy reaches lag 13 first under its threshold. At 34.3 kHz the corresponding lag-6 depths are .151997 and .074850. Direct FIR and decimated convolution agreed at 1e−13 FS absolute tolerance. The maximum direct versus rotating-rebase difference was 6.13e−14 FS² for the two tones and 2.85e−16 FS² for the noise probe. [Study windowing and recurrence](../../../tools/validation/pitch_study.py#L73)

| Expanded cohort | Eligible | Integer accepted / gross | Fitted accepted / gross | Fitted cases outside cents/gross gates or empty |
|---|---:|---:|---:|---:|
| Named grid, both levels, 336 cases | 66,890 | 66,890 / 0 | 66,890 / 0 | 0 |
| Six harmonics at 1/k, 4,950 cases | 39,600 | 39,600 / 60 | 39,600 / 0 | 0 |
| Weak fundamental, 470 cases | 3,760 | 3,760 / 64 | 3,760 / 16 | 5 |
| Dominant second, 470 cases | 3,760 | 3,736 / 3,561 | 3,728 / 3,576 | 454 |
| Six equal harmonics, 470 cases | 3,760 | 3,760 / 125 | 3,760 / 0 | 2 |

Start and follow counts agree for every labeled cohort above. The eligibility mask and input hash matched between methods per case. For example, 88,704 named score-window hops yielded 66,890 above the fixed keep floor for both methods. At .003 FS on the bright 110 Hz tone, 120/200 passed the level gate and all 120 were accepted. The original and 1/k octave improvements survive this coverage check. On the seven unlabeled DI files, starter coverage rose from 9,495 to 9,511 of 10,014 eligible hops and follow coverage from 9,619 to 9,636. Those files provide no independent pitch truth.

On the bend and two vibratos all 90/400/400 eligible frames were accepted. At emission the respective RMS errors were 11.61/9.48/8.40 cents, with aligned RMS .82/.82/1.65 cents at 13/12/11 ms. Both vibratos measured 5 Hz, with depths 34.72/33.71 cents. Silence, clicks, both hums and two noise levels gave zero starts and follows over 10,200 scored hops with the level gate open. The first accepted clean and noisy low-E estimates in the window beginning at 33 ms were at 34.979 ms. The CSV lacks the timestamp of a confirmed pair or an audible launch. S4b still owes both wet deadlines. [Frozen fixtures and scoring](../DSP_VALIDATION_PLAN.md#frozen-analyzer-fixtures-and-scoring)

### Interpolation, confidence and cost

For samples at offsets −1, 0, +1 the unique quadratic is `p(u)=b+(c−a)u/2+(a−2b+c)u²/2`. Setting its derivative to zero gives `u=(a−c)/(2C)`, `C=a−2b+c`. The Rust expression evaluates the ordinate at the clamped offset, clips depth to [0,1], and takes a separate raw-d vertex for Hz. Both fits use zero offset for curvature ≤1e−12, dimensionless for CMND and FS² for raw d. Normalized periodicity Q=1−depth describes repeatability. The 110 Hz counterexample demonstrates how high that score can be at the wrong pitch. The normalized ordinate/raw abscissa distinction agrees with the primary reference. [A2](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a2-bounded-causal-pitch-and-confidence) · [YIN §II.E](https://www.ee.columbia.edu/~dpwe/e6820/papers/deChevK02-yin.pdf#page=4)

I called the reviewed Rust functions with explicit triples. The representable predecessor of .15 starts, .15 only follows, .26 follows, and its successor rejects. Equal-depth fallback troughs select the shorter lag. A flat-bottomed dip uses its rightmost qualifying integer under the asymmetric inequalities. An entirely flat array rejects. Both boundary centers L and K−1 have guard neighbors, and a raw vertex outside ±.5 clamps there. Physical endpoints use the inclusive [64.675, 1326.6] Hz tolerance, then clamp to [65,1320]. The decimation/hop phases end at `(n+1)D−1` and `(j+1)H−1`. A decision needs W+K real observations, while the recurrence retains W+K+1 for the outgoing pair. [Rust fits and traversal](../../../tools/validation/pitch_candidates.rs#L7) · [A1–A2 clock](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a1-input-and-time-base)

For the supported rate interval D≤24, Fₐ<10000, W≤154, K≤155, H≤50. Each analysis sample updates K lag sums. A hop performs one W-pair rebase, K normalization steps and at most K−L trial positions, with constant workspace beyond the prepared arrays. The FIR has at most 769 taps per decimated sample. This is bounded and causal. The offline program allocates vectors and accepts batch input, so its runtime supplies no production callback timing or allocation evidence. V18/V21 still apply to Rust and the package. [Validation budgets](../DSP_VALIDATION_PLAN.md#budget-and-latency-accounting)

### V08 fault duration and H host-tail disposition

A1, A3 Release, M6, V08 and the decision log agree on immediate lane invalidation and N decrements. Counting from the last good sample at n₀−1, the cached gain and signed wet product are scaled once by `(N−1−i)/N` for i=0…N−1. The first fault tick already decrements. At zero, the direct coefficient is one and the wet contribution is zero. A repeated fault cannot restart the fade. I checked integer counts against exact fs/200 rounding and used non-unit cached gain and a negative product to check the mix. For 32/44.1/44.15/48/192 kHz the lengths are **160/221/221/240/960**, and first forced-zero offsets from the fault are **159/220/220/239/959**. At 44.1 kHz, 221 sample intervals from the cached sample are 5.01134 ms, while the zero occurs 4.98866 ms after detection. For N=1 the detection sample is zero. A cached zero may of course be silent earlier. These are arithmetic checks. S4a/S4b own actual split-block stereo behavior and fresh acquisition. [M6](../../architecture/BOWED_STRING_MATHEMATICS.md#m6-energy-release-and-floating-point-limits) · [V08](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria)

I linked a temporary driver against the existing Release processor test objects. For each rate I prepared a valid processor, called `setRateAndBufferSizeDetails(rate,4)`, re-prepared, queried the tail and rendered a .25 FS neutral impulse. At 1e−307 and `0x1.2c00000000000p-1014`, the current processor returned +∞ and rendered .25. The latter rate is exactly 1200/2¹⁰²⁴. At its immediate successor `0x1.2c00000000001p-1014` and the next higher rate, reported tails were finite and multiplied back to 1200 frames. The JUCE VST3 wrapper maps infinite seconds to its infinite-tail constant.

Zero and −1 returned the .30 s sentinel and cleared output. NaN, +∞ and the successor of INT_MAX/.30 returned zero tail and cleared output. The upper boundary itself rendered the impulse with a .30 s tail. Ordinary fallback and the 32/192 kHz boundaries also rendered. **Authorize H** to share the binary64 rate predicate between JUCE prepare and query, return zero tail and clear the previous instrument on the tiny positive overflow interval. Retain the separate finite-nonpositive query branch. Add these adjacent-rate and valid→invalid lifecycle tests to the native suite. I also compiled the lower-bound predicate into an ignored copy of the processor and linked it to the same native driver. All 15 rates produced the expected tail and impulse result. The two formerly overflowing rates returned zero and cleared the impulse. The packaged baseline was reused throughout. The 1.25 s musical tail belongs to S6H. [Processor prepare](../../../Source/PluginProcessor.cpp#L45) · [Processor tail](../../../Source/PluginProcessor.cpp#L194) · [A4 and V20 disposition](../../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md#a4-mix-and-host-contract) · [Native test entry point](../../../tests/FilterBankTests.cpp#L315)

### Final criteria, production surface and stage independence

I compared the parent and reviewed V rows. Six changed. V02 separates static, slide, lift and Fault while retaining the force, work, sweep and cost requirements. V03 identifies the cut-loop measurement. V08 resolves the timing origin and rounding. V18 assigns bypass and host capacity to JUCE. V19 names the active/legacy semantics. V20 adds H's edge cases. The other sixteen rows retain their original acceptance text. The following boundaries, together with the two matrix corrections above, preserve final acceptance.

| Final rows | Evidence still required on the implementing graph |
|---|---|
| V01–V07 | Neutral, aliases, bypass, IDs and recall on their public surfaces. Contact/absolute delay/seed parity, active spectra, branch duty, energy after lift, pressure and speed on the uncolored guide. |
| V08–V11 | Pre-commit guards, finite stereo output and limiter duty at all control corners. Long stationarity and DC, then actual silence expiry. Model-only and delivered bends/vibrato, confirmed legato and one faded replacement. |
| V12–V16 | Direct sums, frame coverage and labeled pitch. Real 60/80 ms launches, false starts, post-note feedback, hold/mute/tail, repicks, automation, polarity, dual-channel tracking and chord fallback at the applicable boundary. |
| V17–V20 | Body gain, harmonics, scatter, alias checks and control audibility. Rust allocation/reset and denormal gates, native capacity and bypass, partition equivalence, both ABIs, fallback behavior, cross-rate RMS/timing and failed prepare. |
| V21–V22 | Packaged replay/recall, latency and actual tail, CPU, memory, deadline distributions and stress. All three human comparisons including live, pickup and spatial stereo material after numerical qualification. |

[Original thresholds and listening requirements](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria) · [Stage ownership and final release gate](../ELECTROBOW_MIGRATION_PLAN.md#validation-ownership)

The Git diff outside `docs/` and `tools/validation/` is empty even from shipping pin `aa1be375`. I traced the JUCE render into `eb_instrument_process`, which validates dimensions, ingests inputs before borrowing output slices, then calls the existing Rust `Instrument::render`. The public header, Rust production crate, CMake, host IDs/defaults, preset reader/writer and UI have identical blobs. No study tool is linked into the plugin. The guide and A4 assign analyzer, controller, junction, delay, pickup and mixing to Rust. H is host validation in C++. Python and the standalone Rust executable are offline oracles. [Implementation guide, rules 1–4](../AI_IMPLEMENTATION_GUIDE.md#rules-for-implementation) · [Active FFI](../../../rust/dsp/src/ffi/instrument.rs#L45) · [Instrument](../../../rust/dsp/src/instrument/mod.rs) · [JUCE control IDs](../../../Source/PluginProcessor.cpp#L17)

The locked offline Rust suite passed 59 unit and 3 integration tests. Existing native Release CTest passed 4/4, including the processor's automation, bypass and v1–v5 coverage. I reused the arm64 shipping bundle with SHA-256 `e2b752927d025a1fe3c90aa093e8bf115685858034e824f027e650d221135fa6`. The 13-rate packaged/ABI probe gave worst neutral error 2.98023224e−8 FS. The non-neutral ABI/VST3 difference was 1.1712931e−38 FS and both payload hashes matched the baseline. New-sound qualification, deadline measurements and listening remain pending. [Baseline measurements and hashes](IMPLEMENTATION_LEDGER.md#observed-results)

With prescribed Hz, V and k, the contact/guide loop closes entirely on its own incident waves. The replayed eager guide passed 90 spectral/RMS cases with minimum η=.990289. The nominal 196 Hz case had 196 slip onsets and duty .13475. The six-second low-E final RMS was .22813106 m/s driven and .00014420 m/s lifted. This verifies the physical dependency boundary even while the tracker is wrong. Rust parity, dynamic retunes, guards and long-hold tests belong to their individual stage exits. [M2–M4](../../architecture/BOWED_STRING_MATHEMATICS.md#m2-arm-routing-losses-and-fractional-phase) · [Eager string replay](../../../tools/validation/string_oracle.py#L63)

## Replay

The run used macOS 26.5.1 arm64, rustc 1.100.0, Python 3.9.6, NumPy 2.0.2, SciPy 1.13.1 and SoundFile .13.1. The fresh CSV contains 40,350 measurement rows, six per fixture. The regenerated manifest SHA-256 was `08c856c64164aaf2e7b38f7ff01f8d476a990adc9320a6704cddca2f27acf798`. Run the following from the repository root with the ledger's Python environment and pinned Rust toolchain. Review-local diagnostics, the native driver and negative-control copy were generated under ignored `build/review-00-1/`.

```sh
py=build/spec-reconciliation/py/bin/python
git show 91b58c5:docs/implementation/status/IMPLEMENTATION_LEDGER.md | sed -n '333,382p' | "$py"
mkdir -p build/spec-signoff
rustc --edition 2024 -O tools/validation/pitch_candidates.rs -o build/spec-signoff/pitch_candidates
"$py" tools/validation/pitch_study.py
"$py" tools/validation/check_pitch_results.py --reference-hashes
"$py" tools/validation/contact_contract.py
"$py" tools/validation/ring_oracle.py
"$py" tools/validation/string_oracle.py
cargo test --workspace --locked --offline
ctest --test-dir build -C Release --output-on-failure
"$py" tools/validation/plugin_baseline.py
```

## Authorization by implementation stage

| Stage | Decision | Scope or condition for exit |
|---|---|---|
| S1 shipping baseline | **Accepted** | Production identity, ABI, host and fixed render reproduced. |
| S2a contact · Prompt 1 | **Proceed now** | Isolated Rust M3a/M3b. Resolve and record the narrow increasing-segment case in this increment. Pass branch, force/flux, fault and probe-count tests before closing V02. |
| S2a fractional ring | **Proceed independently** | Rust read-before-write taps, interpolation, integer boundaries and wraps. |
| S2b driven waveguide | **Proceed after its contact and ring pass** | Prescribed Hz/V/k, absolute one-circuit timing, eager/lazy trajectories, M6 guards and guide-owned V03–V11/V20 clauses. |
| S3 analyzer | **Proceed with measurements. Hold production selection and stage exit.** | A1 and A2 clock, differences, normalization and both fits can be tested in Rust. Live pitch needs per-case accuracy, coverage, confidence and bounded work. |
| S4a injected controller | **Proceed** | Inject frames/commands for timers, event priority, sample-indexed fades and cached products. Add guide-dependent checks as the guide becomes available. |
| S4b live articulation | **Hold exit and live tracking claims** | Requires passing S2b/S3 and raw-input confirmation, launch, replacement, hold and reacquisition. |
| S5 crossover/body | **Proceed in isolation** | Existing-path parity and V17 with synthetic or captured bridge input. Use the verified S2b output for the guide comparison. |
| H defensive host guard | **Proceed independently** | Native rate setup, query and render regression under the existing sound. |
| S6R production Rust/FFI | **Hold** | Route only after S2–S5 pass their applicable gates. Exercise delivered output, both ABI contracts, allocation and reset. |
| S6H musical JUCE integration | **Hold** | Requires S6R and measured release/tail. H has its own native regression gate. |
| S6P packaged plugin | **Hold new-engine qualification** | Requires the integrated native graph, packaged replay and deadline measurements. |
| S7 release | **Hold** | Full V01–V22 matrix, rate/layout/partition coverage, stress and human listening decisions. |

**Prompt 1 is authorized to proceed.** The near-fold branch test is part of the solver's own acceptance. Tracker redesign, production routing, host-tail implementation and listening have separate owners and can follow on their own schedules.
