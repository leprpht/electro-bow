# O2 electrical pickup response

**Proposed for independent Review 6, 27 November 2026.** The input is the qualified O1 aperture velocity. The single candidate is a DC blocker, a nonresonant second-order low-pass and a small parallel return of the same carrier. Use the positive-velocity convention below. The selected product reference is a bowed electric guitar retaining DI character. The winding, magnetic flux gradient, string displacement, cable and amplifier of an incoming DI are unknown. FS is digital full scale. The conversion from m/s to FS is an engineering choice. [Review 5 authority](../implementation/status/REVIEW_05_O1_OBSERVATION.md#open-findings-and-next-authorized-task) · [Measurements and calibration limits](../implementation/status/STAGE_06_O2_SPECIFICATION.md) · [Acceptance matrix](../implementation/O2_ACCEPTANCE_MATRIX.md)

## Preparation and derivation

Proposed private Rust file `rust/dsp/src/instrument/o2.rs`. Prepare once per stream off the audio thread. Use IEEE binary64 throughout the sample recurrence, including gains. Share immutable coefficients across lanes. Use ordinary ordered operations, without reassociation or explicit fused multiply-add. Return a preparation error for nonfinite or out-of-domain rate. The supported path covers every real binary64 rate in [32000,192000] Hz. Existing host routing owns selection of the legacy path. A rate change requires fresh preparation.

With zero history, taking the z transform of the blocked velocity gives
$(1-cz^{-1})H=c(1-z^{-1})V_p$. Hence the DC zero is at 1 and the causal pole at $c=e^{-2π20/f_s}$ lies strictly between zero and one. The exact high-frequency gain at Nyquist is $2c/(1+c)$. Initial conditions matter on the first seeded sample.

For the load, divide the analog denominator by its constant term to obtain $1+\sqrt2 u+u^2$, with normalized Laplace variable $u=s/ω_0$. Substitute $u=(1-z^{-1})/[K(1+z^{-1})]$. Collect powers after multiplying by $K^2(1+z^{-1})^2$. The constant denominator term is D below. On the unit circle $u=i\tan(π f/f_s)/K$. At 3200 Hz the load therefore has magnitude $1/\sqrt2$ and phase −90°. Both analog poles have negative real part and the bilinear transform maps them strictly inside the unit circle. The resulting rational causal load has unity DC, a double zero at Nyquist and denominator $1+a_1z^{-1}+a_2z^{-2}$. The blocker slightly modifies the cascade gain at cutoff. Stability establishes bounded response for bounded input at a prepared rate. Acoustic calibration requires separate evidence.

```text
prepare(fs) -> Result<O2Coefficients, PrepareError>:
    require fs finite and 32000 <= fs <= 192000
    c = exp(-2*pi*20/fs)
    K = tan(pi*3200/fs)
    D = 1 + sqrt(2)*K + K*K
    b0 = K*K/D
    b1 = 2*b0
    b2 = b0
    a1 = 2*(K*K - 1)/D
    a2 = (1 - sqrt(2)*K + K*K)/D
    require all six stored coefficients finite
    return {c,b0,b1,b2,a1,a2}
```

## Per-lane sample and lifecycle

Proposed API `O2Lane::reset(&mut self)` and `O2Lane::tick(&mut self, coeff: &O2Coefficients, v: f64, noise: f64) -> Result<f64, OutputFault>`. The return value is q in FS. The caller supplies the owned observation's `velocity()` from the **same** successful `Guide::tick_observed`. Guide and O1 own all ring interpolation and current-period arithmetic. The separate `Sample::audio` is the original M5 signal. O2 has no input from that tap or from the final mix.

```text
# Prospective f64 locals. prev_v and prev_h in m/s, z1 and z2 in FS.
require finite v in [-8,8], finite noise in [0,1], finite old state
h = c * ((prev_h + v) - prev_v)
a = (.85 * h) / sqrt(2)
l = b0*a + z1
next_z1 = (b1*a - a1*l) + z2
next_z2 = b2*a - a2*l
q = l + (.025 * noise) * (a - l)
require finite h,a,l,next_z1,next_z2,q
commit (prev_v,prev_h,z1,z2) = (v,h,next_z1,next_z2)
return q
```

In the earlier O2 proposal the symbol η in `q=l+.025η(a−l)` means the **smoothed dimensionless Noise depth**. Here it is named `noise` to distinguish the validation harmonic **power fraction**. Both a and l already include the .85 conversion and delivery trim. Noise at 0, .5 and 1 returns respectively l, l+.0125(a−l), and l+.025(a−l). At Nyquist the nominally nulled load admits that fraction of the blocked carrier. A silent initialized history emits zero at every setting. The branch produces no independent friction-noise source. Owner acceptance of this use of public `noise` (version hint 2, default .5) is pending.

Per existing control mapping, accept a finite block snapshot, clamp the target to [0,1], initialize the stream depth to its first target, then update once per host frame by `depth += (1-exp(-1/(.015fs)))*(target-depth)`. Prepare the coefficient outside render. Both lanes use the same updated depth, including separate launch times. No extra O2 smoother. Stream reset forgets the snapshot. Launch and lane disposal leave shared smoothing and linked protection running. Zero-frame processing advances no clocks.

Reset and prepared idle set all four lane values to positive zero. A confirmed new launch resets all four under E=0 before its first observed tick. Compatible reacquisition and pitch retunes retain them. On the first Release tick the controller lifts contact (k=0), continues the live guide and load throughout the fade, then invalidates and zeros lane state at disposal. A commanded physical lift alone keeps the filter running. Idle skips O2 and supplies E=Ew=0. Transport reset clears all wet caches, fades and lane state. Use the existing host denormal guard, exact zero on disposal/reset (including inactive magnitudes below 1e−30), and the V18 standalone near-denormal benchmark. Active f64 tails have no audible gating threshold. Analysis eligibility never changes samples. Test all old states and candidates before publication. A finite O1 velocity can exceed the 4 m/s **physical outgoing-wave** guard. No audio tap uses that limit or a ±1 clip.

## Measurement taps and gain ownership

Initial and reset values of signal taps are zero with silent input. Combinational probes below own no additional histories. For a finite zero-state input let B = 8·2c, S = B times the sum of absolute discrete load impulse weights, and M = max(B,S), in m/s. These conservative mathematical bounds follow from the blocker impulse (absolute sum 2c), stable load and convex blend. They are useful unit checks. The runtime checks use the finite/domain predicates above. For direct/mix ranges use the actual finite Attack output d. Counts, peaks and engineering WAVs use original gain at every pitch.

| Tap and owner | Equation and units | Range, initial/reset or guard | Measurement |
|---|---|---|---|
| Guide M5 baseline, u | .85 times M5's own 20 Hz blocked bridge p, FS | Original M6, silent launch state reset | Authoritative converted bridge denominator, immutable telemetry col 11 |
| O1 spatial, v | observation.velocity(), m/s | [−8,8], absent at idle/fault | Col 21 vs outgoing N (col 3) |
| O2 blocked, h | c(prev_h+v−prev_v), m/s | abs ≤ B, DC pair zero at launch | DC reject, spatial-only reference |
| O2 load input, a | (.85 h)/√2, FS | abs ≤ .85 B/√2 | Scope actual gain order |
| O2 load, l and equivalent velocity L | DFII-T output l, FS. L=√2 l/.85, m/s | abs L ≤ S, zero filter histories | Electrical versus blocked spatial response |
| O2 detail, Q | Q=√2 q/.85, m/s | abs ≤ M, no added state | Intrinsic detail versus L |
| Engineering audio, U | U=.85 Q=√2 q, FS | abs ≤ .85 M | Fixed common gain, U at noise=0 is common-gain loaded |
| Delivery audio, q | l+.025 noise(a−l)=U/√2, FS | abs ≤ .85 M/√2 | Actual return. Last equality describes scaling, never a second trim |
| Voice signed wet, w | w=σ G q for neutral identity, FS. For injected stress replace q by i(q) before σG | σ=±1, G=clip(A_pk/.14,.08,1.4), captured at launch, cache silent until live. abs ≤1.4 sup abs(i(q)) | Isolate capture from coloration |
| Voice product, P | P=E w, FS | E in [0,1], zero at launch, cache last finite E and P | One envelope, recovery cache |
| A4 lane, y | (1−C E)d+C P, FS | C smoothed [0,1], finite convex normal mix, dry in idle | Direct/wet overlap, then convert y to finite f32 for Output |
| Linked Output trim, t | target=2 clamp(public Out,0,1), first frame initializes t to target, then move by at most 2/(.010fs) per frame | [0,2], unset on stream reset, .5 public is unity internal | Separate Out automation and delivery gain |
| Pre-protection, pᵢ | f64(f32(yᵢ)) t, FS | finite, abs ≤2 abs(f32(yᵢ)), input sanitization as existing Output | Per-channel peaks, common instantaneous demand |
| Processed output, oᵢ | f32(pᵢ g_link), FS | shared g_link initially 1, zero release history on stream reset, finite, ≤.980001 FS sample peak | Linked events, recovery, delivered sample |

At fixed f the engineering equivalences above follow from linearity and zero initial state. Future code computes **a before the biquad** so that its persistent state has unambiguous units. Compute scaled probes only when needed. G is dimensionless, captured from pre-swell guitar level and held for the gesture. σ is the A3 peak-bucket orientation. Factory Character and Out do not alter coil sensitivity. `G=1, σ=+1, C=E=1, t=1` exposes delivered q. No per-note normalization, wet makeup, feedback or adaptive headroom. Original-gain and listening-only scalars are recorded separately. [Routing and parameter ownership](ELECTROBOW_TARGET_ARCHITECTURE.md#a4-mix-and-host-contract) · [Linked output implementation](../../rust/dsp/src/output/mod.rs)

## Post-commit output fault and stereo order

The controller owns each lane's last finite E and P=Ew from the preceding host sample, both zero after reset. Keep the pair until an entire wet proposal passes the finite/range checks. Check prospective signed gain, E, product and mix (including f32 representability) before cache publication. If a later wet computation fails after a valid O2 tick, discard its result and reset even the newly committed O2 state. Ownership of a failing **physical** transaction remains with the qualified guide.

```text
for host frame in sample order:
    update direct and shared smoothed controls
    for each prepared lane:
        if already in fault fade: emit fade(i), advance i, skip physical/wet work
        else:
            obtain A3 command, apply launch/reset or lift if commanded
            match guide.tick_observed(hz, V, k):       # at most once
                Err: start_fade(precommit)             # guide already invalid
                Ok(None): emit (0,0)
                Ok(Some((sample,obs))):
                    retain M5 sample unchanged
                    match o2.tick(coeff,obs.velocity(),noise):
                        Err: start_fade(postcommit)
                        Ok(q): compute w,E,P and y in locals
                               if invalid: start_fade(postcommit)
                               else: cache and emit (E,P)
        y = (1-C*emitted_E)*direct + C*emitted_P
    apply existing linked Output to both finite f32 lanes and then host bypass

start_fade(origin) at n0:
    if origin == postcommit: guide.invalidate()        # successful tick is final
    reset O2 and dispose other lane-local wet/pending acquisition state
    latch immutable (last_finite_E,last_finite_P) and N=max(1,floor(.005*fs+.5))
    require fresh acquisition after fade, increment lane fault count once
    emit fade(0) on n0, next index = 1
fade(i):
    s = (N-1-i)/N
    return (s*latched_E,s*latched_P) for i=0..N-1
    after i=N-1 clear latches, caches and fade flag, enter idle
```

On an O2 detection tick its invalid prospective state never commits. The preceding successful guide commit stands and its epoch is immediately invalidated. There is no second guide tick. Repeated faults cannot restart or extend the same fade. A fault during a fresh later epoch starts a new fade. At 44.1 kHz the first decrement occurs on detection and the forced zero 220 samples later. The other lane progresses normally, including its own events and cache. The linked protector can attenuate both outputs. Inspect unaffected progression before linking. [M6](BOWED_STRING_MATHEMATICS.md#m6-energy-release-and-floating-point-limits)

## Local feasibility estimate

Four f64 per lane (32 bytes) plus six f64 prepared once (48 bytes). Stereo persistent O2 payload 112 bytes before enclosing layout. Reset is four scalar zero stores per lane. For the pseudocode per live lane count blocker 1 multiply/2 adds, conversion 1 multiply/1 constant division, load 5 multiplies/4 adds or subtracts, detail 2 multiplies/2 adds or subtracts. At stereo double these counts. At ingress check two ranges and four old state values, then six prospective finite values per lane. A common finite range predicate can reject NaN. Shared existing smoothing is one multiply/two adds for Noise per frame regardless of lane count. Coefficient preparation owns exp/tan. Sample path requires no heap, I/O, synchronization, blocking or data-dependent iteration. Count estimates exclude guide, I1, controller, A4/Output and host. Group delay is frequency-dependent and measured in the acceptance report. There is no new dry transport delay. Existing O1 timings cannot establish callback deadlines. V18–V21 remain integrated obligations.
