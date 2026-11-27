# S2b prescribed-frequency driven guide

## Entry and implementation checklist

Clean entry at e26d2e04e318883715c753f9a783bcd8bed30db3 on feature/bowed-string-engine. Review 2 authorizes an unrouted, per-lane guide. The implementation checklist below was written before the tick. M2–M6 and the corrected V03 measurement govern. [Authorization](REVIEW_02_RING.md) · [Mathematics](../../architecture/BOWED_STRING_MATHEMATICS.md) · [Validation](../DSP_VALIDATION_PLAN.md)

| Quantity and units | Rust ownership, domain, initial value and update |
|---|---|
| fs (samples/s), β, g | Guide.rate prepared once within [32000,192000]. Qualified ring constant BETA=.127 and signed loss factor −.98. |
| f (Hz), V and k (m/s) | Caller supplies continuous hz in [65,1320], speed in [.06,.16] when driven, k=R V/1.65 with R in [1.2,2.4]. Lift sets k=0 and permits speed in [0,.16]. Cleared guide accepts lift, emits zero. Positive contact requires a launch. |
| Z, F_N, F (N) | Z=1 N·s/m convention. Caller supplying physical F_N converts to k=F_N/(2Z). ContactResult.force=2Zj. Static limit F_N μ₀. |
| P, D_B, D_N, D_p (samples) | Tick locals period=rate/hz, bridge_distance=βP, delays βP−1, (1−β)P, βP/2. All legal taps fit the prepared capacity. Recompute each tick. |
| B,N (m/s), C, n, valid history | Guide.bridge and nut own disjoint f64 arrays, C=ceil(fs/65)+4, next-write cursor modulo C and valid age min(n,C). Invalid on prepare/reset/fault. Launch installs original P,V/β in each arm with count zero. Negative bridge history is (V/β)(frac(n/P)−.5), nut is −(V/β)(frac(n/P+1−β)−.5). Never replace these parameters on retune. |
| x, x₁, x₂ (m/s) | x is bridge.read(D_B). Guide.loss holds preceding two x values. Zero at reset. Launch preloads x[−1],x[−2] with shifted interpolated reads. Prospective pair [x,x₁] commits after both stages succeed. |
| a,b,p (m/s) | from_nut=−nut.read(D_N), from_bridge=−.98(.125x+.75x₁+.125x₂), pickup=bridge.read(D_p). All from preceding history, including p before solve per M7. |
| d,r,j,v (m/s), mode | Prepared FrictionCurve solves (V−a)−b with Guide.mode. ContactResult carries r,j,v, force, B=a+j,N=b+j and next mode. Stick initially idle, Slide(+) on launch, Stick on successful lift. Solver owns branch selection, guide owns its memory. |
| c, p_prev, y_prev, u | c=exp(−2π20/fs) at prepare. DC state in m/s starts at zero on launch. y=c(y_prev+p−p_prev), u=.85y FS at unit captured gain and positive polarity. Stage finite results and commit DC with waves. |
| Guards and fault | Validate controls before division, check finite state and prospective values, apply inclusive ±4 m/s wave/pickup/loss guard. Ring tokens for both B,N precede either commit. Fault invalidates epoch on detection tick and yields no audible sample. Reset/invalidate require constant work. |

| Validation checklist | Acceptance boundary |
|---|---|
| V03 | Actual lifted cut, 7 rates × 3 notes, absolute delay ≤.003 sample and all four timing controls. Evolving eager vs lazy, 100 ms including tick-100 downward retune, both preloads, waves, pickup, branch, seed/cursors/count and wraps, 1e−9 m/s. |
| V04, V20 | Full 90 active spectra plus equivalent notes at all seven rates, pitch ≤12 cents, η≥.90, ≥3 upper partials at −36 dB through 880 Hz, AC RMS>.04, stationarity ≤2 dB, nominal 196 Hz duty and episode limits. Measure bridge and pickup in physical units. |
| V05–V07 | Six 7 s runs, paired first second, ≥60 dB passive decay, driven ±2 dB, separate 5–6 s low-E reference. Three pressures, ≥.01 bright-band increase, force ratio 2±1e−6. Two speeds, RMS ratio ≥1.8. Two seconds cleared exact silence. |
| V08, V09 | Unguarded prospective magnitudes, zero nominal faults, reject internal and input faults before committing, invalidate immediately. Per-second 60 s pitch/RMS/η, <2 dB drift after first second, post-DC mean <.002 FS. |
| V10, V11 | Prescribed 196 Hz triangle and 196/440 Hz vibrato, specified windows, independent phase pitch, fixed-lag alignment, uncompensated and aligned residual, slew, continuity and launch count. Model-only vibrato ≤6 cents RMS. |
| Engineering evidence | Defect-specific negative controls, epoch isolation, expired seed, allocator lifecycle, bounded work, prepared memory and optimized tick timings. Common-gain listening WAVs, raw physical data, hashes and replay manifest. Cargo, native, qualification-oracle and shipping-payload regression. |

## Handoff for independent Review 3

**Awaiting independent Review 3.** The DSP and its isolated fixtures are at `7920de44bff3f8f38f90076bcd4316a986dae260`. The final executable evidence source is `bed28ce34e22ae407e72449bcfb5c4b67d7809e4`, which adds per-order pressure spectra and cross-rate slip timing. The render manifest records this clean checkout and individual source hashes. The subsequent handoff commit contains the measured report, ledger and decision log. Resolve its revision with `git log -1 --format=%H -- docs/implementation/status/STAGE_03_WAVEGUIDE_IMPLEMENTATION.md`.

| Changed file | Purpose |
|---|---|
| rust/dsp/src/instrument/mod.rs | Private module registration |
| rust/dsp/src/instrument/waveguide.rs | Prepared guide, launch, complete sample transaction and epoch lifecycle |
| rust/dsp/src/instrument/waveguide/tests.rs | Integrated circuit, independent eager evolution, transaction rejection, expiry, reset and allocator fixtures |
| rust/dsp/src/instrument/ring/tests.rs | Test-only access to epoch metadata, physical storage and deliberate history corruption |
| tools/validation/guide_render.rs | Offline binary adapter, isolated timing and stress workloads, compiled against the actual private sources |
| tools/validation/guide_qualification.py | Continuous control generation, raw trace capture, spectra, phase scoring, float WAV export and manifest |
| tools/validation/guide_mutations.py | Isolated compiled negative controls |
| docs/implementation/status/{STAGE_03_WAVEGUIDE_IMPLEMENTATION,IMPLEMENTATION_LEDGER,DECISION_LOG}.md | Implementation contract, evidence and review disposition |

### Sample transaction and controller interface

Prepare one exclusive `Guide` per lane off the audio thread. Preparation can fail on rate or allocation. The guide owns both boxed outgoing histories, their independent cursors and seed records, mode, two x samples, two DC memories and fixed coefficients. The only successful launch path validates f,V, installs both epochs, reads the two shifted bridge taps, and sets Slide(+). Launch resets prior DC under zero caller gain. Histories read at integer lag ℓ ≤ written count come from committed storage. An older lag evaluates S at (count−ℓ)/P_launch with amplitude V_launch/β and the M4 phase for that arm. At saturation every admissible lag has a stored value.

1. Validate f,V,k before division. A cleared lane with k=0 returns `Ok(None)` (exact zero wet contribution). A positive-load tick on a cleared lane faults. Validate the preceding loss and pickup states against their magnitude limits and the DC state for finiteness.
2. Compute P and read x and a from the bridge and nut respectively. Form b, then read p from the bridge midpoint, before either write. The M2 interpolator gives weight 1−u to lag m and u to lag m+1.
3. Solve the qualified junction using the prior committed mode. Form the prospective DC sample with the prior pickup and DC. Validate the unmodified x,a,b,p,j and finite contact and output quantities. The ring stages enforce the outgoing guards independently.
4. Acquire both exclusive write tokens. After the second succeeds, the two commits and assignments of [x,x₁], next mode, p and y cannot fail. Return `Ok(Some(sample))`. The physical-unit pickup and positive-polarity M5 audio are both available.
5. A failed transaction drops pending writes. On the same tick the public boundary invalidates both epochs, resets lane-local scalars and returns `Err(Fault)`. A test-build diagnostic counts faults. A fresh valid launch is required before driving this lane again.

The caller supplies continuous Hz and smoothed V/k, owns M2's hop ramp and A3's event timing, applies captured polarity/scale to the M5 audio, and fades in from zero gain. On ordinary release set k=0 while rendering the passive return, then invalidate at zero gain. On fault, S4 owns the cached E and E·w ramp beginning on the detection tick. Keep the previous finite wet product outside this guide. Controller silence expiry, stereo recovery, analysis, body, direct mix, host and package integration retain their assigned owners.

### V03 passive timing

The cut replaces only the prospective B write in a test build, leaving the integrated read, loss, contact, nut propagation and commits in place. With lifted contact, write B=0 for 3C ticks to drain the initial history. Inject B=1 at tick zero, write zero thereafter and capture a for 2C ticks. The measured sparse positive return has gain sum .98 and spans at most four sample intervals. Its earliest nonzero tick k is absolute. For each ω compute k−arg(Σ h[n]exp(−iω(n−k)))/ω. The support sector is narrower than π and the residual phase is continuous from DC. Only the final comparison uses fs/f. [Actual circuit test](../../../rust/dsp/src/instrument/waveguide/tests.rs)

Each entry gives measured samples followed by signed error in samples. All 21 passed the .003 limit.

| fs (Hz) | 65.41 Hz | 440 Hz | 1318.51 Hz |
|---:|---:|---:|---:|
| 32000 | 489.221827358 (-0.000004167) | 72.727148629 (-0.000124099) | 24.268048733 (-0.001770988) |
| 44100 | 674.208838008 (+0.000001438) | 100.227331471 (+0.000058743) | 33.445725755 (-0.001119548) |
| 48000 | 733.832746885 (-0.000000402) | 109.090905197 (-0.000003894) | 36.405496042 (+0.000766461) |
| 88200 | 1348.417672463 (-0.000000676) | 200.454542534 (-0.000002921) | 66.893615487 (-0.000075118) |
| 96000 | 1467.665494142 (-0.000000431) | 218.181828226 (+0.000010045) | 72.809380573 (-0.000078589) |
| 176400 | 2696.835346211 (-0.000000067) | 400.909093797 (+0.000002888) | 133.787419769 (+0.000038558) |
| 192000 | 2935.330989236 (+0.000000090) | 436.363636575 (+0.000000212) | 145.618891844 (-0.000026481) |

Worst absolute error was .001770987909 samples at 32 kHz / 1318.51 Hz. Passing a tick-zero impulse or the observed impulse shifted one tick in either direction through the same estimator failed all 21 comparisons for each control. The compiled reversed-weight guide also failed.

### V03 independent evolving histories

At each rate the oracle eagerly materializes a chronological prefix of C samples per arm, appends only its own outgoing waves, and interpolates forward from the older absolute-time neighbor. Its independent M3 evaluation walks the stored-load segments in ascending slip and maximizes the admissible root. It prepares its own loads, carries its own branch, preloads the FIR from the prefix and advances its own loss state. The guide first fills its real buffers with a preceding epoch, invalidates and relaunches at 440 Hz, V=.10, R=1.8. At tick 100 it retunes to 65 Hz and carries the same launch seed through 100 ms total at each rate. This reaches missing negative-time history at the retune, saturation and multiple physical wraps.

All 67,670 ticks across seven rates passed. Comparisons include both preceding FIR taps before every tick, raw x, both incidents, p, both prospective and physically stored outgoing samples, j, slip and committed loss. Epoch state, cursor, count, launch period/amplitude, preceding and committed modes agree exactly. The largest absolute continuous-state difference was 2.234324e−15 m/s against 1e−9. The test reports the first failing rate, tick and quantity. A separate lifted test exhausts the seed with a drained circuit, reconnects the loop and alternates the extreme delays. Both waves, both incidents, pickup and loss remain exactly zero.

### V04 and V20 steady notes

The prescribed V04 grid contains 90 three-second notes. Extending its six notes and three pressure ratios over all seven rates gives 126. V20 also exercises 110 and 329.63 Hz at nominal pressure at every rate. The independent analyzer grades both B and p against the strongest positive Hann peak, eightfold padding and log-quadratic interpolation. The harmonic-band energy fraction uses the measured fundamental. Significant upper partials are distinct orders ≥2 tested against the requested f. AC RMS is mean-free in m/s. Counts include a Slide onset at the window boundary only when the previous sample was Stick. [Scorer and per-case predicates](../../../tools/validation/guide_qualification.py)

The table covers the equivalent six-note × three-pressure grid. The extrema span both observation points and all 18 fixtures per rate. The final columns show measured physical behavior for the nominal 196 Hz note. The manifest includes every note's frequency, partials, RMS, drift, branch statistics and maxima.

| fs (Hz) | Worst pitch (abs cents) | Min η | Min upper partials through 880 Hz | Min AC RMS | Max abs drift (dB) | 196 Hz B / p RMS | Measured period (ms) |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 32000 | 6.48003 | 0.988776 | 7 | 0.178990 | 0.056590 | 0.224187 / 0.223757 | 5.103198 |
| 44100 | 2.74911 | 0.991631 | 9 | 0.194107 | 0.054824 | 0.225033 / 0.224781 | 5.102041 |
| 48000 | 4.52470 | 0.995397 | 8 | 0.193931 | 0.053778 | 0.227272 / 0.226804 | 5.104166 |
| 88200 | 2.74911 | 0.997042 | 13 | 0.213281 | 0.055916 | 0.227156 / 0.226994 | 5.102041 |
| 96000 | 4.52470 | 0.997044 | 14 | 0.214161 | 0.054916 | 0.227616 / 0.227566 | 5.102679 |
| 176400 | 2.74911 | 0.997884 | 24 | 0.222737 | 0.053583 | 0.227640 / 0.227606 | 5.102041 |
| 192000 | 0.99581 | 0.997078 | 22 | 0.221644 | 0.053516 | 0.228081 / 0.228027 | 5.102273 |

All launched notes voiced at the prescribed fundamental. At nominal 196 Hz / 48 kHz the final second contained 196 Stick→Slide episodes and duty .13475, matching the eager observations. The normative limits are .95–1.05 episodes/period and duty within .05 of .127. Across rates, the nominal 196 Hz mean interval measured directly between slip onsets was 5.10204–5.10417 ms. The largest equivalent-note pickup RMS spread across the eight nominal pitches was 1.261 dB. Controller timing and final wet-output spread retain separate owners.

### V05 paired seven-second renders

Each pair had bit-identical traces for the first 48,000 ticks. Lift changed only k at tick 48,000. Every subsequent contact increment and branch tag was zero. AC RMS below is p in m/s for [.8,1) s and [6.8,7) s.

| Hz | Common starting RMS | Final driven RMS | Driven change (dB) | Final lifted RMS | Lifted change (dB) | Post-lift pitch, [1,2) s (Hz) |
|---:|---:|---:|---:|---:|---:|---:|
| 65.41 | .22703857 | .22764607 | +.02321 | .0000864893 | −68.38274 | 65.41002 |
| 82.41 | .22941191 | .22535046 | −.15515 | .0000110965 | −86.30859 | 82.41002 |
| 440 | .22217193 | .22212422 | −.00187 | 5.71913e−25 | −471.78718 | 439.99958 |

In the separate [5,6) s reference window, low E measured .22813106186949889 driven and .00014419916804091405 lifted. Both reproduce the published eager values within .000002 m/s.

**Investigation of the 440 Hz late tail.** An initial harness assertion applied the 12-cent active strongest-peak gate to the final second of the passive render. It failed. At [6,7) s the pickup has AC RMS 5.55725e−22 m/s. The strongest bin after mean subtraction is .25 Hz (log-interpolated .221 Hz). The decaying quasi-static component dominates that spectrum. The strongest bin above 65 Hz remains 440 Hz. A fresh seven-second eager render reproduced every pickup sample and branch exactly, including the low-frequency peak. V05 specifies final passive decay and driven stability. We retain the unrestricted late spectrum and use the first second after lift for the additional passive pitch observation. The driven pitch check still uses the final second. The original failure and manifest are under `build/stage-03/late-tail-{diagnostic.log,manifest.json}`. Reproduce it by checking `last_second.cents` in the 440 Hz lift case against ±12. The wave equations and specified gates were preserved.

### V06 pressure and V07 speed

All rows are 440 Hz / 48 kHz. Force peak covers all three seconds. RMS, power fractions, onset count and duty use the final second. Static limit is F_Nμ₀. Band fraction uses the 1 s mean-free Hann from 1500 through 6000 Hz inclusive.

| R | V (m/s) | F_N / static limit (N) | Actual force RMS / peak (N) | Pickup RMS | Band fraction | Measured Hz | Onsets / duty |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1.2 | .10 | .145455 / .240 | .078382 / .239471 | .218979 | .099935 | 440.00000 | 440 / .16833 |
| 1.8 | .10 | .218182 / .360 | .107322 / .352585 | .222144 | .113704 | 439.98252 | 440 / .15408 |
| 2.4 | .10 | .290909 / .480 | .139457 / .472542 | .224628 | .122173 | 439.79060 | 440 / .14535 |
| 1.8 | .06 | .130909 / .216 | .066655 / .214328 | .133903 | .117786 | 440.00000 | 440 / .15083 |
| 1.8 | .16 | .349091 / .576 | .175712 / .570974 | .354835 | .111023 | 439.79060 | 440 / .15715 |

High/low F_N ratio was 2.0. The band fraction rose by .022239. Pickup power in the fundamental fell from .663775 through .647500 to .635233 of total non-DC power. The manifest gives each harmonic's fraction on both probes. Fast/slow pickup RMS ratio was 2.649946. After clear, two seconds of k=V=0 returned exact silence at both sample-rate extremes.

### V08 isolated faults and bounded trajectories

The 152 renders completed 46,242,000 ticks, including the phase-analysis context. In a separate 28,800,000-tick, 48 kHz stress run, xorshift32 seed 1 chose 600 successive notes and targets. It exercised the velocity/load endpoints and intermediate values, bounded Hz slews, active contact, lift, reset and relaunch. Every nominal tick succeeded. The following values are peaks of the actual unmodified prospective quantities in this stress run. Stage checks reject a violation without clipping. The per-render manifest retains each individual fixture's maxima.

| a | b | B | N | p | x | j | r | F (N) | v |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| .913382 | .785302 | .857577 | .913995 | .846752 | .835927 | .384000 | 1.541851 | .768000 | 1.381851 |

FIR x₁ and x₂ are committed preceding x values, including the two bounded launch preloads. A test-only proposal editor injects invalid reflection, x, force, slip, j, B, N or pickup. Further tests poison actual committed ring slots, previous FIR and DC state, and feed out-of-range or nonfinite controls. For each fault the harness compares both complete physical buffers bitwise, both cursors/counts/seeds, mode, FIR and DC across the rejected private transaction. It then calls the real public tick, checks the fault counter, immediate invalidation, hidden stale storage, zero idle result and absence of outgoing writes on the detection tick. The editor and diagnostics compile out of the production configuration. S4a/S4b own cached fade, other-lane continuity and recovery. S6R owns the full stereo six-control V08 scenario.

### V09 stationarity and DC

At 48 kHz, 196 Hz, V=.10 and R=1.8 all 60 one-second windows passed on both probes. Pickup pitch was 195.918389–195.918773 Hz, η .998796–.999499 and AC RMS .226521–.226832 m/s. Bridge AC RMS was .226995–.227328. After the first second the maximum/minimum drift was .004642 dB on p and .004637 dB on B. Maximum outgoing magnitude was .427051 m/s, pickup .424050 m/s, and fault count zero. At nominal .85 FS/(m/s), G=1 and positive polarity the whole-render mean was +3.65986e−6 FS. The largest absolute one-second mean was .000139155 FS. An independent filter replay agrees with the actual Rust post-DC stream to <1e−12 FS. Each window is retained in the manifest.

### V10 and V11 prescribed pitch motion

One launch per render, zero faults. Prescribe Hz each sample at 48 kHz. Use a zero-phase third-order Butterworth first-harmonic bandpass (150–290 Hz or 335–650 Hz), Hilbert phase and symmetric ±6 ms derivative. Bend scoring uses [1,1.45) s of the three-second triangle fixture. Vibrato uses [1,3) s, with a further .25 s of identical continuous control as right context for the independent analysis filter. The listening WAV ends at 3 s. Use the earliest maximum Pearson log-pitch correlation on the 0–45 ms grid at 1 ms spacing. Fit offset, sine and cosine to every scored vibrato sample from 3–8 Hz at .01 Hz intervals. Report finite-difference pitch slew over 12 ms. Full sample steps include the physical slip transitions.

| Fixture | Observed Hz range | Slew range (cents/s) | Lag (ms) | Unaligned / aligned RMS (cents) | Modulation (Hz / cents peak) | Max sample step B / N / p (m/s) |
|---|---:|---:|---:|---:|---:|---:|
| 196 Hz, +200-cent bend and return | 195.755–219.656 | −1247 / +1258 | 0 | 1.9723 / 1.9723 | — | .64290 / .46604 / .35281 |
| 196 Hz vibrato | 191.809–200.146 | −1333 / +1359 | 0 | 1.9254 / 1.9254 | 5.00 / 34.863 | .64289 / .47144 / .37477 |
| 440 Hz vibrato | 431.039–449.014 | −1220 / +1200 | 0 | 1.1557 / 1.1557 | 5.00 / 34.582 | .40890 / .40305 / .52778 |

Zero lag is the measured best grid point for this isolated prescribed-Hz experiment. Each NPZ stores the output and labeled pitch trace in cents against time. Live tracking, true legato and delivered-output alignment remain downstream.

### Negative controls

The mutation runner compiles disposable copies of the actual Rust components and requires a panic in the named numerical test with its expected predicate. All 11 compiled and were rejected. [Mutation definitions and predicates](../../../tools/validation/guide_mutations.py)

| Compiled defect | First observed rejection |
|---|---|
| Incorrect reflection sign | Negative first return failed positive support |
| Remove loss FIR delay | 32 kHz / 65.41 Hz returned 488.221827 samples |
| Add an integer propagation sample | Same case returned 490.221827 |
| Reverse bridge interpolation weights | Same case returned 489.959487, expected 489.221832 |
| Preload second FIR tap using first offset | Tick 0 x₂ .293701 vs .282874 |
| Wrong launch neck phase | Tick 0 a −.193701 vs −.007158 |
| Use current retuned period for seed | Tick 100 a −.233760 vs −.127196 |
| Swap outgoing arm destinations | Tick 0 stored B −.247854 vs +.032815 |
| Discard previous contact branch | Tick 10 B −.114775 vs −.230618 |
| Commit bridge before nut validation | Rejected transaction advanced bridge cursor/count |
| Keep stale bridge valid on invalidate | Old epoch metadata survived invalidate |

The additional measured-response controls covered zero delay and both one-tick shifts on every V03 case. These tests combine absolute timing, independent wave evolution, physical buffer inspection, spectral/energy behavior and the previously qualified contact differential.

### Real-time work, allocation and isolated timings

One normal tick performs three interpolated ring reads, at most six bounded integer lookups, one qualified contact solve (at most nine segment probes and one selected-segment division), fixed scalar guards, two stages and two scalar commits. Lookup uses a branch to wrap and saturated valid age. Missing history uses division and floor. Exponentials are confined to preparation. Launch performs two shifted reads. Reset and fault invalidation change fixed metadata and scalars. The only heap ownership is the two off-thread prepared boxed payloads. Source inspection found no reachable synchronization, I/O, growth, deallocation or unbounded search on these operations. The existing thread-local allocator counted zero alloc/realloc/dealloc in debug and release for cleared ticks, launch, moving delay, lift, failed tick, failed launch and reset.

Apple M2 Pro, macOS 26.5.1, aarch64-apple-darwin, rustc 1.100.0, native `rustc -O` without test hooks. Each run warms one second, then times ten seconds of one-lane samples with a monotonic clock, changing 196/440 Hz every 2048 samples and relaunching once per second. Results consume the complete tick via black_box. Five run means are summarized below. Memory includes the 280-byte guide and both f64 payloads, excluding allocator bookkeeping.

| Rate | Slots/arm | Bytes/guide | Median ns/tick | Five-run range (ns/tick) |
|---:|---:|---:|---:|---:|
| 32000 | 497 | 8232 | 29.524 | 29.472–29.983 |
| 48000 | 743 | 12168 | 30.114 | 30.035–30.333 |
| 192000 | 2958 | 47608 | 30.216 | 30.171–30.253 |

V18 still requires integrated allocation, reset, denormal and native-capacity testing. V21 retains callback deadlines, percentiles, stress and host accounting.

### Audio evidence and regression

The renderer produced 152 physical-unit traces, three pitch-trajectory archives and 12 listening WAVs. The 48 kHz mono float WAVs include sustained E/G/A, the driven/lifted low-E pair, bend, both vibratos and the pressure and speed comparisons. Each WAV sample equals the corresponding Rust M5 audio rounded once to binary32. All use positive polarity and unity captured/listening gain following the common .85 conversion. The loudest WAV sample was .576892 FS. Every file hash and every WAV sample was checked after generation. The manifest includes source provenance, control hash and configuration, rate, duration, raw column order, RMS, peaks, spectra and branch measurements, as applicable. Generated evidence stays in the ignored output directory. [Local manifest](../../../build/stage-03/manifest.json) · [196 Hz WAV](../../../build/stage-03/steady_48000_196_R1.8_V0.1_3s.wav) · [Bow lift WAV](../../../build/stage-03/lift_48000_82.41_R1.8_V0.1_7s.wav)

Manifest SHA-256 for the measured checkout is `d8f946c74fa073f2e822df496c4ed30bdc91461bb45baeede23aaf50096f03f2`. Replays at subsequent documentation commits record their own Git revision. The WAVs provide engineering listening evidence. V22 requires a separate human perceptual evaluation.

| Regression | Observed result |
|---|---|
| Cargo format and Clippy with denied warnings | Passed |
| Workspace debug/release, native arm64 | Each passed 76 unit and 3 integration tests |
| Workspace release, x86_64 under Rosetta | 76 unit and 3 integration tests passed |
| V02 contact differential, both targets | 84,886 comparisons each, at most one ULP explicit slip difference, ≤9 probes |
| Existing ring tests, mutations and Python circuit | Passed, including all six compiled ring defects |
| Eager string suite and embedded low-E reference | 90 spectra, minimum η=.9902886844, both embedded assertions passed |
| CMake Release and CTest | Build succeeded, 4/4 tests passed including native processor and editor |
| Packaged 13-rate baseline | Worst neutral error 2.98023224e−8 FS. Non-neutral ABI/VST3 diff 1.1712931e−38 FS. Both tail peaks zero |
| Signature and symbols | Deep strict codesign verification passed. VST3 exposes GetPluginFactory and the existing eb_* entry points |

The fixed-pluck input, ABI and packaged audio SHA-256 match the shipping ledger: `5206d315d476ce363fb99c0e90884b68f599a3a3e4626e33bfad5c7322588709`, `c4f241531583ac391ec9419a8c21fc7b5f27465e78d93dfcb0aaa64e12fae6de`, `c7251cd11c37048e8cbd2a645897bcd14e0b87dfec1f650c2797bd5cb5658a94`. The rebuilt executable SHA-256 is `28dbe9a2c3209da8be9746d905577ff0ea7fd101147f8c051bf467a8ed45db78`. Diff against entry shows private registration and test instrumentation. Contact arithmetic, ring implementation, public ABI, JUCE parameters/presets/UI, legacy filter bank and production render sources match entry. Native Intel, Linux and Windows execution were unavailable here.

### Replay

Run from the repository root with the ledger's Python environment (Python 3.9.6, NumPy 2.0.2, SciPy 1.13.1). The single qualification command recompiles the adapter, renders the audio, scores it and writes the manifest. The Rust tests independently execute V03 and M6.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
py=build/spec-reconciliation/py/bin/python
w=build/stage-03
mkdir -p "$w"
OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 "$py" tools/validation/guide_qualification.py
"$w/guide-render" --stress
"$w/guide-render" --benchmark
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test --workspace --release --locked --offline
cargo test --workspace --release --locked --offline --target x86_64-apple-darwin
cargo test --release --lib waveguide --locked --offline -- --nocapture
"$py" tools/validation/guide_mutations.py
"$py" tools/validation/ring_mutations.py
"$py" tools/validation/ring_oracle.py
rustc --edition 2024 -O tools/validation/contact_replay.rs -o "$w/contact"
rustc --edition 2024 --target x86_64-apple-darwin -O tools/validation/contact_replay.rs -o "$w/contact-x86"
"$py" tools/validation/contact_contract.py --rust "$w/contact"
"$py" tools/validation/contact_contract.py --rust "$w/contact-x86"
"$py" tools/validation/string_oracle.py
sed -n '/^```python$/,/^```$/p' docs/implementation/DSP_VALIDATION_PLAN.md | sed '1d;$d' | "$py"
cmake -S . -B build -DJUCE_PATH="$PWD/JUCE" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
bundle=build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
codesign --verify --deep --strict "$bundle"
"$py" tools/validation/plugin_baseline.py
nm -gU "$bundle/Contents/MacOS/ElectroBow" | rg '_GetPluginFactory|_eb_'
shasum -a 256 "$bundle/Contents/MacOS/ElectroBow"
git diff --check e26d2e0
```

To reproduce the late-tail comparison, run the existing eager suite via Python's runpy.run_path, then call its trace(48000,440,1.8,.1,True,7). Compare its p and branch arrays with columns 5 and 10 (zero-based) of the corresponding lifted Rust trace. The maximum pickup difference and the branch mismatch count were both zero.

### Downstream ownership

The future S4 caller launches under zero wet gain, supplies M2 smoothed continuous Hz and A3 physical controls, retains the passive epoch during ordinary release and invalidates at zero gain. On Fault it starts M6's cached lane fade on the detection tick and requires fresh acquisition. It owns captured scale/polarity, gain, cached product, lane independence and transport ordering. S3 owns the live analyzer and unresolved A2 selector. S5 owns wet crossover/body. S6R/H/P own complete routing, final mix, host behavior, tail and packaged integration. S7 owns release and V22 listening. Independent Review 3 is pending.
