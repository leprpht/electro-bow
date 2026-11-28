# I1 · Source identity on the bowed electric-guitar carrier

**Stage 08.2 offline proposal awaiting independent Review 8.2.** The selected design compresses observed DI harmonic contrasts, drives a complex correction bank with the actual returned O2 sample, and admits a common fraction of the requested correction from recent measurements of the physical carrier. A separate measurement of delivered coloration can withdraw that permission. The original converted M5 reference, O1 geometry and O2 electrical transaction retain their Review 7 meanings. [Physical and electrical boundary](../implementation/status/REVIEW_07_O2_IMPLEMENTATION.md) · [Engineering gates](../implementation/I1_ACCEPTANCE_MATRIX.md)

Use **Stage 08.2 chronological transaction** below for sample order, event ownership, arithmetic, scheduling and storage. The preceding stages preserve the derivations, historical comparators and preregistered acceptance thresholds. The DI projection, feature smoothing, correction poles, transfer derivation, A3 and M6 retain their prior definitions.

## What a held note preserves

Pitch identity is continuous performance Hz from the future A2/A3 boundary. Dynamics include attack level, polarity and the controller's captured G and E. I1 supplies neither. A pickup setting, string/fret, guitar and pluck position can all leave repeatable **relative harmonic levels** in a DI attack. I1 transfers a compressed, reliability-weighted part of those levels to harmonic orders already excited by the bowed string. A repeatable shallow notch can survive. A missing partial supplies no reliable gain estimate. The twelve numbers discard waveform phase, broadband noise, absolute loudness and most of the pick transient. They cannot separate pickup from recording chain or string from plucking position.

One pluck constrains observations of the combined excitation and response. Bow force, bow velocity, pickup position/aperture, electrical loading, impedance, complete guitar transfer and bowed radiation are underdetermined. Virtual bow pressure, speed, modal phase, lift and decay come from the guide. A changing carrier remains audible through the same held filter. A difference between two wet waveforms establishes dependence. Repeatability and identifying a withheld source by a predeclared signed cue are separate tests.

## Source observation

All arithmetic here is binary64, levels in FS, sample origin zero. Share the A1 sanitized pre-swell 30 Hz blocker, including its chronological state. With D=floor(fs/16000), F=fs/D and L=32D+1, prepare the symmetric Blackman-windowed low-pass

```
u[j] = (.42 − .5 cos(2πj/(L−1)) + .08 cos(4πj/(L−1)))
       × (.68F/fs) sinc((.68F/fs)(j−16D))
b[j] = u[j]/sum(u),  sinc(x)=sin(πx)/(πx), sinc(0)=1
hp[n] = exp(−2π30/fs) (hp[n−1]+x[n]−x[n−1])
z[j] = sum(i=0..32D) b[i] hp[o+(j+1)D−1−i]
```

o is the host origin after prepare/reset. FIR center delay is 16D samples. For an identity hop H=round(.005F), freeze current supported pitch f, M=round(4F/f), K=min(12,floor(.28F/f)). Round positive durations by floor(value+.5). For chronological window xₘ=z[J−M+1+m], m=0…M−1, hₘ=.5−.5cos(2πm/(M−1)), μ=sum(xₘ)/M,

```
a[k] = 2 sum(hₘ (xₘ−μ) exp(−i 2π k f m/F)) / sum(hₘ)
A[k] = abs(a[k])
ε = max(1e−5, 3N₀, .015 max(A))
r[k] = clamp((A[k]−ε)/(3ε),0,1)
b[k] = 20 log10(k max(ε,A[k]) / 1 FS)
T[k] = clamp(.45 r[k] (b[k]−sum(r b)/sum(r)),−5,5) dB
```

Hann normalization estimates peak sinusoidal FS. The factor two collects the positive-frequency half of a real sinusoid. A four-period, rounded window and negative-frequency leakage give a finite-window estimate. The complex origin is the first decimated sample, phase is discarded after projection. Use the arithmetic mean before windowing. Accumulate the two weighted moments of x and unity for each k, sum of x and sum of h in one chronological pass. Compute the mean subtraction at completion. The factor k removes a reference 1/k tilt. Centering cancels common gain when eligibility stays fixed. Absolute and relative floors deliberately break exact level invariance near noise.

N₀ is A3's previous quiet RMS estimate, initially .00032 FS, updated with a 1 s one-pole only on idle hops without opening evidence and A<.004, clamped [.0001,.002]. The amplitude comparisons here use peak estimates against an engineering multiple of RMS noise. They do not estimate an order-specific noise variance. Require at least two positive r and finite inputs. Set unobserved orders to zero dB on a valid frame. A single order crossing below ε then tends toward neutral. A wholly invalid frame leaves the held vector untouched. Reliability rises from zero at ε to unity at 4ε. These choices avoid unstable division by a null and prevent an isolated sinusoid from creating twelve fabricated contrasts. Real DI supports using a conservative floor and illustrates lost upper cues. The constants have no controlled instrument calibration. Keep them as a single testable representation. [DI evidence](../research/I1_SOURCE_REPEATABILITY_EVIDENCE.md)

At a valid publication update the owning gesture's I in dB by I←I+(1−exp(−HD/(fs τ)))·(T−I), τ=.060−.040m s, using smoothed Movement m∈[0,1] sampled at snapshot. On every live host tick, after events/publication, update each **linear correction coefficient** c←c+(1−exp(−1/(.015fs)))·(expm1(ln(10) I/20)−c). New live bank c starts at zero, including when a pending I is promoted. On Hold or ordinary Release keep I and finish the slew. A monitor gate can still attenuate c. A publication elapsed across skipped frames uses one coefficient step, without accumulated catch-up. Separate hard-corner tests inject constant c at launch to exercise the entire box.

## Host clock and gesture ownership

Use absolute sample counter n and generation origin o. Empty host calls advance nothing. Reset on n invalidates all earlier work, sets o=n, then n is the first input of the new stream. Use checked u64 differences and an i128 doubled center for comparisons. A stream-clock overflow retires the lane and requires prepare. All ranges here have inclusive endpoints unless written [a,b).

| Object | Exact host time and evidence |
|---|---|
| Decimated j | Last available input e(j)=o+(j+1)D−1. Its FIR uses HP values [e(j)−32D,e(j)], effective center e(j)−16D. |
| Snapshot s | At s=o+iHD−1, i≥1. J=iH−1. Left FIR end l=s−(M−1)D, raw FIR support [l−32D,s], first/last centers l−16D and s−16D. Feature center numerator l+s−32D over 2. Snapshot stamp is s even if the center is fractional. |
| First eligible snapshot | Smallest such s with l−32D≥max(o,event), l−16D−event≥ceil(.025fs), valid stamped input frames and ≥2 supported orders. FIR support includes every integer HP sample in the specified interval. The IIR HP itself depends on raw history from o onward. An onset never resets the shared HP. |
| Pitch evidence | A2 interval P=round(.005fs/Dₐ)Dₐ, Dₐ=floor(fs/8000). Use endpoints o+iP−1. Include the last endpoint at or before l−16D through the last endpoint ≤s, and every scheduled endpoint during the job. Each is present, for the same source token, explicitly compatible or confirmed, Q≥.74, A≥max(.001,3N₀), f∈[65,1320], supported. Require absolute 1200 log₂(f_frame/f_snapshot)≤25 cents. A missing due frame invalidates continuity. Initial acquisition/confirmed change additionally obey A3's starter rules in the controller. |
| Sliced job | B=ceil((ceil(4F/65)+1)/(HD)). Start at s inclusive. Take at most B chronological window indices per tick. Finish and finalize at s+ceil(M/B)−1, always before s+HD. Retain a separate completed slot until exactly s+HD. A publication consumes only this due result. A missing identity hop expires the unpublished result and starts no replacement job for the missed snapshot. A late completion is discarded. History capacity ceil(4F/65)+1+H+2 prevents overwrite while calculating. |
| Publication/consumption | At n=s+HD after same-sample controller events, launch/disposal and frame invalidation, before the new snapshot and current wet calculation. Check the completed token, saved snapshot, publication deadline and current compatibility. A matching active gesture can consume on n. A pending gesture stores its own published I for promotion. |

The oracle computes the same causal projections by array operations over the exact window and models the bounded completion time. Its mutation/replay checks compare actual nonzero features, clocks and owners. At 48 kHz, f=196, event=0, continuously eligible frames, D=3,H=80,M=327. The first eligible snapshot is 2399, FIR support [1325,2399], doubled center 3724, B=5, completion 2464, publication 2639. The historical 51-update reproducer uses the earlier capture convention. [Clock oracle](../../tools/validation/i1_clock.py)

A3 supplies the winning transition, gesture serial, generation, current confirmed/compatible Hz and source-frame end, quality, fast amplitude, previous noise floor, support, Movement, launch/dispose and current guide controls. Every new pick including same-pitch repick allocates a fresh token (lane, reset generation, gesture serial). Acquisition and confirmation refer to the same token. Physical guide epoch binds to it on launch. Never identify the source solely by Hz or amplitude. A pending job may survive launch only with this exact token. Source and physical epochs have different lifetimes.

1. Resolve reset/fault, pick, mute, confirmed incompatible change, ordinary support, timeout in original A3 priority. Keep old audible I frozen during replacement release. A superseding pick/change clears pending I, its job, completed result, source history validity and anchor together. A soft entrance uses its confirmed event tick. Mute or timeout closes the old capture, cancels its job, freezes old held sound through release, and consumes the event. A new pending gesture may collect during the old fade. Disposal of the old wave clears its held state only.
2. Apply stamped A2 support or explicit missing-hop marker. Invalidate an in-progress or completed job immediately on bad support, wrong token or incompatible pitch. Held output cannot refresh on weak/decaying input. Compatible return resumes the current token after a whole eligible window. Unconfirmed large steps enter Hold. A3 alone decides confirmation, mute, new launch and release duration.
3. On confirmed silent launch promote matching pending I only if its snapshot is ≤2HD ticks old, otherwise start neutral. An unconfirmed pending event ages out on A3's 120 ms acquisition expiry. Pending data cannot recolor the releasing old voice. Dispose before replacing at gain zero. At same-tick launch/publication the due result can update the new active I. Launch before publication starts from pending or neutral and slews when evidence arrives. Publication before launch affects pending I only. Pick/mute/reset wins over any old result due on that tick.
4. Complete/publish due work, start the new snapshot, advance its first slice. Then update c, bank and monitor. Pending acquisition, confirmed gesture, active held feature, running job, finished slot, last publication, coefficient target, smoothed coefficient and audible scale are distinct owners. Release never revives a partly faded orbit. For reset/prepare or a new physical epoch clear output states, counters and monitor permission. Keep the compatible continuation's state. Baseline A3 dynamics, M4 launch and M6 fault policy apply. The proposed amended bow entry remains held.

## Frozen transfer and its limits

Let φ=2πf/fs, ρ=exp(−2π·.22f/fs), pₖ=ρ exp(ikφ) and tₖ=clamp((.28F−kf)/(.03F),0,1). Drive both cascaded complex states from returned q in FS. All twelve rotate through a retune, including orders of zero taper.

```
z1[k] = p[k] z1_old[k] + (1−ρ) q
z2[k] = p[k] z2_old[k] + (1−ρ) z1[k]
bₖ[n] = 2 tₖ Re(z2[k])
d[n] = sum(cₖ[n] bₖ[n])
y[n] = q[n] + λ[n] d[n]       normalized wet before σG and E
```

With u=exp(−iω), each cascade contributes C(p,u)=(1−ρ)²/(1−pu)². The second stage uses updated z1, hence no extra u in the numerator. Twice the real part adds the conjugated impulse response. For frozen coefficients the exact transfer is

**T(ω)=1+λ Σₖ cₖ tₖ [C(pₖ,u)+C(conj(pₖ),u)].**

For carrier component Qⱼ=Aⱼexp(iθⱼ), the output phasor is QⱼT(ωⱼ), phase θⱼ+arg T when eligible. Internal paths sum as complex amplitudes before taking magnitude. A narrow pickup null cannot be repaired by dividing by Aⱼ. The realized H2/H1 ratio is (A₂/A₁)|T(ω₂)/T(ω₁)|. Independent ±5 dB requests allow a 10 dB relative change before overlap. The measured −1.325 dBr inversion witness crosses to +9.237 dBr. A nonzero common scalar cancels in this ratio. It also cannot compensate the opposing all-up and all-down level violations at one common gain.

For a stationary resolved line spectrum, fundamental dominance requires |Q₁T₁|≥|QⱼTⱼ| for **every** competing line, including nonharmonic lines and the real Nyquist endpoint with its correct one-sided FFT convention. For finite Hann scoring the exact requirement is on the bin sums of the windowed, mean-removed y. Their leakage terms retain relative phase. At fixed weights on any measured sample interval the exact uncentered energy is ⟨q²⟩+2λ⟨qd⟩+λ²⟨d²⟩. This includes DC, interference among sections, finite-window cross terms and nonharmonic energy. V17 requires this between 10^(−3/10)⟨M5²⟩ and 10^(3/10)⟨M5²⟩. For stationary stochastic excitation the corresponding infinite-window power integral uses |T|² times the actual input spectral measure. A diagonal sum over twelve peak magnitudes omits energy.

Convex contraction gives |z1|,|z2|≤sup|q| from zero even with changing f. Thus |y|≤(1+24(10^.25−1))sup|q| before G. This is an instantaneous bound relative to input history, far too loose for V17 or limiter sign-off. A frozen T has no predictive claim during retunes, gain transitions or transients. [Numerical transfer and causal monitor](../../tools/validation/i1_safety.py)

## Coupled safety law and realized veto

This section records the Stage 08 stationary algebra and historical fixed-bin control. Use the Stage 08.1 contract below for the proposed moving permission.

Use one lane-wide real coefficient ray, maximizing λ over {16/16,15/16,…,0}. λ=0 returns the same-tick q bit-for-bit (explicit neutral return). Each effective gain 1+λcₖ lies between unity and its bounded requested gain. The weights of the direct q path and of the physical guide stay fixed. Keep parallel O2 and bank observations at depths 0,.5,1 on the same signed O1 velocity. These sentinels have identical non-depth controls and coefficient histories. Grade all three together. An independently moving delivery depth additionally needs its own actual q/bank/monitor stream, or may alias an identical fixed-depth lane for the entire epoch. Full-wet V17 comparisons use the fixed G=E=1 pre-mix tap. Do not drive the decision from the dry mix, limiter or a per-note gain fit.

At launch start with λ=target=0, veto latched and good count zero. Use contiguous **nonoverlapping** monitor windows [s,s+N), N=2 floor(4fs/f_guide[s]+.5). Hop=N exact host ticks, approximately eight periods. Keep the actual min/max guide Hz during the window. Take no decision on a partial window. Missing guide reference/clock, failed bridge anchor or pitch spread >25 cents makes a window unavailable. Unsupported *source DI* freezes I while the held physical pitch and committed carrier remain measurable. Physical silence/invalid epochs cannot earn permission.

For each depth take two-column X=[q,d]. Compute G=XᵀX/N and normalized complex DFT rows Pⱼ=(1/N)Σ X[n]exp(−2πijn/N) for j=0,8,16,N/2, with window-local n. Accumulate in time, no FFT allocation. For v=(1,λ), W=vᵀGv, D,F₁,F₂,Y=|Pⱼv|. With this normalization a real Nyquist tone has one spectral line. For real data the remaining two-sided power is R=max(0,W−D²−2F₁²−2F₂²−Y²). By Parseval, **every remaining positive DFT bin** is ≤sqrt(R/2). The short-window certificate compares F₁ with B=max(F₂,Y,sqrt(R/2)). Index 8 is the eight-cycle bin at fs·8/N. This certificate can be conservative for a rich but well-ordered spectrum. Its rectangular bins and hop differ from the one-second inclusive-Nyquist V17 scorer.

For a candidate on a completed window require all of the following, inclusive unless stated otherwise.

- M=mean(M5²)>1e−24 and G₀₀>1e−24 at every monitored depth. Apply the same short-bin dominance test to original M5 alone using the live margin below. Missing/invalid references block admission. A failing neutral V17 observation is separately recorded as an upstream failure.
- W/M and W/G₀₀ in [10^(−2.85/10),10^(2.85/10)]. F₁>1e−12 and F₁≥10^(.30/20) B+1e−12 FS. No square-root of negative remainder from roundoff.
- From the synchronized two-column difference X_factory−X_off compute its own Gram J. Require vᵀJv≤10^(−28.5/10) vᵀG_off v. Squared residual includes all phase and correlation terms. Candidates never use their own wet samples as the bridge denominator.
- For the separate realized veto form X_live=[q,y−q] using actual emitted pre-σG samples and v=(1,1). Apply ±3 dB, .05 dB dominance plus the same absolute floor, and factory difference ≤−28 dB using actual paired y. This observes the complete *time-varying* λ and c trajectory on the finished window.
- An unavailable window, any live failure or a neutral ray point failing the candidate certificate latches veto after **one** observation and clears good count. Otherwise increment good count up to two. After **two consecutive** available, passing observations clear the latch. A latched or infeasible decision targets zero. Else choose the largest passing discrete λ. Never assume the feasible set is monotonic. Equalities pass, fixed descending index resolves ties. A positive V17 neutral with inadequate inner margin simply stays neutral and reports lack of correction permission.

Accumulate the frozen window statistics through e−1, e=s+N. On e…e+16 process exactly one of the 17 candidate grid points per tick, retaining the largest passing index. Meanwhile collect the next independent window. Publish target on e+17 before wet for that tick. On each tick move λ toward target by min(|target−λ|,1/round(.005fs)). Reach exact zero in at most round(.005fs) steps after veto takes effect. All correction poles continue to track q during fallback. Hysteresis and bounded coefficient changes avoid a forced waveform step or repeated resets. The source publication smoother continues independently. Disposing the epoch retires its permissions. Only an actual numerical fault uses M6. A bounded slew guarantees continuity of the *weight*, without a claim of inaudibility.

These checks use measured prior states. The quadratic is exact for a replay of that block with fixed λ and the recorded d[n]. Replacing λ by a trial value does not change the open-loop bank states. The endpoints of two different windows do not prove their interpolation safe, and the admissible set need not be convex. The realized veto monitors the interpolated waveform independently. A transient violation can last a window plus decision/slew time; a straddling failure too short to dominate any observation can escape. Future transients, future source generations and arbitrary future V17 spectra are unobservable. The authoritative one-second scorer, η, measured sweep, control corners and actual paired residual therefore still govern every required stationary fixture. This design cannot promise a perceptual distinction on all sources or sustained coloration on all vibratos.

## Proposed Rust interface, storage and callback work

| Owner | Prepared and lane-local data |
|---|---|
| Shared immutable preparation | Valid finite fs∈[32000,192000], D,F,H,P,B, FIR half-taps including center, smoothing constants, static grid thresholds and normalized units. Return PrepareError on range, size or allocation failure. |
| SourceAnalysis | Shared HP history with A1, decimated ring, finite validity count, sample origin/phase, bounded A2 frame queue. SourceFrame contains end, generation, gesture, continuous Hz, Q,A,N₀,m,support and compatibility. |
| Capture | Gesture tokens, anchor, pending/confirmed/held I[12], last publication stamp. Job contains frozen owner, s,M,K, frequency/floor/Movement, ring cursor, 12 restarted phasors, weighted complex x and unity moments, mean and Hann oscillator. Completed holds T[12], done and due. Published carries token, snapshot/center and I[12]. |
| ColorBank | c[12], target[12], current rotations/tapers, two complex states per order per observed carrier. Output units FS. Sample input is returned O2 q, distinct from raw velocity in m/s. |
| GainSafety | Window start, even N, M5 power/projections, depth-wise Gram/DFT statistics for candidate and realized, residual Gram, extrema, live latch, good count, double-buffered statistics, grid cursor/winner, λ/current target. No retained N-sample audio. |
| Controller/lane | Confirm, launch and guide epoch, M6 invalidation owner, signed G, E and preceding finite (E,Ew) cache. Reset/dispose independently of other lane. SourceUnavailable/Rejected/Veto are ordinary outcomes. Invalid output or corrupt bank/monitor state is OutputFault. |

```
prepare: validate size and fs, prepare O2 shadow instances, FIR and bounded arrays off thread
reset: invalidate stamps/counts and jobs, clear HP/FIR validity, active/pending gains, poles,
       both sets of moment counters and M6 cache; restart decimation at this tick
tick: accept sanitized input and update HP; on decimator phase append FIR dot to ring
      ingest A3 transition and current A2 frame, cancel obsolete evidence
      dispose or promote pending under E=0 if commanded
      on publication hop consume exactly eligible completed result, snapshot next job
      do up to B oldest-first indices, finalize T when last index consumed
      update live coefficient smoother and guide controls from baseline A3
      advance guide exactly once; a cleared tick delivers zero, a failed tick enters M6
      on commit retain actual M5; pass same O1 observation to O2 and sentinel lanes
      advance monitor grid cursor using prior completed statistics; slew λ
      calculate prospective z1 then z2, d, y, σG y, E and E·w; finite-check before caching
      add current M5, q, d and actual y to running sums; at end of window swap statistics
      on valid live sample use original A4 mix, update cache, proceed to other lane
release: freeze active source, lift/release on controller command, dispose at E=0
fault: count the physical commit that occurred; invalidate/reset exactly once; cached fade
```

Use f64 finite/range guards on controls, coefficients, all prospective sums, output and cache products. An unsupported input feature is skipped. A nonfinite waveform or live state is a real lane fault. Maintain ordinary binary64 active subnormals for reference parity, retire inactive histories on disposal, never add dither as a denormal remedy. Use logical ring validity and zero reads for unavailable history, avoiding ring-sized clears on an audio reset. Handle empty blocks without consuming events. A reset/launch/cancel does bounded scalar/array work with no heap, blocking, locks, worker, I/O or synchronized handoff.

For pre-commit guide failure the guide has already invalidated itself and no O2 call is made. An O2 fault follows a successful physical commit. An I1 fault follows that commit and successful electrical processing. In the latter two cases invalidate only the committed lane once, clear O2/I1, and retain the preceding finite E₋ and P₋=(Ew)₋ for audible recovery. On detection tick n₀ and i=0…Nf−1, Nf=max(1,floor(.005fs+.5)), use Eᵢ=E₋(Nf−1−i)/Nf and Pᵢ=P₋(Nf−1−i)/Nf. Mix (1−C Eᵢ)d_direct+C Pᵢ. The first decrement occurs on n₀. No replay of the physical tick, double application of E, second invalidation or extension of an existing fault fade. The other lane advances before linked processing. Fresh acquisition follows the fault fade.

### Bounded cost and delay

The decimator costs 32D+1 MAC terms per emission, at most 385 in one tick and ~32 per host sample on average. Shared A1 can coincide with another 769 terms. Symmetric taps can be stored once per mirror pair. The projection costs M·K≤17,724 order/indices per hop, each with a phasor rotation and two complex accumulations, plus M Hann, sample and DC terms. B≤7 indices per host tick, hence ≤84 order/indices. Finalization uses at most 12 magnitudes, logarithms, reliability and mean operations, 12 gain conversions on publication. Window history scans at most ceil((4/65+.035)fs/P)+3 stamped frames. Event priority and gain smoothing are fixed-size work. One sin/cos and exp on pitch change plus at most twelve complex products prepare pole rotations per lane/tick.

Conservative four-carrier implementation costs 96 complex first-order updates and 48 real weighted output terms per lane/tick. Constant-depth laboratory cases need three carriers. Streaming candidate moments per carrier use 3 real second moments and 12 real projection terms (two columns, real DC/Nyquist and two complex bins); actual y uses one second moment and 6 projection terms with a shared neutral denominator. M5 adds 7 terms, the factory difference adds 3 second moments and one actual difference square. A maximum of 99 such real accumulations per tick for four carriers, plus phase rotation. At most 17 grid trials per monitor hop, **one trial per tick** on the next hop. One trial evaluates four two-variable quadratics plus four sets of four row products, residual/off quadratics and scalar comparisons. One realized and one M5 decision per completed monitor window. N≥194 at the fastest pitch at 32 kHz, leaving room for all trials. The tightest A1/I1 tick can also include 84 projection terms, finalization/publication, 96 pole updates, monitor close and a preceding-window trial if due, plus the unchanged A2 W/K work and guide. Stereo doubles lane work. A one-sample callback receives the full coincident work.

Reserve 8 bytes per decimated slot, ceil(4F/65)+1+H+2 slots, ≤12,800 bytes/lane. Share the longer A1 HP history and prepare (16D+1) identity half-taps, ≤1,544 shared bytes. Per-lane fixed layout ceiling 8,192 bytes beyond this ring: 1,536 for four banks' complex histories, 512 for rotations/gains, 1,792 for jobs/feature vectors, 2,048 for two sets of monitor statistics/grid, 1,536 for stamped support and 768 for counters/control/three additional O2 histories. Use packed bounded arrays and verify actual layout. Rate-specific I1 stereo payload is 33,736 bytes at 32 kHz, 33,864 at 48 kHz, 35,016 at 192 kHz and 42,248 just below 48 kHz. Full stereo upper budget including A1/A2 is calculable as
```
2 × (I1_lane + 8(32Dₐ+1 + 2W+2 + 2(W+2)) + 1536)
+ 8(16D+1 + 16Dₐ+1),  W=ceil((fs/Dₐ)/65).
```
This assigns 1,536 additional bytes/lane for A1/A2/controller scalars and uses symmetric A1 taps. Check at every D,Dₐ upper boundary and 192 kHz (monotone capacities within each interval). The maximum of this layout estimate is 62,608 bytes immediately below 192 kHz, within 64 KiB. Adding the original conservative 128 KiB stereo rings, 8×8192×4 and 6×8192×4 band/raw scratch and 64 KiB JUCE wet scratch gives 717,968 bytes, within .8 MiB before other stream overhead. These are layout targets and arithmetic estimates. Production V18 allocation/subnormal and V19–V21 partitions/deadline measurements remain required. In particular, 48 kHz/64 allows 133 μs mean stereo, 192 kHz/1 only 5.21 μs per callback. [Original budgets](../implementation/DSP_VALIDATION_PLAN.md#budget-and-latency-accounting)

Source center to publication age is 16D+(M−1)D/2+HD host samples. Onset exclusion, frame support, hop alignment and admission add acquisition time. Monitor length spans about 6.1–123.1 ms, its midpoint is (N−1)/2 samples old at close, plus 17 scheduled samples before the new target. Recovery needs two clean observations. The bank's isolated center group delay is 2ρ/(1−ρ) samples for the positive-frequency contribution, approximately 22 ms at 65 Hz or 1.1 ms at 1320 Hz. Actual total group delay is −d arg T/dω where T is nonzero, with direct q and conjugate overlap. None of these clocks buffers the direct audio or delays neutral q.

## Stage 08.1 permission and transaction contract

### Qualification and runtime authority

The replay producer opens one physical epoch with its fixture's controls. It records the successful commit count, original converted M5 (column 11), O1 velocity (21) and returned off/factory/full q (25…27), executable, control bytes, trace digest and half-open interval. The off/factory wet pair cites that existing record and the common correction setting. Every required stationary case first scores **each actual neutral depth** against the original M5, including the original pitch, level, η and paired residual gates where applicable. An ineligible, failed or missing prerequisite fails the case independently of its corrected waveform. The offline runner can evaluate the complete one-second window retrospectively. [Replay ownership](../../tools/validation/o2_records.py) · [Stationary qualification](../../tools/validation/i1_stationary.py)

For live use the controller supplies a trusted carrier validity disposition for the identified physical epoch, derived from qualified preparation, controls, observation and successful commit. Unknown or contradicted eligibility requests zero permission. At launch the scale starts at zero. An observation outage during an active epoch latches veto at the completed monitor block and slews to zero. A continuing contradiction cannot rearm the gate. A fault in committed numerical state takes the M6 path on detection. This conditional permission, the independent measurement of realized wet, and stationary V17 have separate authority. Motion diagnostics use the same operating point's qualified stationary prerequisite and the actual moving M5 observation on every block. Future live-controller issuance of valid epoch receipts remains an integration dependency.

### Host-time moving harmonic fit

Index the first successful guide commit of an epoch as n=0. Use the continuous Hz actually supplied to that successful tick, with its physical observation. Initialize φ[−1]=0, then advance φ[n]=φ[n−1]+2π f[n]/fs before projecting sample n. Retunes and vibrato preserve phase. Accumulate increments within a block and reduce modulo 2π at its boundary. The epoch's first sample therefore has phase 2π f[0]/fs. The epoch owns phase and sample parity. A stopped/failed guide contributes no observation. A new launch reinitializes both. The command sets the demodulation reference. A detuned or H2-dominant realized orbit can disagree with it.

Use nonoverlapping rectangular host-time windows of N=2 floor(4fs/f[first]+.5), including the first sample. Close on exactly the Nth committed sample. The last incomplete window contributes audio and no decision. The measure is the ordinary arithmetic host-sample mean, including during pitch ramps. Let B have rows

```
[1, cos φ, sin φ, cos 2φ, sin 2φ, (−1)^n].
G = BᵀB/N,  U = BᵀX/N,  M = XᵀX/N
A = solve(G,U),  w = [1, λ]ᵀ,  a = A w
P = wᵀ M w,  R = P − aᵀ G a
```

For a sentinel, X consists of q and its fully overlapped signed correction d after coefficient smoothing. For an actual realization use q and y−q at unit scale. The bridge uses M5 and zero correction. Both sinusoidal images are represented by each real sine/cosine pair. DC and Nyquist have one real column each. Keep every off-diagonal entry in G and M. In exact arithmetic Bᵀ(Xw−Ba)=0, so R is the mean squared host-time residual. For any vector orthogonal to B with unit host RMS, its absolute inner product with Xw is at most √R by Cauchy–Schwarz. The fitted-components' individual energies use their own diagonal **blocks** of G. Their sum can differ from the total fitted energy because the different groups correlate.

Set H₁²=a₁:₂ᵀG₁:₂,₁:₂a₁:₂ and similarly H₂² using indices 3,4 (zero-based). Require H₁ ≥ m max(H₂, √2 |a₅|, √R)+10⁻¹² FS. The √2 preserves the single-real-bin Nyquist comparison in the stationary orthogonal limit. DC is fitted separately, retained in the level energy and excluded from the voiced comparison. This certifies separation of the fitted first harmonic from the named competitors and the orthogonal remainder on the measured block. A finite-window off-grid tone can project onto more than one fitted column. A rapid retune can increase R or make a window ineligible. Neither the instantaneous acoustic pitch nor the maximum of a one-second Hann spectrum follows from this inequality. The latter remains an explicit stationary measurement. [Projection and actual-delivery oracle](../../tools/validation/i1_safety.py)

Use binary64 throughout. If any finite-input, finite-accumulator or conversion check fails, reject the prospective state and take the lane fault. For each row of G, let rᵢ be the sum of absolute off-diagonal elements. Require min(Gᵢᵢ−rᵢ)≥.25 and max(Gᵢᵢ+rᵢ)≤2, sufficient for positive definiteness and κ₂≤8. A finite degenerate fit produces ordinary unavailability. Use a fixed 6×6 SPD factorization and forward/back substitution for each RHS. The offline dense solve has fixed dimensions. Evaluate nonnegative quadratic forms using their signed sum S and scale L=|w|ᵀ|M||w|. For the residual subtraction use L=|P|+|aᵀGa|. Allow −64 N ε₆₄ L ≤ S < 0 to round to zero, with ε₆₄=2⁻⁵². A larger negative is an invalid numerical result. Require finite inputs, sums, signed result, tolerance, and strictly positive eligible denominators above (10⁻¹² FS)². The bound is a conservative adjudication envelope for the dot products and conditioned solve. Direct nonnegative sums of squares are checked finite. Remaining comparisons use the displayed inclusive binary64 thresholds and fixed arithmetic order. The absolute dominance guard applies even at equality. A target implementation must demonstrate parity at near-equality and subnormal boundaries.

### Four returns and exact gain order

Drive four independent electrical/bank histories from the same committed velocity. Three depths are fixed at 0, .5 and 1. The fourth consumes the actual provisional sample-indexed detail coefficient. Each bank uses the twelve current guide poles and its own q. Analytic interpolation of *colored* sentinel outputs has no role in admission. For each candidate λ=16/16,…,0, test all three sentinel levels against both original M5 and same-depth neutral within ±2.85 dB, the fitted dominance with m=10^(.30/20), and the Gram energy of [q_factory−q_off,d_factory−d_off] divided by candidate **off wet** energy ≤10^(−28.5/10). Retain the largest passing common λ. A failing neutral candidate vetoes the entire grid. Measure the actual four delivered trajectories with ±3 dB and .05 dB dominance and the actual paired residual ≤−28 dB. The delivered stream can veto even when all three sentinels admitted a candidate. Host-time Gram(q,d,y), pre-scale-slew target power and actual post-slew output power distinguish the requested trajectory from its delivery.

Publish the frame update in dB, update the twelve linear c coefficients with the 15 ms pole, advance z1 then z2, then apply the common λ to their signed sum. Decrement or increment λ toward its current target by at most 1/round(.005fs) per sample, clamped to target. Veto requests zero through the same slew. The first 17 samples following a completed block still use the previous target. Offset 17 uses the new decision. Require two consecutive available, neutral-admissible, realized-clean blocks to recover. The measured Gram proof holds for frozen λ applied to the measured d. Intermediate c and λ change the realized trajectory. Each completed block measures those *actual* samples, including the transitions. During λ=0 deliver q by direct return, preserving every bit including negative zero. Continue valid history, phase and slews through neutral. A finite envelope or level bound for a past block cannot certify future samples.

### Joined event and ownership table

Each lane has a source token (generation, acquisition serial) and a separately increasing physical epoch within a generation. Lane identity is implicit in its instance. The injected Confirm receipt represents the original A3 strong-frame decision with a checked token and absolute last-valid tick. An incompatible confirmed change requires A3's original three-frame rule. External A3 also supplies the verified E envelope, lift and disposal/launch commands. Freeze the old held identity during a pending replacement. An unrelated pending result can never address the old live bank. The elapsed acquisition timeout is ceil(.120fs), even if support continues. A confirmed receipt expires on the first tick beyond its inclusive validity. The newest A2 endpoint is the last scheduled endpoint at or before the current tick, including every intervening due stamp.

| Order | Condition | Changes on that host tick |
|---|---|---|
| 1 | Transport reset | Clear shared HP/FIR validity, jobs, both source tokens, active epoch, histories, pending decision and cache/fade. Change generation and FIR origin. Consume coincident older events. |
| 2 | Known physical fault or an already active cached fade | Fault invalidates the affected live lane once. During the cached fade the lane consumes no live guide/O2/I1 tick. Ignore repeated failures and require fresh acquisition after the terminal zero. Reset still wins. Other lane and shared analysis continue. |
| 3 | Pick, then mute, then incompatible change, then Hold/compatible return/release/timeout | Only the highest applicable gesture wins. Pick/change issues a new pending token and clears its old evidence. Mute/release/timeout retires acquisition, job and unpublished result. Hold freezes held I. A compatible return keeps epoch and resumes supported capture. A3 owns the energy trajectory. |
| 4 | Explicit confirmation and expiry | A matching unexpired pending authorization becomes confirmed. Retire expired tokens together with their saved feature and job. Later support alone cannot recreate a token. |
| 5 | Stamped support and missing work | Consume each due A2 observation or a missing marker. Bad, incompatible, stale-owner or missing support cancels even a completed unpublished job. A missing due identity hop discards its result and starts no new snapshot. No catch-up. |
| 6 | Disposal then launch | Require old epoch identity and E=0 or its earlier invalidation. A matching confirmed token can launch only into a vacant lane at zero E and with a newer epoch. Reject wrong or missing ownership and nonzero boundary. Start a zero-state bank/monitor. Promote only a matching feature whose snapshot age ≤2 identity hops. |
| 7 | Complete, publish, consume, snapshot | At the designated tick finish the job. Publish only on its exact due hop with matching generation/token, full stamped support and completed job. If newly launched with that owner, consume on this sample. Snapshot the next permitted job after publication. Pending, completed and held features occupy separate slots. |
| 8 | Gain, guide, O2, I1 | Derive target c; update live c once. A precommit guide error has no returned sample and skips O2. A postcommit O2 error preserves the physical fields. After both succeed, form prospective bank, candidate/realized moments, scale, wet and finite conversion. An I1 arithmetic error preserves both commits. Ordinary rejected DI, finite ill-conditioning, veto and unsupported source use normal hold/slew. |
| 9 | Cache, then next lane | On success publish prospective I1 state and finite E and P=E·w. On fault abandon the prospective I1 state. Snapshot the preceding finite cache and start the first fade decrement immediately. Advance the other lane on the same host tick. |

Duplicate commands of one typed kind are invalid fixture/controller input. A same-sample reset, mute, expired receipt or repick cannot publish an old token. Separate absolute hop clocks use the FIR support and doubled-center formulas above. The joined trace records held/requested and smoothed gains, phase/permission decisions, epoch cursor, four returns, actual depth, wet, E, cache, fault site and invalidation count through its sparse event records and aligned arrays. The physical rows are replay observations, the typed events, support, envelope and fault sites are injected. [Clock](../../tools/validation/i1_clock.py) · [Joined stereo oracle](../../tools/validation/i1_joined.py)

### Proposed private interface and transaction

```rust
struct Token { generation: u64, serial: u64 }
struct Confirm { source: Token, valid_through: u64 }
struct Dispose { epoch: u64, energy: f64 }
struct Launch { source: Token, epoch: u64, energy: f64 }
enum Upstream { Unknown, Valid { epoch: u64 }, Contradicted }
enum Observation<T> { NotDue, Missing { tick: u64 }, Due { tick: u64, value: T } }
struct Commit { epoch: u64, tick: u64, hz: f64, m5: f64, velocity: f64 }
struct Returns { q: [f64; 4], actual_depth: f64 }
struct Cache { energy: f64, product: f64 }
enum WetResult { Live(f64), FaultAfterO2 }
```

A checked A3 event frame carries the single winning gesture plus optional Confirm, Dispose and Launch and both due-hop stamps. Use explicit prepared state sized at the lane rate. The signature of the proposed lane operation is `tick(&mut self, commit: Commit, taps: Returns, held_db: &[f64; 12], validity: Upstream) -> WetResult`. The caller owns the physical/O2 transaction, E, polarity, G and cache. Feature rejection never calls the fault path. Only stream reset invalidates shared source analysis. Normal disposal retires only the old wet epoch. A numerical fault retires its pending wet acquisition, correction states, permission and O2/guide once, retains the shared analysis and routes its saved cache into M6. A failed precommit guide already owns physical invalidation.

```
for each host sample, for each lane in fixed order:
    resolve typed events and due hops using the table
    if fading: emit cached E and cached product with current fade coefficient
    else if live:
        consume exactly one successful guide observation or handle precommit fault
        obtain actual four electrical returns or handle postcommit O2 fault
        evaluate one scheduled prior-window candidate, apply target/slew for this tick
        propose c, poles, bank, d and wet; at zero scale copy q directly
        accumulate actual completed host-time measurements; finite-check all proposals
        on failure: abandon wet proposal, retain committed fields, enter cached fade now
        on success: publish state and update cache(E, E * signed_gain * wet)
    mix using the cache product once, continue the other lane
```

At fault tick n₀, i=0. For Nf=round(.005fs), emit E₋(Nf−1−i)/Nf and P₋(Nf−1−i)/Nf separately. Exact zero occurs at n₀+Nf−1. Do not replay physical history, multiply the product by E again or restart the fade.

### Prepared storage and worst simultaneous work

The proposal keeps the previous 96 complex section updates and 48 weighted terms per lane/tick. A fixed prepared implementation can maintain the following binary64 buffers, using host-time sufficient statistics without storing a monitor window. For each of two buffers allow 21 packed symmetric basis products, 78 cross-products (six basis values for each of q,d,y on four paths plus M5), 12 q/d energies, four realized y powers, M5 power and four paired products. This is 120 doubles, 960 bytes per buffer. The fourth d column also supports delivery diagnostics. Maintain two extra sums for actual-depth pre/post-slew diagnostics separately. During a close, factor the 6×6 Gram once, solve 13 RHS in fixed-size scratch and retain the needed Gram subblocks. A trial tests three sentinel two-column fits and the paired/off quadratic. Actual comparisons at closure cover four outputs and M5. All grid trials fit into the following block (N≥194).

Reserve the original 2,048 bytes/lane for the two packed buffers and counters. Add **1,024 bytes/lane** to the prior layout for factor/solve staging (21+78 doubles), two diagnostic sums, phase/parity and typed token, event and fade bookkeeping (216 remaining bytes). Reuse the retired buffer in place for trial coefficients. Keep its group subblocks until the last trial. This leaves 64,656 of 65,536 stereo analysis/control bytes, 880 bytes slack. The full prepared estimate is 720,016 bytes before remaining stream overhead, under .8 MiB. These are allocation ceilings for a future layout check. The offline Python oracle retains sample arrays to enable independent grading.

Every live tick has a phase advance and sin/cos pair, H2 from a double angle, at most 21 basis products, 78 projection updates and 21 energy updates, and bounded check/slew/event work. A close can coincide with one sample of both FIRs (385+769 terms), up to 84 projection order-indices, 12 feature finalizations/conversions, guide and four O2 updates, 96 bank sections and all 120 monitor accumulations **per lane**. Budget one 6×6 Cholesky (21 entries, inner loop ≤5), two six-row triangular substitutions for each of 13 RHS (≤390 multiply-subtracts and 156 divides, reciprocal reuse possible), and five realized/bridge checks on the close tick. Test one frozen λ per tick at offsets 0…16 of the next window. A trial has three 6×2 coefficient combinations, three dense six-coordinate fitted-energy quadratics, their two 2×2 group forms, and the level/pair forms. Both lanes may close and publish on the same single-sample callback. There is no data-dependent iteration count, dynamic allocation, inverse search or block-length amortization assumption. At 192 kHz stereo this adds up to 46.08 million statistic updates/s before closure work, alongside 36.864 million complex section updates/s. The 5.21 μs single-sample stereo deadline, exact layout, allocations, subnormal behavior and V18–V21 timings remain unmeasured for I1. Independent review owns the decision whether this bounded schedule merits an isolated Rust prototype.

## Stage 08.2 chronological transaction

This section governs the proposed private implementation. Use one epoch operation for any sequence of committed guide frequencies. The numerical reference calls that operation once per host sample, including within its contiguous-array adapter. The two-lane harness interleaves source and wet transactions. A prerecorded Rust row supplies exactly the next successful guide commit and observation. Its four electrical inputs are the returned off/factory/full q and the independent returned q on the same velocity with the current automated depth. The controller fixtures supply timestamped decisions. Full-window fits, time-series exports and the historical fixed-bin driver are offline comparators. [Stereo coordinator](../../tools/validation/i1_joined.py) · [Sample kernel](../../tools/validation/i1_stream.cpp) · [Capture job](../../tools/validation/i1_capture.py)

### Ownership and types

Proposed Rust field types follow. Use binary64 through analysis, banks, monitoring and cache. Use fixed-width unsigned counters with checked differences. A prepare or controlled stream reset precedes counter exhaustion. A lane borrow lasts for one sample. Prepared FIR halves, rates and O2 coefficients are shared by immutable reference. The lane owns its support, jobs, wave and all four electrical and correction histories. Borrow the current and retired blocks disjointly by their slot indices. A fault retires the whole mutable epoch. This permits writing tentative values in private epoch storage without a second copy of every bank.

```rust
struct Token { generation: u64, serial: u64 }
struct PhysicalEpoch { generation: u64, id: u64 }
struct Pending { source: Token, pick: u64 }
struct Confirmed { source: Token, valid_through: u64 }
enum Gesture { Reset, Fault, Pick, Mute, Change, Hold, Compatible, Release, Timeout }
struct Confirm { source: Token, valid_through: u64 }
struct Dispose { epoch: PhysicalEpoch, energy: f64 }
struct Launch { epoch: PhysicalEpoch, source: Token, energy: f64 }
struct EventFrame { generation: u64, gesture: Option<Gesture>, confirm: Option<Confirm>,
                    dispose: Option<Dispose>, launch: Option<Launch> }
enum Stamp<T> { NotDue, Missing, Present { generation: u64, end: u64, value: T } }
struct CaptureJob { owner: Token, snapshot: u64, due: u64, first: u64, i: u32, m: u32,
                    hz: f64, floor: f64, alpha: f64, sum_x: f64, sum_w: f64,
                    xw: [Complex64;12], w: [Complex64;12] }
struct Capture { pending: Option<Pending>, confirmed: Option<Confirmed>,
                 active: Option<(PhysicalEpoch,Token)>, job: Option<CaptureJob>,
                 published: [f64;12], published_end: Option<u64>, held: [f64;12] }
struct PreparedI1<'a> { fs: f64, d: u32, hop: u32, budget: u32, hp: f64,
                        fir_half: &'a [f64], gain_step: f64, slew_step: f64 }
struct CorrectionBank { z: [Complex64;12], zz: [Complex64;12] }
struct Moments { gram: [f64;21], projection: [[f64;6];13], qd: [[f64;3];4],
                 yy: [f64;4], mm: f64, pair: [f64;3], pair_y: f64,
                 delivery_pre: f64, delivery_qy: f64, delivery_dy: f64 }
struct Retired { slot: u8, length: u32, conditioned: bool, available: bool, realized: bool }
struct Trial { index: u8, largest_pass: Option<u8> }
struct Permission { actual: f64, target: f64, selected: f64, pending: bool, clean: u8 }
struct Delivery { depth: f64, q: [f64;4], signed_gain: f64, energy: f64 }
struct WetCache { energy: f64, product: f64 }
struct FaultFade { source: WetCache, index: u32, length: u32 }
enum NumericSite { Phase, Coefficient, Bank, Trial, Moment, Normalize, Gram,
                   Residual, Wet, Conversion, Cache }
enum Fault { Physical, ElectricalAfterCommit, IdentityAfterCommit(NumericSite) }
enum CaptureOutcome { NoChange, InvalidFrame, RejectedFeature, Published(Token) }
enum MonitorOutcome { Partial, FiniteUnavailable, Veto, Admitted }
enum LaneOutput { Idle, Live(WetCache), CachedFade(WetCache) }
```

The types describe the interface. Lay out optional commands and pending slots with explicit tag words and payload arrays in the arena below. The library's native enum layout is not a storage contract. Each lane receives its own trusted controller receipt carrying generation, epoch, rate, valid-from, inclusive valid-through and permitted control ranges (committed Hz, speed, force and depth). Only the acquisition/controller owner may issue or refresh one from already available qualified operating conditions. Revocation, wrong scope or a missing receipt requests zero on this tick, invalidates pending selection, and starts the usual bounded slew. Measured contradiction, including an unfit original M5, makes the observed window ineligible for recovery. Two complete eligible observations and a fresh matching disposition are needed to resume after revocation. A preset, a colored sample or a successful previous physical epoch cannot issue permission. The one-second neutral V17 score is a separate offline prerequisite prior to the run. The live issuer, operational coverage, revocation source and A2/A3 decision quality remain controller integration work. [Receipt and joined tests](../../tools/validation/i1_stream_checks.py)

### One host sample

1. Take the next sanitized direct inputs and timestamped shared controls once. Resolve stream reset before running either lane. Increment generation once, restart both FIR origins and clear both physical epochs, caches and fades. Retain immutable preparation. Advance the active control values at this timestamp before guide commit. In qualification the two lanes consume the injected values and E on this timestamp.
2. Validate the event frame before applying commands. The external frame has a generation, at most one command of each structural kind, and a finite E on physical commands. Repeated copies of any gesture invalidate the frame. Multiple distinct gestures reduce to the existing A3 order in the gesture enum above. Reset consumes other commands. Fault, mute, release and timeout cancel acquisition and unpublished work. Pick/change replace its serial, anchor, pending feature, job and support stamps. Hold cancels refresh for the active owner. Compatible resumes capture without restarting the wave. During cached fade consume only stream reset. An invalid frame drops its commands and records the invalid input while ordinary sample processing continues. Source serials remain monotone across transport resets. Physical epoch numbering may restart in the new generation.
3. Before confirmation, expire an unconfirmed acquisition when elapsed host samples reach `ceil(3 fs / 25)`. The half-open interval starts at the pick. At integer rates compute the ceiling by integer arithmetic. At prepare, express the finite binary64 rate as its exact integer ratio a/b and take ceiling(3a/(25b)) with integer division. Store the resulting host-tick count. At 32008.333333333336 Hz this gives 3842. Computing the product in binary64 first rounds it down to 3841. A previously confirmed token expires on the first tick strictly after its own inclusive deadline. Expiry retires the token, running/completed job and pending publication together.
4. Advance causal HP and the current decimated emission from samples through this tick only. Record due A2 frames at the reset-relative endpoints. A missing frame, wrong owner, incompatible pitch or weak support cancels even completed work awaiting publication. Confirm only a matching pending token with current matching supported evidence. A timeout, reset or loss of support coincident with confirmation wins. A mismatched token or prior generation has no authority to promote. An explicit disposal checks the old physical epoch and zero actual E. Disposal precedes replacement launch. A matching confirmed token may launch at E=0 into a vacant lane with a strictly newer epoch. Preserve a different pending owner's work on ordinary disposal.
5. Advance the outstanding projection by at most B chronological indices. Only one job can own the source accumulator. At completion retain its target until the exact due identity hop. At a missing hop cancel the slot. At the correct hop publish only a completed, supported matching result. Launch before publication can consume that tick's result. Launch after publication uses a matching snapshot at most two identity hops old. Pending capture and publication can run during an older audible hold. After publication start an eligible new snapshot and advance its first slice. The preceding job always finishes before this hop. For each slice accumulate unweighted sum x, Hann sum, and the complex sums of weighted phasor and weighted sample times phasor. Finalize as `2*(XW-mean(x)*W)/sum(Hann)` with the accepted reliability and centering law. Use the history ring and cursor throughout the job.
6. Handle an already active M6 fade or idle state before the guide. Otherwise perform exactly one qualified physical transaction with the current controls. A precommit failure yields no physical sample. Read M5 and O1 from the successful commit. Tick four independent O2 lanes at 0, .5, 1 and current delivery depth. An O2 failure retains the successful physical commit. Smooth each coefficient from the current held target once, advance 48 pairs of complex sections from the four returned q values and compute four d values. An offline prescribed held-coefficient fixture can drive the same bank port directly.
7. On the first tick of a new observation set the even window length from this commit's frequency. Advance phase before observing this sample. Consume a selection completed on a previous tick. Perform at most one trial on the retired block. Its result becomes eligible for the **following** sample. Apply the bounded slew to the already available target and form the four actual y values. Copy each q directly at zero permission, including signed zero.
8. Accumulate the current sample into the prospective block. At closure retire the complete block, condition and factor it, solve the basis fits and evaluate original M5 and the four realized trajectories. Sample n has already been computed. Reduce phase modulo 2π at closure. Arm the first candidate for n+1. No retrospective decision alters sample n.
9. Check finite conversion, signed wet, E, the product and the candidate A4 mix before publishing the cache. On success publish this epoch's finite state, E and P=E·σG·y_delivery. On a checked failure retain evidence of whichever physical/electrical operations succeeded, invalidate the affected epoch once and abandon its tentative I1 state. Snapshot the preceding finite pair and emit the first M6 decrement on this detection tick. Both lanes finish before the next host sample. Apply the existing shared output stage to their emitted products and direct inputs.

Wrong confirmed token, stale launch, mismatched physical disposal and nonzero disposal cannot replace the live wave. A structurally valid frame may independently confirm an eligible acquisition even if its physical operation is ineligible. A malformed injected command is a harness/controller disposition. A finite feature rejection, a finite unobservable fit, and an ordinary veto retain the valid guide, bank and phase. A bad numerical operand in a checked computation is a lane fault. Broken array dimensions, an impossible cursor, unexpected exceptions or an invalid qualification inventory fail the run. The native kernel catches only its numerical fault type.

### Packed binary64 arithmetic

Offsets refer to a single contiguous block. Within each row use increasing order. Each word occupies eight bytes.

| Offset | Field | Count |
|---:|---|---:|
| 0 | Lower triangle of basis Gram, row i entry j at i(i+1)/2+j | 21 |
| 168 | Six basis projections for each of q,d,y by path, then original M5 | 78 |
| 792 | q², qd, d² for each path | 12 |
| 888 | Actual post-slew y² for each path | 4 |
| 920 | Original M5² | 1 |
| 928 | (q₁−q₀)², (q₁−q₀)(d₁−d₀), (d₁−d₀)² | 3 |
| 952 | Actual (y₁−y₀)² | 1 |
| 960 | Actual delivery (q₃+target·d₃)² before slew | 1 |
| 968 | Actual delivery q₃y₃ and d₃y₃ | 2 |

The exact block is **984 bytes**. The 24-byte difference from 960 retains the pre-slew power and both remaining delivery Gram cross terms. The realized diagonal already belongs to y². The native layout has two blocks at 0 and 984, first and second complex histories at 1968 and 2736, twelve complex poles at 3504, twelve tapers at 3696, then six f64 values beginning at 3792 (last pitch, radius, phase, scale, target, selection). Seven i32 fields at 3840 hold current slot, count, length, retired length, trial, consecutive passes and best index. Seven one-byte flags at 3868 hold observed, conditioned, bridge, available, realized, veto and pending. Five alignment bytes complete the 3880-byte epoch. Generation and physical owner reside in the enclosing lane. The exact proposed shared/lane field offsets and rate-dependent rings are produced by the layout calculator. [Arena layout](../../tools/validation/i1_layout.py)

Use round-to-nearest binary64 with separate multiply and add, gradual underflow and explicit finite checks. Compute phase as previous plus `(2π*f)/fs` and generate 1, cosφ, sinφ, cos2φ, sin2φ and epoch parity. Each window is even, so its first parity is positive. Add each product in increasing host time. The retired normalization multiplies by the same precomputed 1/N. Gershgorin radii sum the absolute off-diagonals in increasing column order. Require the inclusive bounds [.25,2]. Finite rank loss produces ordinary unavailability. Cholesky uses ascending rows, ascending lower columns and ascending prior pivots. Solve each of the 13 RHS by increasing forward index and decreasing backward index, visiting remaining columns in ascending order. Replace retired projections in place with the fitted coefficients. A failed numerical pivot after an accepted conditioning screen faults the epoch.

At grid weight w, form each fitted coefficient by a_q+w·a_d. Evaluate level as `(qq+2*w*qd)+(w*w)*dd`, and evaluate each component's energy as the ordered double sum of `(a_i*G_ij)*a_j`. The fitted energy uses all 36 terms. Residual is level minus fitted energy. For these nonnegative quantities clip only a finite negative inside `64*N*eps*sum(abs(terms))` to zero. Use |level|+|fit| for the residual. Material negatives and any nonfinite arithmetic fault. Compare H1 RMS to the maximum of H2 RMS, √2|a₅| and residual RMS with the accepted absolute floor and margins above. Use original M5 and same-path neutral q² denominators. Compute candidate paired power with its full three-term quadratic. For the realized test use the measured y projections, powers and actual difference directly. Strictly positive reference eligibility and inclusive level, dominance and paired gates retain their previous thresholds. A completed fit describes only that block's host-time samples.

For close tick c, trial ticks c+1 through c+17 test indices 16 down to 0. Retain the largest passing index. Zero must pass. At the last trial, reset the clean count on a failed realized, reference, eligibility or neutral gate, otherwise saturate at two. The selected target first drives the slew on **c+18**. The new block has already accumulated 17 samples. With N≥194 there is at most one retired owner. Reset, disposal or fault removes it immediately. Partial windows and partial trials produce no selection. Recovery uses the same schedule as withdrawal. M6 overrides that schedule on a numerical fault.

### Prepared storage, retirement and deadline

Prepare two lane arenas and one shared arena. Use the layout calculator's byte offsets for ring capacities, feature vectors, source cursor, support stamps, typed events, eligibility, all four O2 states, projection arithmetic, monitor factor and trial scratch. Pool fixed-size working slices on the stream, including the Cholesky factor and component vector. The maximum occurs immediately below 192 kHz. Proposed analysis/control capacity is **61,376 bytes**, including 4,768 shared bytes and two 28,304-byte lanes. Four 4096-element physical rings, both families of 8192-frame scratch, JUCE stereo wet scratch and explicit guide/stream descriptor reservations bring the proposed prepared payload to **726,976 bytes**. These dimensions describe a proposed arena. Allocator usage and stack/ABI layout on a future implementation still require measurement. [Executable layout and sizes](../../tools/validation/i1_layout.py)

On ordinary disposal retire the old held feature, physical epoch, all O2/bank histories, phase, prospective/retired moments, trials, permission and wet cache. Keep HP/FIR history and a distinct pending acquisition. An affected lane fault also retires its pending and confirmed acquisition, published/completed source slot and support validity. Shared analysis and the other lane progress. Cache exactly E and P from the previous finite sample. Emit `(Nf−1−i)/Nf` times each saved value, with i=0 at detection. Finish at exact zero on detection+Nf−1. A second fault during this interval has no live epoch to invalidate. Stream reset clears the fade immediately, restarts both origins and invalidates all generation-stamped work. Use ring valid counts and generation stamps for logical clearing. Fixed 123-word blocks and fixed banks can be zeroed on acquisition of their slot. No large physical history needs clearing on a sample.

Per live lane the maximum regular tick executes one guide/O1, four O2 updates, 12 coefficient updates, 96 complex section updates, 48 weighted output terms, one phase increment, four harmonic basis evaluations and 123 moment additions of products. A change of pitch also prepares 12 pole/taper pairs. One trial evaluates three two-column fits, their three fitted/component energy comparisons and the paired/off power comparison. Closure normalizes 123 words, screens six rows, performs one lower 6×6 Cholesky (21 entries), 13 forward and 13 backward triangular solves (15 multiply-subtracts per direction per RHS), five measured comparisons and one realized paired comparison. It clears the next 123-word slot. There is no same-lane trial on that closure. Two lanes can close together, or one can close while the other executes a trial.

Include coincident A1 and I1 FIR emissions (up to 769 and 385 taps per lane), A2's K=W+1 lag updates at an A1 emission and bounded O(K+W) hop work, ≤7 source indices ×12 orders per lane, ≤12 magnitudes/logs at finalization, a due publication of up to 12 gains, both M4 launches, direct/mix conversions and shared stereo output. Old-job finalization occurs before the next snapshot's hop. A late postcommit fault can incur almost the full tick plus fixed invalidation and the first cached pair. Stream reset cancels retired work before reuse. Neither allocation, locking, I/O, retries, an unbounded search nor unbounded catch-up belongs on the proposed sample path. Offline diagnostic export is outside this bound. The 192 kHz one-sample deadline is 5.2083 µs. V21's 48 kHz/64 mean budget is 133 µs, with 667 µs p99.9. Counting bounded operations leaves their Rust callback timing open.

### Private stereo entry point

```rust
struct Lane<'a> {
    acquisition: Capture, source_ring: &'a mut [f64], support: &'a mut [Stamp<Support>],
    electrical: [O2Lane;4], bank: [CorrectionBank;4], moments: [Moments;2],
    retired: Option<Retired>, trial: Option<Trial>, permission: Permission,
    cache: WetCache, fade: Option<FaultFade>, physical: Guide,
}
struct Support { source: Token, hz: f64, quality: f64, amplitude: f64, previous_floor: f64,
                 movement: f64, supported: bool, compatible: bool }
struct Receipt { epoch: PhysicalEpoch, prepared_rate: f64, issued: u64, through: u64,
                 hz: [f64;2], speed: [f64;2], force: [f64;2], depth: [f64;2] }
enum ReceiptUpdate { Keep, Revoke, Grant(Receipt) }
struct ControllerFrame { events: EventFrame, a2: Stamp<Support>, identity: Stamp<()>,
                         receipt: ReceiptUpdate, hz: f64, speed: f64, force: f64,
                         depth: f64, signed_gain: f64, energy: f64 }
struct HostFrame { reset: bool, direct: [f64;2], character: f64, controller: [ControllerFrame;2] }
struct Stereo<'a> { generation: u64, tick: u64, prepared: PreparedI1<'a>, lanes: [Lane<'a>;2] }
fn stereo_tick(s: &mut Stereo<'_>, host: HostFrame, output: &mut [f32;2]);
```

```text
take frame at s.tick; resolve transport reset for both lanes
read the smoothed shared controls for this timestamp
for lane in left, right:
    decode frame; expire by elapsed pre-confirm age
    advance source ring and ingest due support or missing stamp
    confirm eligible owner; dispose old zero-energy epoch; launch fresh owner at zero
    finish previous job slice; consume exact due publication; start next eligible slice
    if cached fade: emit next cached pair
    else if idle: emit zero pair
    else:
        match guide.tick_observed(current controls):
            physical error -> issue Physical fault
            success -> retain converted M5 and O1, tick four O2 histories
        electrical error -> issue ElectricalAfterCommit fault
        update coefficients and four banks; advance phase
        use prior available target; advance one retired trial
        slew; form d and actual y; update packed moments
        if closing: factor, solve, classify realized, retire, arm trials
        propose signed wet, product and finite conversion
        checked failure -> issue IdentityAfterCommit(site) fault
        success -> publish lane state and cache
    on fault: invalidate once; emit fade index zero using the previous cache
    output[lane] = checked_f32((1-C*emitted_E)*direct+C*emitted_product)
advance the existing shared output stage once on the stereo pair; increment tick
```

A nonempty sample call owns one timestamp. Empty partitions execute no step. Prepared scratch belongs to the exclusively borrowed stream. The closure factor needs 35 multiply-subtractions, 15 divisions and six square roots. The 13 pairs of substitutions need another 390 multiply-subtractions and 156 divisions. Each fitted comparator evaluates 36 full-energy terms plus two four-term component energies. Five comparators run at closure and three in a scheduled trial. Include finite checks and absolute-term sums for roundoff at those sites. A2 has at most 155 lag positions per decimated emission. A failed finite conditioning test skips the solves but still consumes the bounded trial schedule.
