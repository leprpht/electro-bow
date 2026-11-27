# I1 · Source identity on the bowed electric-guitar carrier

**Proposed for independent Review 8, offline evidence only.** The selected design compresses observed DI harmonic contrasts, drives a complex correction bank with the actual returned O2 sample, and admits a common fraction of the requested correction from recent measurements of the physical carrier. A separate measurement of delivered coloration can withdraw that permission. The original converted M5 reference, O1 geometry and O2 electrical transaction retain their Review 7 meanings. [Physical and electrical boundary](../implementation/status/REVIEW_07_O2_IMPLEMENTATION.md) · [Engineering gates](../implementation/I1_ACCEPTANCE_MATRIX.md)

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

At a valid publication update the owning gesture's I in dB by I←I+[1−exp(−HD/(fs τ))](T−I), τ=.060−.040m s, using smoothed Movement m∈[0,1] sampled at snapshot. On every live host tick, after events/publication, update each **linear correction coefficient** c←c+[1−exp(−1/(.015fs))](expm1(ln(10) I/20)−c). New live bank c starts at zero, including when a pending I is promoted. On Hold or ordinary Release keep I and finish the slew. A monitor gate can still attenuate c. A publication elapsed across skipped frames uses one coefficient step, without accumulated catch-up. Separate hard-corner tests inject constant c at launch to exercise the entire box.

## Host clock and gesture ownership

Use absolute sample counter n and generation origin o. Empty host calls advance nothing. Reset on n invalidates all earlier work, sets o=n, then n is the first input of the new stream. Use checked u64 differences and an i128 doubled center for comparisons. A stream-clock overflow retires the lane and requires prepare. All ranges here have inclusive endpoints unless written [a,b).

| Object | Exact host time and evidence |
|---|---|
| Decimated j | Last available input e(j)=o+(j+1)D−1. Its FIR uses HP values [e(j)−32D,e(j)], effective center e(j)−16D. |
| Snapshot s | At s=o+iHD−1, i≥1. J=iH−1. Left FIR end l=s−(M−1)D, raw FIR support [l−32D,s], first/last centers l−16D and s−16D. Feature center numerator l+s−32D over 2. Snapshot stamp is s even if the center is fractional. |
| First eligible snapshot | Smallest such s with l−32D≥max(o,event), l−16D−event≥ceil(.025fs), valid stamped input frames and ≥2 supported orders. FIR support includes every integer HP sample in the specified interval. The IIR HP itself depends on raw history from o onward. An onset never resets the shared HP. |
| Pitch evidence | A2 interval P=round(.005fs/Dₐ)Dₐ, Dₐ=floor(fs/8000). Use endpoints o+iP−1. Include the last endpoint at or before l−16D through the last endpoint ≤s, and every scheduled endpoint during the job. Each is present, for the same source token, explicitly compatible or confirmed, Q≥.74, A≥max(.001,3N₀), f∈[65,1320], supported. Require absolute 1200 log₂(f_frame/f_snapshot)≤25 cents. A missing due frame invalidates continuity. Initial acquisition/confirmed change additionally obey A3's starter rules in the controller. |
| Sliced job | B=ceil((ceil(4F/65)+1)/(HD)). Start at s inclusive. Take at most B chronological window indices per tick. Finish and finalize at s+ceil(M/B)−1, always before s+HD. Retain a separate completed slot until exactly s+HD. A publication consumes only this due result. A late completion is discarded. History capacity ceil(4F/65)+1+H+2 prevents overwrite while calculating. |
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
