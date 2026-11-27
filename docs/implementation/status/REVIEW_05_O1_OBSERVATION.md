# Review 5 · Independent Rust O1 spatial pickup qualification

**Decision, 27 November 2026.** Qualify the isolated Rust O1 observation, its private lifecycle and local real-time behavior, and exact preservation of the guide and M5 diagnostics. Retain S2a contact, S2a ring and S2b guide qualification. The accepted quantity is the three-station, nut-side total transverse velocity in m/s. The observation is ready for isolated use. O2 implementation remains held pending a bounded specification and evidence review. Human listening was unavailable for this review. The buzzing complaint and acoustic perceptual gate remain open.

## Revisions and inspection

| Boundary | Revision and state |
|---|---|
| Review 3 guide report and reference corpus | `29cf221ec3326a56cc84dbd474ba29f34da3df5c` |
| Review 4 authorization and Stage 05 entry | `0c98f608a0d46e29693320f36e85f173cd3faefe` |
| O1 Rust and executable observer evidence | `9342622a9144e6316af6a89b2b49250affe2be4b` |
| Entry-guide state parity probe | `027a7a6a9b71d9c8bc5e463e419053e02af49f17` |
| Stage 05 submission and Review 5 entry | `cc43f9fb4a64fdfea70bd88c25e5dbd0991c7021`, `feature/bowed-string-engine`, at the matching remote tip. Index and working tree clean before review edits. |

I read the Stage 05 handoff, Reviews 3 and 4, both architectural specifications, all three validation and migration plans named for this review, the implementation ledger and the decision log. I inspected the full 19-file entry-to-submission diff, the actual Rust guide/ring, test predicates, harnesses, eager histories, contact reference, compiled mutations, render adapters, measurements and fixture digests. Independent scripts, instrumented source copies, logs and generated evidence live under ignored `build/review-05/`. Review 5's tracked changes are this report and the implementation ledger. [Stage 05](STAGE_05_O1_OBSERVATION_IMPLEMENTATION.md) · [Review 4](REVIEW_04_ACOUSTIC_RESEARCH.md) · [Review 3](REVIEW_03_WAVEGUIDE.md) · [O1–I1 specification](../../architecture/ELECTROBOW_ACOUSTIC_OUTPUT_SPECIFICATION.md) · [M1–M7](../../architecture/BOWED_STRING_MATHEMATICS.md) · [Acoustic validation](../ACOUSTIC_VALIDATION_PLAN.md) · [Acoustic migration](../ACOUSTIC_MIGRATION_PLAN.md) · [DSP validation](../DSP_VALIDATION_PLAN.md) · [Ledger](IMPLEMENTATION_LEDGER.md) · [Decision log](DECISION_LOG.md) · [Reviewed diff](../../../build/review-05/reviewed.diff)

## Mathematical and source review

Let $s=x/L$ increase from bridge $0$ to nut $1$, with bow at $\beta=.127$. At the current tick, $P=f_s/f$ samples is a complete circuit and one string length takes $P/2$. $N[n]$ is the signed velocity emitted toward the nut. Both traveling waves use the same positive transverse direction. The rigid nut enforces zero total velocity, so it reverses the reflected velocity. The direct path to $s\ge\beta$ has length $s-\beta$. The returning path has length $(1-\beta)+(1-s)$. Thus

$$d_+= (s-\beta)P/2,\qquad d_-= (2-s-\beta)P/2,\qquad v(s,n)=I(N,n-d_+)-I(N,n-d_-).$$

Both ray fields in `Station` retain the stored sign of $N$. `velocity()` subtracts the long-path field once. The traveling velocity already has units m/s, so no differentiation, bridge FIR or electrical gain enters this observation. At the bow $d_+=0$ and $d_-=(1-\beta)P$, giving $N[n]+a[n]$, where $a$ is the guide's signed nut incident. At the nut both delays are identical and the finite tap values cancel to exactly positive zero. Both endpoint checks passed on every maintained grid tick, including the first tick of replacement epochs. [Station and aperture](../../../rust/dsp/src/instrument/waveguide.rs#L25) · [Guide recurrence](../../../rust/dsp/src/instrument/waveguide.rs#L206) · [Endpoint predicates](../../../rust/dsp/src/instrument/waveguide/o1_tests.rs#L118)

For $m=\lfloor d\rfloor$ and $u=d-m$, the neighbors are $N[n-m]$ and $N[n-m-1]$ with weights $1-u$ and $u$. After commit the write cursor represents $n+1$, so the integer lags are $m+1$ and $m+2$. The helper splits the **unshifted** binary64 $d$ first and adds one to the integer lag. Adding `1.0` in floating point before decomposition loses the original fraction for the binary64 predecessor of .5. At integer $d$, the zero-weight neighbor is skipped. At 32 kHz and 1320 Hz, the left direct path uses $d=.42424242424242425$. The first tick's interpolation therefore includes $N[0]$ and the launch seed at $-1$. The actual −.3955149818148254 m/s differs from a pre-commit read of −.3959055118110236 m/s. [Lag conversion](../../../rust/dsp/src/instrument/ring.rs#L89) · [Fraction and first-tick tests](../../../rust/dsp/src/instrument/waveguide/o1_tests.rs#L444)

| Station $s$ | $d_+/P$ | Direct range, samples | $d_-/P$ | Return range, samples |
|---:|---:|---:|---:|---:|
| .162 | .0175 | .424242–51.692308 | .8555 | 20.739394–2527.015385 |
| .174 | .0235 | .569697–69.415385 | .8495 | 20.593939–2509.292308 |
| .186 | .0295 | .715152–87.138462 | .8435 | 20.448485–2491.569231 |

For every legal real $f_s$, $P\le f_s/65$ and the oldest contributing lag is $\lceil d\rceil+1\le\lceil .8555 f_s/65\rceil+1<C=\lceil f_s/65\rceil+4$. At the largest rate it is 2529 against 2958 slots. The post-write count caps at $C$. A lag within count selects a committed slot. A larger legal lag implies count is still the epoch's true age and the corresponding negative time is `count − lag`. That input is evaluated at original launch $P,V$. Once saturated, every legal lag uses written storage. The seed magnitude is at most $.16/(2\beta)<.630$ m/s. Staged emissions are in [−4,4] m/s. Convex interpolation preserves this bound and each subtraction is within [−8,8] m/s. The aperture's nonnegative normalized weights preserve [−8,8] m/s. There is ample binary64 headroom, and no clipping or normalization is needed. [Capacity, seed and staging](../../../rust/dsp/src/instrument/ring.rs#L36)

The implemented Simpson average is $(v_{.162}+4v_{.174}+v_{.186})/6$, the center at $.174$ and normalized width $.024$. For a frozen harmonic at $\omega=2\pi k/P$, the ideal spatial response from $N$ is $2i\exp[-i\omega(1-\beta)P/2]\sin[k\pi(1-s)]$. Linear interpolation adds the respective two-tap response $e^{-i\omega m}[(1-u)+u e^{-i\omega}]$ to each ray. The position envelope at integer orders has magnitude proportional to $|\sin(k\pi s)|$, with a center cancellation near order $1/.174=5.747$. Writing $z=k\pi(.024)/2$, the symmetric three-point multiplier is $(2+\cos z)/3$. The exact rectangular average gives $\sin z/z$. At the latter's first aperture zero, order $83.333$, the three-point multiplier is still $1/3$. The geometry is an effective normalized reference under the guide's instantaneous-period bend model. A fixed physical pickup, fretted speaking length and actual magnetic sensitivity would require additional calibration. [Spatial proposal](../../architecture/ELECTROBOW_ACOUSTIC_OUTPUT_SPECIFICATION.md#o1-reconstruct-the-sensed-quantity) · [Analytical histories](../../../rust/dsp/src/instrument/waveguide/o1_tests.rs#L196)

## Lifecycle and ownership

`Guide::tick_observed(&mut self, hz, speed, k)` advances exactly once and returns `Result<Option<(Sample, Observation)>, Fault>`. `transaction` calculates one local `period`, uses it for propagation and returns it only after both writes and the loss, mode and M5 state commit. `advance` invalidates on `Err`. The `?` and success `Option::map` prevent O1 from sampling on errors or cleared ticks. `Observation::read` is private, and there is no guide getter for an independently supplied delay, stale epoch or deferred read. `read_committed` is an internal ring primitive with a live post-commit precondition. The only non-test O1 call path to it is the success closure. A copied observation owns the six numeric values from its tick and continues to describe that tick if retained across an invalidation. [Observed entry point](../../../rust/dsp/src/instrument/waveguide.rs#L158) · [Commit boundary](../../../rust/dsp/src/instrument/waveguide.rs#L241) · [Private ring primitive](../../../rust/dsp/src/instrument/ring.rs#L89)

| State or event | Verified result |
|---|---|
| Prepared, cleared or transport reset | Valid idle tick returns `Ok(None)`. Invalid controls follow the ordinary guide fault contract. Invalidation clears both logical histories, loss, contact mode and M5 memories. |
| Launch | Existing zero-wet-gain launch precondition belongs to the caller. Launch opens M4 with count zero. O1 is created only by the first successful observed tick. |
| Active and lifted | Each successful tick produces one observation from that commit, including `k=0` during lift. Lift clears the contact force and allows ongoing propagation. |
| Rejected launch or failed tick | Lifecycle probes covered invalid controls and an injected nonfinite outgoing proposal at the staging boundary. Both histories are invalidated on the existing fault path and no O1 sample is returned. |
| Invalidation and replacement | No O1 on a cleared tick. Relaunch installs its own seed and count. The stale physical slots are excluded until overwritten. Retuning a live epoch changes read times and retains its historical seed. |

A future isolated controller consumes both results from the same call and owns the sample clock, silent idle output, launch at zero wet gain, lift/release duration and M6 cached `E` and `E·w` fade. O1 has no post-commit error branch within its proved domain. If a future output stage fails after this boundary, the guide tick has committed. The output fault needs an explicit detection-tick product and invalidation rule. The controller must neither rerun the guide tick nor apply E twice to the cached product. These are interface findings. S4a behavior was outside this implementation. [Lifecycle implementation](../../../rust/dsp/src/instrument/waveguide.rs#L109) · [State parity tests](../../../rust/dsp/src/instrument/waveguide/o1_tests.rs#L293) · [M6 recovery](../../architecture/BOWED_STRING_MATHEMATICS.md#m6-energy-release-and-floating-point-limits)

## Numerical qualification and negative controls

The separate Review 5 oracle rendered actual Rust with `--o1`. It used append-only histories, unsigned prefix storage for negative times and the floor/ceil of the real-valued absolute time $n-d$ for O1. It has no cursor, wrap formula or physical occupancy counter. A recorded-$N$ comparison isolates pickup indexing. The full eager comparison computes its own M4 seeds, bridge FIR history, two incident waves, signed contact branch, scattering and both emitted waves. It locates the convex load graph's increasing-side root and uses its own new $N$ for O1. No Rust emission or mode feeds its evolution. Feedback reads keep absolute indices and fractional offsets separate to avoid accumulating clock subtraction error. The maintained Rust oracle separately uses chronological indexing from the older neighbor and enumerates contact intersections without Rust physical state. [Independent review oracle](../../../build/review-05/independent_o1.py) · [Independent results](../../../build/review-05/independent.log) · [Maintained eager fixture](../../../rust/dsp/src/instrument/waveguide/o1_tests.rs#L68)

The frozen grid used 32, 44.1, 48, 88.2, 96, 176.4 and 192 kHz, the interior neighbors of 32 and 192 kHz, 44,150.5 Hz, and 48,100 Hz with both adjacent binary64 rates. At each rate the stationary cases span 65, 65.41, 82.41, 196, 440, 1318.51 and 1320 Hz, both .06/.16 m/s speeds and R=1.2/2.4. Each stationary case runs through three complete capacities and the next tick. The moving cases cover the tick-100 440→65 Hz retune across the seed seam, 65→1320→65 Hz bend with lift and two cycles of 5 Hz ±35-cent vibrato. The maintained grid pre-saturates a different epoch before starting each case. An additional independently generated grid exercised 4 octaves/s down and up at all 13 rates. The tests keep the same $10^{-9}$ m/s absolute tolerance at every tick.

| Comparison | Cases / ticks | Worst absolute error |
|---|---:|---:|
| Review 5 absolute read of recorded $N$ against six Rust rays, three station differences and aperture | 403 / 2,066,150 | 1.836670e−12 m/s |
| Review 5 independently evolved guide to actual O1 | 403 / 2,066,150 | 1.836614e−12 m/s |
| Review 5 eager physical incidents, outgoing waves and FIR input | 403 / 2,066,150 | 6.772360e−15 m/s, with exact branch tags |
| Review 5 explicit 4 octaves/s, eager guide to O1 | 13 / 272,290 | 9.618695e−13 m/s |
| Maintained Rust eager and chronological comparisons | 403 / 2,066,150 | 2.559064e−14 and 2.564615e−14 m/s |
| Maintained actual Rust/Python spatial replay | 152 / 46,242,000 | 4.440892e−16 m/s |

The two independent frozen-grid O1 maxima were the right return history at tick 75,038, $f_s=191999.99999999997$ Hz, $f=437.4780374263806$ Hz, adjacent times 74,667 and 74,668. Forming the full floating absolute time accounts for the small increase relative to the split-coordinate Rust test. No expected case diverged, faulted or missed a live observation. The maintained grid's peak was 1.2298851055 m/s. Synthetic impulse, constant-history and harmonic cases passed. In particular, a constant cancels, an impulse follows the linear triangle, and the complex two-tap transfer matches measured rays at $P=1000$ and $93.4$ for the tested orders strictly below Nyquist. The oldest legal ring read, exact integers and their adjacent floats passed in 73,830 independent patterned-storage reads across all 7,383 edge rates (predecessor, exact value and successor of 2,461 multiples of 65 Hz). Integral taps matched bitwise. The cut-and-flush test stayed at exact zero despite alternating delay limits after M4 seed expiry. [Edge and partition probe](../../../build/review-05/edge-check/rust/dsp/src/instrument/waveguide/review5.rs) · [Edge results](../../../build/review-05/edge.log) · [Expired-seed test](../../../rust/dsp/src/instrument/waveguide/tests.rs#L320) · [Cargo and O1 results](../../../build/review-05/cargo.log)

The independent numerical negatives use the actual 32 kHz Rust trace launched at 1320 Hz and retuned to 65 Hz on frame 100. The table shows the first failing value for the nominated predicate. Values are m/s except lifecycle status. The maintained compiled suite also rebuilt and rejected all 12 Rust mutants at their named runtime predicates, including center position `.170`, a floating shift before decomposition, and an independent stale-epoch fixture. [Independent negative results](../../../build/review-05/independent.log) · [Compiled suite and predicates](../../../tools/validation/o1_mutations.py) · [Compiled results](../../../build/review-05/o1-mutations.log)

| Defect | First failure | Defective / actual |
|---|---|---|
| Single ray | Tick 0, left velocity | −.395515 / −1.003389 |
| Wrong reflection sign | Tick 0, left velocity | +.212359 / −1.003389 |
| Wrong overall sign | Tick 0, left velocity | +1.003389 / −1.003389 |
| Pre-commit read | Tick 0, left direct | −.395906 / −.395515 |
| One-sample extra lag | Tick 0, left return field | −.271677 / +.607874 |
| Reversed interpolation weights | Tick 0, left direct | −.401420 / −.395515 |
| Extra reflection | Tick 0, left velocity | −.395515 / −1.003389 |
| Uniform aperture | Tick 100, weighted total | +.336663 / +.324551 |
| Rebuilt seed at retune | Tick 100, left return field | +.351969 / −.157190 |
| Previous tick's period | Tick 100, left return field | +.450394 / −.157190 |
| Stale epoch after invalidation | Compiled review mutant, 32 kHz tick 5005 | `Some` / `None` |

The sub-sample seam distinguishes a first-tick emission from two seed samples. The integer-boundary and impulse tests determine the absolute tap time independently of the ring, guarding against a common one-sample shift in both oracle and implementation. [Independent stale-epoch mutation](../../../build/review-05/stale_epoch.py) · [Stale-epoch rejection](../../../build/review-05/stale-epoch.log)

## Exact guide and M5 parity

The diff retains the force solve, incident waves, bridge FIR and reflection, contact branch and M5 calculation in their original operation order. The shared ring interpolation retains its existing arithmetic for guide reads. O1 is read-only after commit. The fresh observed replay matched **all 12 original binary64 columns on 46,242,000 rows** across 152 cases, including signed zeros. Their canonical sorted raw/WAV digests matched the Review 3 corpus, the Stage 05 entry corpus and the fresh unobserved run. I also freshly encoded all **12** original float32 M5 WAVs from observed-mode rows and recovered their original bytes. The canonical 152 raw/12 WAV list has SHA-256 `a8bd257a945a47aecb31c81cf4569613b87b296db52fd5b93a568ce39eb124b3`. [Observed results](../../../build/review-05/observed.log) · [Fresh guide manifest](../../../build/review-05/replay/build/stage-03/manifest.json) · [Review 3 corpus](../../../build/review-03/submitted/manifest.json) · [Fresh WAV check](../../../build/review-05/acoustic.log)

The private state differential compiled `0c98f60` and the submitted Rust separately. Its 13,820 rows matched byte-for-byte at 32 and 192 kHz through launch, wraps, pitch changes, lift, fault and replacement. The comparison includes incidents, emissions, contact force/slip/mode/probes, loss memory, M5 pickup/audio and DC memory, live state, epoch seeds/counts/cursors, newest storage slots and lifecycle counts. Row-stream SHA-256 was `3565e329514d00d05e28272120b4ca0cae3a3d000ec5da959f6496439451a51f`. The in-crate twin-guide differential also compared **every** physical slot after each of 4,000 ticks and reproduced four launches and two failures. [Entry-state builder](../../../tools/validation/o1_state_parity.py) · [State probe](../../../tools/validation/o1_state.rs) · [Result](../../../build/review-05/state.log) · [All-slot predicate](../../../rust/dsp/src/instrument/waveguide/o1_tests.rs#L293)

## Local real-time behavior and partition invariance

Beyond the existing tick, O1 uses three fixed stations, six `read_committed` calls and at most twelve bounded history lookups. Each lookup branches between a single indexed slot (at most one wrap) and a constant-cost seed expression. On the legal committed path the indices are bounded by the proof above. Scalar subtraction and the three-term weighted sum finish the observation. Inspection found no sample-level heap operations, locks, blocking, I/O or unbounded iteration. Setup owns the two existing boxed arrays. The returned observation is six `f64` values (48 bytes), and no new persistent guide state or history allocation is required. The actual allocator probe counted **zero allocations, reallocations or frees** at 32 and 192 kHz while processing idle, launch, active moving taps, lift, induced fault and logical reset. [Allocator instrumentation](../../../rust/dsp/src/ffi/tests.rs#L12) · [O1 allocation test](../../../rust/dsp/src/instrument/waveguide/o1_tests.rs#L357) · [Results](../../../build/review-05/cargo.log)

Fresh optimized timings on Apple M2 Pro, arm64 macOS 26.5.1 and rustc 1.100.0. Each column reports the median [min, max] of five warmed batch means in ns/tick. Guide workloads toggle 196/440 Hz, with one relaunch per simulated second. The standalone O1 read uses saturated storage. Guide bytes are the reported inline state plus two arrays, excluding allocator bookkeeping.

| Rate | Guide bytes per lane | Guide | Guide plus O1 | O1 read only |
|---:|---:|---:|---:|---:|
| 32 kHz | 8,232 | 41.742 [41.474, 42.416] | 53.423 [53.090, 53.779] | 11.280 [11.170, 11.439] |
| 48 kHz | 12,168 | 42.049 [41.681, 42.640] | 54.157 [53.779, 54.371] | 11.259 [11.120, 11.416] |
| 192 kHz | 47,608 | 42.378 [42.058, 42.887] | 54.251 [54.217, 54.430] | 11.404 [11.171, 11.490] |

The maintained 6,016-event test matched one-sample, fixed-64 and deterministic variable blocks with empty calls. The independent 15,009-event probe matched each sample and timestamp at 32, 48 and 192 kHz under fixed sizes 1/17/64/127/256/1024/4097 and a 46-block random schedule with empty blocks. It compared O1 rays, station and aperture bits, M5 pickup/audio, launch/fault counts, epoch and empty/fault events. Controls were assigned in sample time, including launch, retune, lift, invalidation, rejected launch and relaunch. This is local partition qualification around a sample API. Full V18–V21 still require callback/ABI reachability, denormal and integrated memory tests, host timing distributions and deadline evidence. [Guide timings](../../../build/review-05/bench-guide.log) · [Observed timings](../../../build/review-05/bench-observed.log) · [Isolated timings](../../../build/review-05/bench-read.log) · [Partition results](../../../build/review-05/edge.log)

## Acoustic evidence and limitations

`guide_render.rs --o1` invokes `tick_observed`, exports the six rays, three station velocities and aperture alongside unchanged M5, and writes 22 little-endian binary64 values per tick. The rebuilt offline epoch probes check real state after launch and every tick, including right context. The 15 retained trace digests match the tracked manifest (TSV SHA-256 `85ecdac330745653572bd43dd1f7eeebddf80f75511e10daa9a5dc14d0084464`). The 15 traces agree with Stage 04's extracted Python **spatial velocity** to 4.440892e−16 m/s. Stage 05 writes physical-data traces. The M5 float32 WAV encoder and Stage 04's loaded comparison and listening encoders operate offline. FS units on those files follow their documented conversion and listening gains. [Rust adapter](../../../tools/validation/guide_render.rs#L13) · [Python spatial and loaded paths](../../../tools/validation/acoustic_observation.py#L61) · [Trace digests](../../../tools/validation/fixtures/o1_sha256.tsv) · [Independent measurement](../../../build/review-05/acoustic_check.py)

| Actual Rust O1, 48 kHz | Last delivered second RMS, m/s | Whole delivered render peak, m/s |
|---|---:|---:|
| 82.41 Hz nominal | .293011 | .733152 |
| 196 Hz nominal | .291341 | .728816 |
| 440 Hz nominal | .284173 | .743362 |
| 440 Hz, R=1.2 / 2.4 | .279780 / .287389 | .721619 / .767341 |
| 440 Hz, V=.06 / .16 m/s | .171379 / .453288 | .448548 / 1.206812 |

Independent complex Hann projection of the final second of nominal 440 Hz gives H1–H7 peak amplitudes `.2601485, .2212383, .1645018, .1000775, .0386226, .0112516, .0464592` m/s. Corresponding phases referred to the start of that second are `144.81°, 109.63°, 74.47°, 39.35°, 4.32°, 149.09°, 114.89°`. H6 is −27.28 dBr relative to O1 H1. At H6 the center/N magnitude ratio is −11.29 dB and the aperture/center ratio is −.041 dB. The station cancellation accounts for much of that dip, with additional influence from the source spectrum and fractional interpolation. At 1318.51 Hz the 32/48/96 kHz RMS values are .234590/.253276/.277924 m/s. Resampling to a common 16 kHz analysis bandwidth gives centroids 2135/2349/2633 Hz and ≥6 kHz power fractions .00775/.01402/.02950. These are cross-rate O1 diagnostics and do not qualify final V20. At 440 Hz lift the 50 ms windows from .95, 1.00, 1.10 and 2.00 s measured .284250, .233330, .081702 and .00001450 m/s RMS. The auxiliary bend/vibrato scorer measures the acoustic observation and does not grade V10/V11. Their maintained guide predicates passed. [Acoustic replay measurements](../../../build/review-05/acoustic.log) · [Full trace measurements](../../../build/review-05/replay/build/stage-05/manifest.json)

The separate Python O1+O2 research path applies the blocker, .85 FS/(m/s) engineering conversion and 3.2 kHz Butterworth surrogate. At nominal 440 Hz it gives .236271 FS last-second RMS before the fixed delivery trim. Those electrical changes do not originate in Rust O1. Human listening was unavailable. Changed amplitude and phase distributions establish a different observation of the string and supply no perceptual finding about the owner's reported buzz. [Research comparison](../../research/BOWED_TIMBRE_ACOUSTIC_RESEARCH.md) · [Listening gate](../ACOUSTIC_VALIDATION_PLAN.md#compact-human-architecture-gate)

**MINOR · Phase and relative levels in the near-silent lift tail.** The Stage 05 summary emits phase, H1-relative envelope and transfer ratios for any below-Nyquist order. In the final second of the 440 Hz lift, O1 RMS is 2.889e−22 m/s and the largest projected partial is 5.930e−23 m/s. Independently changing the equivalent summation changed a raw reported phase by .0055°. Those values describe numerical residuals and are unsuitable for a voiced or transfer comparison. In this review the phase agreement check used a 1e−12 m/s diagnostic eligibility floor, with agreement within 1e−6 degrees for eligible orders. This review choice does not change any V-number. Before using manifest ratios or phase as acceptance, declare a minimum denominator or eligible window and publish an unavailable value below it. Keep the physical decay and trace. [Summary implementation](../../../tools/validation/o1_qualification.py#L42) · [Reproduction](../../../build/review-05/acoustic.log)

## Regression, artifacts and documentation

| Check | Fresh result |
|---|---|
| Cargo format and Clippy | Passed formatting and all-target Clippy with warnings denied. |
| Cargo workspace tests | Debug, arm64 Release and translated x86_64 Release each passed 84 unit and 3 integration tests. The ignored timing test was run separately. |
| Contact and ring | Stored-graph reference and both executable contact differentials passed 84,886 finite comparisons each, with at most 1 ULP slip error and 9 probes. Ring absolute circuit error was .001770988 samples. Six ring mutants failed their intended predicates. |
| Eager and guide | 90 eager string spectra and the embedded low-E assertions passed. All 152 maintained guide renders passed. Eleven compiled guide defects were rejected. Stress ran 28,800,000 ticks with 600 launches, zero faults and 1200 invalidations. |
| Review 3 validator corrections | Actual extra launch rejected at tick 96,000. A 440 Hz trace labeled 196 Hz rejected at +1399.91 cents. Both `python -O` and `PYTHONOPTIMIZE=1` were rejected. Exact zero and sub-floor dB ratios yielded −240 dB and the zero-decay metadata serialized as strict JSON. |
| Native and packaged | Release CMake build and CTest 4/4 passed, including Rust, filter-bank, pitch detector and editor smoke. Deep strict VST3 signature and `GetPluginFactory` and existing `eb_*` exports verified. The 13-rate replay matched all three shipping payload hashes. Worst neutral error was 2.98023224e−8 FS, non-neutral ABI/package difference 1.1712931e−38 FS and all measured tail peaks zero. |

The rebuilt VST3 executable matches the qualified SHA-256 `28dbe9a2c3209da8be9746d905577ff0ea7fd101147f8c051bf467a8ed45db78`. The input, ABI and VST3 payload hashes are `5206d315d476ce363fb99c0e90884b68f599a3a3e4626e33bfad5c7322588709`, `c4f241531583ac391ec9419a8c21fc7b5f27465e78d93dfcb0aaa64e12fae6de` and `c7251cd11c37048e8cbd2a645897bcd14e0b87dfec1f650c2797bd5cb5658a94`. These check the existing production path. Native Intel hardware, Linux and Windows were unavailable. The full diff preserves shipping routing, public ABI, JUCE parameters and IDs, saved state/presets, UI, and legacy fallback. All generated WAVs and physical traces are ignored. All 24 accepted input digests verified. The permanent fixture files and CC BY 4.0/CC0 attributions are unchanged. [Cargo](../../../build/review-05/cargo.log) · [Prior suites and fixture verification](../../../build/review-05/previous.log) · [Guide mutations](../../../build/review-05/guide-mutations.log) · [Validator negatives](../../../build/review-05/guide-negative.log) · [Stress](../../../build/review-05/stress.log) · [Native/package](../../../build/review-05/native.log) · [Attribution](../../../tools/validation/fixtures/acoustic/ATTRIBUTION.md)

**INFORMATIONAL · Limited documentation corrections verified.** The legacy scatter reads the original two lower input bands and adds the grain to each of the upper excitations. The same-pitch-string four-band ablation replay is 1.3070024995 dB. The attack pair shares the E schedule, lift and terminal controls and can realize different early amplitudes. The motion figures are labeled auxiliary. The O1 specification correctly records the integer cursor shift, original fraction, successful-period ownership and idle/fault boundary. [Scatter](../../../rust/dsp/src/instrument/body.rs#L94) · [Identity replay](../../../build/review-05/replay/build/stage-04/identity.json) · [Updated stroke protocol](../ACOUSTIC_VALIDATION_PLAN.md#compact-human-architecture-gate)

**MINOR · Proposed migration text needs reconciliation before the next acoustic implementation.** The combined O1/O2 row still specifies a read-only accessor and an untouched `ring.rs`. The accepted implementation delivers O1 only from an observed tick, with shared ring interpolation. The M7 draft also says “observer/output failure.” O1 is infallible after a legal guide commit. The S4a plan rows describe the proposed stroke alongside the separate baseline controller permission. Taken literally as work orders, these rows could authorize a deferred accessor, conflate a post-commit output failure with M6 pre-commit failure, or pull the amended gesture into baseline S4a. Split the completed O1 row from held O2 in the next specification patch, describe an output-stage fault as post-commit, and state baseline A3 and the held amendment separately. No DSP change is required for this finding. [Migration stages](../ACOUSTIC_MIGRATION_PLAN.md#L10) · [M7 proposal](../ACOUSTIC_MIGRATION_PLAN.md#proposed-reconciliation-for-independent-sign-off) · [S4a validation row](../ACOUSTIC_VALIDATION_PLAN.md#gates-at-the-earliest-observable-boundary) · [Review 4 authorization](REVIEW_04_ACOUSTIC_RESEARCH.md#g-implementation-authorization)

## Open findings and next authorized task

### MAJOR · O2 acceptance and calibration remain open

The prototype fixes reproducible engineering settings of .85 FS/(m/s), a further fixed $1/\sqrt{2}$ delivery scalar, a 20 Hz DC blocker and a 3.2 kHz Butterworth low-pass. This makes the research replay unambiguous. Product acceptance still lacks approved gain staging and reference criteria. The low-E record fits position over H2–H6 (up to about 494 Hz). It cannot calibrate the 3.2 kHz surrogate, pickup-specific loading or transients. The held-out windows are from that same performance. The Stage 04 high-note runs cover 32/48/96 kHz and the observed differences persist after common-band analysis. No accepted loading evidence spans all supported qualification rates or a comparable-pitch reference onset. The proposed detail blend `l + .025 η(a − l)` returns some upper carrier content. The owner's accepted Noise semantics, acceptable high-band behavior and audible objectionability remain undecided. [O2 equations](../../architecture/ELECTROBOW_ACOUSTIC_OUTPUT_SPECIFICATION.md#o2-loading-detail-and-unit-conversion) · [Scope of the reference fit](../../research/BOWED_TIMBRE_ACOUSTIC_RESEARCH.md) · [Stage 04 findings](REVIEW_04_ACOUSTIC_RESEARCH.md#ranked-findings-and-required-corrections)

I reproduced the Review 4 gain corner from actual Rust O1 with an independently formed direct-form Python load. The 48 kHz/440 Hz, V=.16, R=1.8, neutral-detail/identity render peaks at 1.005419 FS at common gain and .710938 FS with the proposed fixed trim. At G=1.4 and internal Out=2 it implies 1.990628 FS before protection, or 6.155 dB instantaneous demand against .98. This isolated wet witness demonstrates a headroom question. It does not measure a linked limiter, stereo lane recovery or the fraction of limiter-active frames on the specified nominal .35 FS DI pluck at unity Out. In particular a limit on the wet observation's own peak cannot establish V08 or the delivered V17 behavior. The exact normalized V17 denominator across `M5 → spatial → loaded → corrected → delivered` is still underspecified in the proposed plans. [Reproduction](../../../build/review-05/acoustic.log) · [V08 and V17](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria) · [Proposed measurement points](../ACOUSTIC_VALIDATION_PLAN.md#gates-at-the-earliest-observable-boundary)

**Next acoustic task authorized, specification and offline evidence only.** Submit one O2-only acceptance patch and a compact replay matrix for independent review. Use the qualified O1 traces and existing M5/DI fixtures. Use the current surrogate as the single candidate under assessment. Add a documented pickup/load response or comparably direct electrical reference covering the useful band around 3.2 kHz, and a documented comparable-pitch onset/release example. If evidence is unavailable, record that and keep the disputed calibration or semantics held. The patch must resolve the following items before O2 can be assigned for implementation.

| Open requirement | Required exit from the specification task |
|---|---|
| Engineering and delivery gain | Freeze the scalar and units at the O1, blocked, common-gain loaded, delivery-trimmed, signed-G, `E·w`, A4 mix, internal Out and pre/post-protection taps. State exactly once where sign, G, E, the normalized Out mapping and the delivery trim apply. Declare reset/initial conditions and prohibit per-note level matching in engineering acceptance. |
| V17 denominator and points | Tabulate named numerator and denominator signals, window, fixed levels and bypassed stages for every `20 log10(RMS ratio)` predicate. Keep spatial transfer versus $N$, electrical load versus spatial-only and the neutral corrected comparison at common gain as separately named diagnostic rows. Preserve the original M5-to-final V17 gate and report original-baseline exceedance until an explicit amendment is independently approved. Freeze dominant-fundamental detection, harmonic power/eligibility, noise-on/off subtraction and sweep reference. Keep ±3 dB, strongest fundamental, η≥.85, factory residual ≤−28 dB and narrow-peak >6 dB criteria under their existing owners. |
| Nominal limiter use and fault/corner behavior | Freeze the exact .35 FS nominal pluck, unity Out, mix, stereo cases, eligible frame count and limiter-demand definition for the <1% gate. Grade nominal use separately from high-G and max-Out combinations and delivered peak ≤.980001 FS. Specify pre/post protection logs, gain slew, link, Out=0, mute/release, lane-local output-state fault, cached `E·w` on the detection tick and unaffected-lane progression. Provide neutral identity and explicit injected worst-credible gain vectors for corner expectations. Assign integrated tests to S6R without treating an offline bound as passage. |
| Electrical reference and rates | Record the provenance, knowable pickup/loading setup, fitting and withheld bands, and what the reference can constrain. Replay the proposed coefficients, impulse decay, complex gain/phase, cutoff and pole bounds at all seven V20 rates through 192 kHz. Freeze tolerances before scoring. Decide whether 3.2 kHz is justified as a surrogate and whether 20 Hz, damping and trim meet the same fixed output contract. |
| High band, onset and detail | Use the existing 1318.51 Hz diagnostics at 32/48/96 kHz, add the 192 kHz load comparison and score a common bandwidth against a declared decimated reference. Predeclare ≥6 kHz energy, residual/harmonic masks, phase, and 0–10/50 ms onset and lift windows. Check Noise 0/.5/1 and same-seed subtraction, silence, release, Nyquist leakage and loading/level corners. Decide whether intrinsic carrier detail is the approved meaning of Noise and freeze audible acceptance with original-gain and separately level-matched listening copies. |
| Owner and review decisions | Record the owner's reference/gain intent, Noise/detail decision and dated listening outcome, including any failed excerpt. Resolve the fixed measured-criteria matrix with a separate review. Leave V18–V22 and their existing staged owners in force. |

This task may use offline filtering of O1 and reproduce the evidence above. Its scope is one provisional electrical load and its acceptance conditions. Rust O2, source feature implementation, gesture changes, pickup-geometry retuning and feedback work require separate decisions. [Original stage ownership](../ELECTROBOW_MIGRATION_PLAN.md#validation-ownership) · [Review 4 dependency decisions](REVIEW_04_ACOUSTIC_RESEARCH.md#g-implementation-authorization)

### BLOCKER · I1 fails V17 and lacks capture and repeatability contracts

Using the Review 4 196 Hz injected DI counterexample and O2 research loading on actual O1, the neutral H2/H1 is −1.32479 dBr. Proposed I1 produces +9.23725 dBr and moves the strongest FFT bin from 196 to 392 Hz. The steady gains are −4.99148/+4.99148 dB after 51 updates. The required remediation is a reviewed gain rule and realized-output veto covering weak fundamental, nulls, complex overlap, changing pitch and held vectors. In addition, freeze the host-sample capture and publication schedule, FIR center/window support, thresholds, same-sample launch/release/mute/reset ordering and pending/confirmed epoch ownership. Use a clocked oracle with stale, absent, weak, bent and early-repick cases. Require matched same-string/fret, same-gesture pickup repeats at common gain, predeclared cues and held-out takes. Grade the corresponding matched same-pitch string/guitar repeats and held full-wet identity independently of the mathematical inversion. The existing stimulus distances and this replay do not supply those data. [I1 specification](../../architecture/ELECTROBOW_ACOUSTIC_OUTPUT_SPECIFICATION.md#i1-causal-source-feature-and-confidence) · [Review 4 counterexample](REVIEW_04_ACOUSTIC_RESEARCH.md#v17-counterexample-replay) · [Fresh reproduction](../../../build/review-05/acoustic.log) · [Identity results](../../../build/review-05/replay/build/stage-04/identity.json)

### MAJOR · Amended gesture and live S4b remain held

The current 196 Hz experiment changes the V/R schedule and seed under the same E schedule. It does not hold the realized early envelope constant or demonstrate causal DI-triggered entry, repick, legato, mute or release. Review 4's common gain FreePats pair still shows that the recorded layers cannot stand in for a controlled DI gesture series. Require a frozen command/capture ordering contract, a measured common-gain DI soft/hard pair with pre-E controls and 10/90% output, event and release tests, a matched early-envelope negative, comparable-pitch physical onset/release and an owner verdict. Preserve M4. S4b also depends on an accepted A2 selector and S4a qualification. The historical A2 replay still reproduced 112 clean cases and the 200/200 and 199/199 accepted octave-down artifacts. Bounded analyzer measurement retains prior scope. [A3 proposal](../../architecture/ELECTROBOW_ACOUSTIC_OUTPUT_SPECIFICATION.md#a3-gesture-amendment) · [Prior findings](REVIEW_04_ACOUSTIC_RESEARCH.md#ranked-findings-and-required-corrections) · [A2 replay](../../../build/review-05/a2-ledger.log)

## Authorization decisions

| Item | Disposition |
|---|---|
| S2a contact qualification | **Retain.** Stored graph, signed branch and V02 evidence stand. |
| S2a ring qualification | **Retain.** Existing pre-commit interpolation, seed and timing evidence stand. The post-commit shift passed its own checks. |
| S2b guide qualification | **Retain.** Prescribed-control guide and its guide-owned V03–V11/V20 checks stand. |
| O1 mathematical observation | **Qualified.** Two-ray total transverse velocity, fixed three-point effective geometry and absolute-time numerical match. |
| O1 lifecycle and real time | **Qualified in isolation.** Same successful tick, current period, live epoch, bounded read, zero heap activity and local partition invariance. |
| O1 exact diagnostic parity | **Qualified.** Original guide/M5 samples, lifecycle and retained state match, including refreshed WAVs. |
| O2 implementation readiness | **Hold.** Authorize the bounded specification and offline evidence task above. |
| I1 source identity | **Blocked.** V17 fundamental inversion, capture scheduling and source repeatability remain required. |
| S4a independent injected controller | **Proceed in the existing scope.** Baseline A3, synthetic frames, M4 launch, actual private epoch and cached M6 fade tests. Use existing M5 or qualified isolated O1 as a test observation. |
| Amended gesture and S4b | **Hold.** Gesture contract, measured and perceptual evidence, A2 clearance and S4a qualification are pending. |
| Supported S5 wet processing | **Hold.** O2, corrected I1, live articulation, integrated V17/V20 and perceptual evidence remain required. Existing independent legacy/crossover parity work retains its scope. |
| Production integration | **Hold S6R/H/P.** The independent host-only H guard keeps its prior authorization. |
| Acoustic perceptual gate | **Pending.** Human listening was unavailable. The logged buzz on raw M5 remains an open product question. |
| Release qualification | **Hold S7.** Full integrated DSP, callback, cross-platform, host and V22 evidence remain required. |

## Replay record

The [Stage 05 replay](STAGE_05_O1_OBSERVATION_IMPLEMENTATION.md#regression-and-replay) gives the maintained commands for Cargo, contact/ring, guide, Rust O1, acoustic tools, native/package and attribution hashes. Review 5 ran them with Python 3.9.6, NumPy 2.0.2 and SciPy 1.13.1. A replay directory with symlinks to the original tracked inputs keeps the 152 rebuilt guide and 15 O1 traces separate from the submission corpus. The independent probes can be repeated from the repository root while `build/review-05/` is available.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1
w="$PWD/build/review-05"
py="$PWD/build/spec-reconciliation/py/bin/python"
export TMPDIR="$w/tmp"
mkdir -p "$w/replay" "$TMPDIR"
for name in rust tools ThirdParty Cargo.toml Cargo.lock rust-toolchain.toml; do
  test -e "$w/replay/$name" || ln -s "../../../$name" "$w/replay/$name"
done
(
  cd "$w/replay" &&
  "$py" tools/validation/guide_qualification.py &&
  build/stage-03/guide-render --stress &&
  "$py" tools/validation/guide_negative_controls.py &&
  "$py" tools/validation/guide_mutations.py &&
  "$py" tools/validation/acoustic_observation.py &&
  "$py" tools/validation/o1_qualification.py &&
  "$py" tools/validation/o1_mutations.py &&
  "$py" tools/validation/o1_state_parity.py &&
  "$py" tools/validation/acoustic_identity.py
)
rustc --edition 2024 -O tools/validation/guide_render.rs -o "$w/absolute-render"
"$py" "$w/independent_o1.py"
"$py" "$w/acoustic_check.py"
(cd "$w/edge-check" && cargo test --release --lib --locked --offline waveguide::review5 -- --nocapture)
"$py" "$w/stale_epoch.py"
"$py" tools/validation/ledger_pitch.py
"$w/absolute-render" --benchmark
"$w/absolute-render" --benchmark --o1
cargo test --release --lib isolated_read_cost --locked --offline -- --ignored --nocapture
```
