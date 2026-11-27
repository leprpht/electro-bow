# Discrete bowed string for ElectroBow

This is the normative specification for **one** voice. A trailing guitar analysis supplies frequency, dynamics and articulation. The mechanical model runs at host sample rate. Values below describe a first calibrated engineering range. The isolated numerical checks in the [validation plan](../implementation/DSP_VALIDATION_PLAN.md) use the stated contact, losses and initial history. Plugin performance and listening acceptance remain separate gates.

## M1. From a vibrating string to a causal delay

Let x be distance along a flexible string (m), t time (s), y transverse displacement (m), L speaking length (m), T tension (N), λ mass per unit length (kg/m). Assume constant T and λ, small slope, a point contact, one transverse polarization and fixed endpoints. Local momentum balance on dx gives λ y_tt = T y_xx away from the bow. Hence c=√(T/λ) m/s, y=u(t−x/c)+w(t+x/c). Use **velocity** waves, the time derivatives of u and w, with the same positive transverse direction for both. Their sum is physical transverse velocity v. The slope-related force T y_x is Z times the difference of the two velocity waves (left-going minus right-going), with transverse impedance Z=T/c=√(Tλ) in N·s/m. One right-going wave A carries average power Z·mean(A²) W in its travel direction. A rigid termination enforces v=0, reflecting velocity with sign −1. Two such reflections give positive round-trip polarity. The period is 2L/c, f₀=c/(2L), delay P=fs/f₀ **samples per complete circuit**. These equations omit string stiffness, torsion, bow width, moving fingers and coupled body modes. [Smith, string models](https://www.dsprelated.com/freebooks/pasp/String_Models.html), [Woodhouse, returning waves](https://euphonics.org/9-2-1-time-domain-simulation-of-a-bowed-string/)

A point pluck at x=ξL (0<ξ<1), displacement height h, piecewise straight sides, initial velocity zero, gives freely decaying modes sin(kπx/L) with initial displacement coefficients 2h sin(kπξ)/(π²k²ξ(1−ξ)), k≥1. This follows by integrating the triangle against 2 sin(kπx/L)/L. Pickup position, velocity sensing and losses modify the observed spectrum. The entire available initial energy is finite. Bowing instead supplies a localized force F(t)δ(x−x_b) on the right of the wave equation. Work ∫Fv dt can replenish losses every period. The returning waves determine each new force. In ideal Helmholtz motion a corner travels around the string once per period. At a bow a fraction β of L from the bridge, v stays near bow speed V for fraction 1−β, then slips back at approximately −V(1−β)/β for fraction β, giving zero mean displacement drift. The rapid change in velocity generates phase-related harmonics. Bridge force is approximately sawtooth-like, with ideal harmonic magnitudes proportional to 1/k. This describes a regime, not a required spectral slope at a guitar pickup. [Woodhouse, Raman's argument](https://euphonics.org/9-1-2-ramans-argument-for-bowed-string-waveforms/)

A stable linear filter driven by white noise of PSD S has output PSD |H|²S. Its resonant peaks have finite bandwidth and random phase. Multiplying noise by the input can shape its level but does not synchronize one stick/slip event to each round trip. A pluck supplies estimates of periodicity, level, attack, decay and spectral balance. Bow hair pressure, intended bow direction, exact sounding string of a unison, and intent to continue bowing after silence are latent choices. The control rules in the [architecture](ELECTROBOW_TARGET_ARCHITECTURE.md) make those choices explicit.

## M2. Arm routing, losses and fractional phase

Use two circular histories of **outgoing** velocity, B toward the bridge and N toward the nut. Keep β=.127 fixed for the first release. From the bow, the physical bridge distance is βL and the nut distance (1−β)L. Fold each outbound and reflected path into a single round trip. Put one symmetric loss FIR on the returning bridge arm. For all synthesis rates 32–192 kHz inclusive and 65–1320 Hz, define

```text
P = fs/f
D_B = β P − 1                     D_N = (1−β)P
tap(U,D,n) = (1−u) U[n−m] + u U[n−m−1],   m=floor(D), u=D−m
x[n] = tap(B,D_B,n)
b[n] = −g(.125 x[n] + .75 x[n−1] + .125 x[n−2]),  g=.98
a[n] = −tap(N,D_N,n)
```

Here a comes **from the nut**, b **from the bridge**. Perform every read before writing B[n],N[n]. No implicit last-output delay exists. For ω=2πf/fs rad/sample, the bridge FIR without its sign has response g exp(−iω)(.75+.25cos ω), phase delay exactly one sample, magnitude ≤.98. Thus the two nominal segment delays plus the FIR phase are P. Reflection signs cancel. Applying the correction to the bridge arm preserves the physical contact position. For fixed D the interpolator response is exp(−iωm)[(1−u)+u exp(−iω)]. Its magnitude is ≤1; phase delay is m+atan2(u sin ω,1−u+u cos ω)/ω. Using u=D−m gives a small residual, not an extra whole sample. At the worst ω=2π·1320/32000, the maximum absolute difference between u and this fractional phase delay is below .0011 samples per arm (evaluate u over [0,1], or differentiate the atan). Nonlinear pitch flattening can be much larger. Validate **active** pitch as well as passive tuning. [Faust reflection FIR](https://github.com/grame-cncm/faustlibraries/blob/cd7555d111e7f35a981af96068dea8b44d33607b/physmodels.lib#L701-L708), [STK linear interpolation](https://github.com/thestk/stk/blob/6aacd357d76250bb7da2b1ddf675651828784bbc/include/DelayL.h#L120-L179)

At the shortest period even D_B >2 and the pickup tap βP/2 >1. Allocate each ring for ceil(fs/65)+4 samples (rounding up to a power of two is optional). Indices wrap by a bounded conditional or mask. Keep time counters distinct from ring indices. Reads when changing pitch always use the current delay and old waves; retain FIR state. Linearly interpolated reads cross integer boundaries continuously. A changing delay is time-varying and can shift frequency or change energy. Keep β fixed; transmit measured bends through P. Never reset phase on a regular analysis hop.

**Control-rate trajectory.** With H analysis samples/hop, decimation D and hop length J=HD host samples, accepted pitch controls are continuous Hz. At each decision compute Δ=clip(log₂(f_target/f_current),−4J/fs,+4J/fs) octaves, replace the current multiplicative ramp by multiplier 2^(Δ/J) for J samples. Integrate from the current value and hold the endpoint; a new target interrupts at the current value. Initialize exactly to the confirmed frequency only when launching a silent voice. This 4800 cents/s cap passes typical bends and vibrato, with up to one additional hop of interpolation. Changes beyond the rearticulation threshold use a faded new voice. Compute reciprocal period with an inverse multiplier or division; never quantize to notes. Clamp endpoints to [65,1320]. Reject invalid targets before log or division.

## M3. Force and scattering at a massless point

Let F be the transverse force on the string in N, normal bow force F_N≥0 in N, V bow speed in m/s, r=V−v actual signed slip in m/s. Let d=V−a−b be the mismatch **before** applying force. Continuity equates the two waves summed on each side of the bow, v=B+b=N+a. The free solution is v₀=a+b. Each semi-infinite arm reacts to an incremental velocity j with impedance Z, so F=2Zj and

```text
v = a+b+j           B = a+j           N = b+j
r + F/(2Z) = d      j = d−r           k = F_N/(2Z).
```

Z is constant in this normalized virtual instrument. Use Z=1 N·s/m as a scale convention; the engine only requires k (m/s). It does not estimate real guitar tension. With no contact (k=0), set j=0, clear slip memory and propagate. In sticking, r=0 and F=2Zd provided |d|≤k μ₀. In sliding, F=sgn(r) F_N μ(|r|); hence |d|=q+k μ(q) with q=|r|>0, sign(r)=sign(d). The unknown actual slip appears on both sides of the original load line. Evaluating the friction coefficient directly at d would be incorrect. [Smith, bow scattering](https://www.dsprelated.com/freebooks/pasp/Bow_String_Scattering_Junction.html), [Woodhouse, eqs. 4–5](https://euphonics.org/9-2-1-time-domain-simulation-of-a-bowed-string/)

**M3a. Discrete constitutive law.** Take a piecewise-linear interpolation of the published positive-slip fit, samples in m/s at qᵢ=(0,.005,.01,.02,.04,.08,.16,.32,.64,1.28). At prepare time calculate μᵢ=.35+.85 exp(−qᵢ/.01)+.45 exp(−qᵢ/.1), dimensionless. Use μ₀=1.65, the one-sided limit of that equation. Extend μ as the constant μ₉ above 1.28. Use odd signed force and a vertical static interval at q=0. This specific discretization preserves the decreasing, convex curve; the knot spacing and saturation of the tail are engineering choices. Use f64 for feedback and contacts. Normal control V=.06… .16 m/s and k=R·V/μ₀ with dimensionless static threshold ratio R=1.2…2.4. Thus F_N=2Zk and |j|≤R V≤.384 m/s. Pressure and velocity are coupled by policy to keep the playing point in range, though isolated junction tests can vary them separately.

**M3b. Bounded numerical solution.** Store previous mode as Stick or Slide(sign). Define h=abs(d), s=sign(d) and hᵢ=qᵢ+kμᵢ. The piecewise-linear function q+kμ(q) is convex, tends to infinity, and can initially decrease. The descending portion has negative incremental load-line slope; use the upper, increasing slip branch. If the previous mode was Stick or the slip sign changed, choose static whenever admissible. Otherwise follow the upper slip branch as long as it exists. This implements hysteresis at multiple intersections. A transition to the opposite sign can still require slipping when the static force limit is exceeded. [Desvages, Fig. 4.9](https://www.acoustics.ed.ac.uk/wp-content/uploads/Theses/Desvages_Charlotte__PhDThesis_UniversityOfEdinburgh_2018.pdf)

```text
if d or k is nonfinite, k < 0, or required curve/load arithmetic is nonfinite: return Fault
if k == 0: return (0, Stick)
if d == 0 or (previous != Slide(s) and h <= k*mu0): return (d, Stick)
if h == h9: q = q9
else if h > h9: q = h − k*mu9
else:
    search i = 8 down to 0, at most 9 segments
    choose first with h[i] <= h <= h[i+1]
    if found and h == h[i+1]: q = q[i+1]      # includes coincident endpoints
    else if found:
        u = (h−h[i])/(h[i+1]−h[i])            # 0 <= u < 1 before rounding
        q = q[i] + (q[i+1]−q[i])*u
    else:
        if h <= k*mu0 + 1e−12: return (clamp(d,−k*mu0,k*mu0), Stick)
        otherwise raise numerical fault
if q <= 1e−12 m/s: return (d, Stick)
return (d − s*q, Slide(s))
```

Each hᵢ is the stored binary64 sum of qᵢ and the separately rounded product kμᵢ. Use ordinary f64 multiply, add, subtract and divide in the order shown. An enclosing pair with distinct increasing loads is admissible at **any representable width**. Descending pairs cannot satisfy the ordered enclosure. A coincident pair intersects only at its common load and contributes its upper slip endpoint. Searching from high q selects the maximum admissible intersection, including ties with neighboring segments and the constant tail. The enclosed numerator bounds the interpolation fraction in [0,1]. There is one division on the selected interior segment and at most nine probes.

The rejected width cutoff of 1e−12 m/s lost a root at k=.12029965498642688 and h=.12026541114915246. The stored loads span 5.000028169277471e−13 m/s. From Slide(+) they give q=.030000277554192455 m/s by the f64 operations above. Forming the endpoint loads in exact arithmetic from the same prepared knots gives approximately .03000064466 m/s. The inverse slope amplifies endpoint rounding near a fold. V02 compares against intersections of the stored graph. [Review counterexample](../implementation/status/REVIEW_00_1_SPECIFICATION.md) · [Implementation replay](../implementation/status/STAGE_01_CONTACT_IMPLEMENTATION.md)

V02 uses the signed load-line residual |r+j−d|<1e−10 m/s on every finite result. With k=0 the returned Stick tag clears branch memory, j=0 and physical r=d. With k>0 on Stick the graph at r=0 is the static interval: require |r|<1e−10, |j−d|<1e−10 and |j|≤kμ₀+1e−10 m/s. On Slide require q>1e−12, sign(r)=sign(d)=stored sign, |q+kμ(q)−h|<1e−10 and the same force bound. Check the chosen upper root independently of these residuals. The 1e−12 static fallback margin applies only after exhausting the search. The separate 1e−12 slip floor applies only after selecting q. Fault is a distinct solver result with no usable wave or branch to commit. M6 owns the audible response.

**Worked branch check.** At Z=1, k=.10 m/s and d=.14 m/s, the static limit is .165 m/s. From Stick, j=.14, r=0 and F=.280 N. From Slide(+), the same inputs intersect the increasing segment between q=.08 and .16 at r=.08552224 m/s. Then j=.05447776 m/s and F=.10895552 N. Both choices satisfy the load line. This case catches a solver that always chooses the static root, reverses the slip sign, or uses the full force as a velocity correction.

Additional unit-test vectors at Z=1 follow. Branch signs refer to the previous and resulting samples. Force is 2j.

| d (m/s) | k (m/s) | Previous | New mode | j (m/s) | r (m/s) |
|---:|---:|---|---|---:|---:|
| −.14 | .10 | Slide(+) | Stick | −.14 | 0 |
| −.20 | .10 | Slide(+) | Slide(−) | −.0447478545412210 | −.155252145458779 |
| .14 | 0 | Slide(+) | Stick (cleared) | 0 | .14 |
| .2726217499930773 | .3486472674315129 | Slide(+) | Slide(+) | .1926217499930773 | .08 |

The last vector records the reference stack. For a target-prepared exact-tie fixture, start at k=(.08−.04)/(μ(.04)−μ(.08)) and search up to 64 adjacent binary64 values each way for equal stored endpoint loads. Use their common h and both signed adjacent queries. Choose its upper endpoint on the adjacent increasing branch. At h₉ the constant-tail and last-segment roots coincide. A signed zero mismatch gives j=r=0; nonfinite d even with k=0, negative/nonfinite k, invalid intermediates, or exhaustion without an admissible static interval produce Fault. The positive .14 Stick example has sliding-curve discrepancy .025 m/s because static force can take any value inside the interval.

## M4. Launch from a physically consistent history

A zero-state start with a suddenly moving bow can settle into multiple slips per cycle. On cold acquisition or deliberate rearticulation, initialize the *history* of an ideal lossless Helmholtz orbit at the confirmed f and starting V. This represents energy associated with the detected stroke. Ongoing energy must then come from the solved force. No periodic seed is injected after startup.

Write S(t)=frac(t)−.5, frac(t)=t−floor(t) also for negative t. Choose origin at the start of slip and A=V/β in m/s. The required outgoing histories for integer sample times n<0 are

```text
Bseed[n] =  A S(n/P)
Nseed[n] = −A S(n/P + 1−β).
```

In the ideal guide v=B(t)−B(t−βP), giving V except during the first β of a cycle where it gives V−A. The shifted negative neck ramp gives the same velocity by v=N(t)−N(t−(1−β)P). Thus these are consistent traveling waves, without a pickup discontinuity being fed to the listener. Initialize FIR memory x[−1], x[−2] from the interpolated Bseed at the specified D_B. Start contact memory as Slide(+1), with a positive virtual bow. If analysis selected a negative polarity, apply it at the output, keeping the bow convention positive.

For bounded callback cost, implement **lazy negative-time reads**. Keep launch P,V, write index and count of samples written since launch saturated at capacity. On a read at integer lag l≥1 before writing the current sample, return the ring value if l≤count, otherwise evaluate the appropriate seed at n=count−l. Interpolate the two integer reads normally. The previous FIR values need the same rule shifted by −1,−2. No runtime clearing or filling of O(fs/f) samples per pick. All later reads beyond written history continue to use the original launch values until the ring covers the maximum allowed lag. Idle/fault uses an invalid-history flag with zero reads; new confirmed launch installs a fresh epoch. Full transport reset may clear preallocated memory. Always fade in after launch. The artificial initial condition sacrifices the fine noise of a real bow attack for dependable harmonic capture.

## M5. Output, body and guitar identity

Tap outgoing B at delay βP/2 before either write. Call the value p (m/s). At a rigid bridge, total transverse velocity cancels and force magnitude is twice impedance times incident velocity. A single incident-wave tap is therefore proportional to bridge force, with an arbitrary global polarity. The lumped round-trip FIR approximates distributed and bridge losses. This midpoint tap is a bridge-force proxy. Apply a 20 Hz DC blocker to p using y[n]=c(y[n−1]+p[n]−p[n−1]), c=exp(−2π·20/fs). Reset its state on launch under zero wet gain. Convert to audio units using nominal factor .85 FS/(m/s). Level matching for quiet notes and stereo polarity are specified in the target. Never feed the colored or limited sample back into the string.

Feed the resulting u through a separate causal four-band splitter with the existing crossover coefficients. With sᵢ its signed bands, reuse the eight existing modes and their gains/center frequencies. Keep the saturation blend on the middle bands. Modify the body scatter sent to the upper pair to .08 times its current modulation value (the existing modulation already includes .80·Noise); keep its multiplication by s₀+s₁. Let C be the old pickup sum with the revised scatter and existing movement. Use w=.85u+.15 C. The coefficients are a starting calibration: 85% of the bridge source survives in the wet branch at every Character value. Bound body coloration experimentally in the validation suite and measure with noise disabled first. Bowing still works with body gain, scatter and direct gain all set to zero except the bridge tap. Input-derived dynamics and the shaped direct contribution retain guitar identity; the spectral envelope of a completely decayed pickup cannot be recovered from pitch alone.

## M6. Energy, release and floating-point limits

The net wave power leaving the junction is Z[(a+j)²+(b+j)²−a²−b²]=2Zj(a+b+j)=Fv. For V02 verify |net flux−Fv|<1e−10 W and |F|≤F_N μ₀+2e−10 N at Z=1. Work from an external virtual bow is FV; F(V−v)=Fr≥0 is frictional dissipation. A stationary bow cannot supply positive net energy in this point model. Periodic steady state requires positive cycle-average string work equal to reflection/interpolation losses, a return near one string period, and a contact regime that alternately sticks and slips. Force bounds alone cannot guarantee that regime. In free decay the loop magnitude at fixed frequency is at most g per circuit. At 65 Hz ideal amplitude T60≈ln(.001)/(65 ln(.98))=5.26 s; interpolation shortens the decay of high harmonics. The musical release uses a shorter output envelope.

At fixed bounded controls the interpolation and FIR are convex averages and the nut has unit reflection magnitude. If |j|≤J, sup norms satisfy |N|≤g sup|B|+J and |B|≤sup|N|+J after including initial histories. Thus an invariant bound exists, e.g. B_bound≥2J/(1−g), N_bound=gB_bound+J with both bounds also covering the seed and preloaded FIR. This loose bound even applies to valid time-varying reads; it makes no assertion of musical output. Keep a stricter **fault threshold** of 4 m/s absolute for either outgoing wave, pickup or loss state. If a value is nonfinite, exceeds this guard or the solve fails, invalidate the affected wave epoch immediately, before storing waves or rendering that tick. For a fault at host sample n₀ take the last finite lane wet product E·w and its E from n₀−1 (zero from reset). For i=0…N−1 output both cached values scaled by (N−1−i)/N, N=max(1,floor(.005fs+.5)). The first decrement occurs at n₀, the first forced zero at n₀+N−1. Use the cached product directly in A4. Never extend the fault fade on a repeated fault. The unaffected lane advances normally and linked output protection still processes the final mix. Require fresh acquisition for the invalidated lane. Keep a diagnostic counter in test builds. Never use regular clipping of the internal waveform to pass a stability test. A nominal note must generate zero faults, sample-peak limiting should seldom engage.

On loss of control confidence freeze the last pitch for the finite hold specified in the architecture. At release lift the bow (k=0) and fade the rendered wave to zero before invalidation. With silence from reset there is no launch, no stored wave, and exact zero wet output. Use finite checks on control ingress and ring accesses only after bounded indexing, f64 states, f32 I/O, the existing host denormal guard and explicit zeroing of very small inactive states (<1e−30). The DC blocker and meters are outside the feedback loop.

## M7. One audio tick, in dependency order

```text
features = analysis.advance(raw_sample)        // emits only on its own sample clock
cmd = voice.advance(features, raw_envelopes, controls)
if cmd.launch: install_M4_history(cmd.hz, cmd.speed)
if wave_live:
    P = fs / smooth_frequency_one_sample()
    x = B.read(β*P−1)
    a = −N.read((1−β)*P)
    b = −.98*(.125*x + .75*x1 + .125*x2)
    p = B.read(β*P/2)
    V = smoothed_speed
    k = contact_enabled ? smoothed_ratio*V/1.65 : 0
    (j, branch) = solve_M3b(V−a−b, k, previous_branch)
    check(a+j, b+j, p, x, j)
    B.write(a+j); N.write(b+j); x2=x1; x1=x
    u = .85 * dc_block(p) * captured_output_scale * polarity
    w = .85*u + .15*body.pickup(split_wet(u), revised_scatter, blend, saturation)
else:
    w = 0
y = mix_with_shaped_guitar(w, cmd, character)  // target architecture A4
apply_linked_output_protection_after_all_channels(y)
```

Booleans and velocities are per voice, host controls and body modulation share a frame clock. React to a fault before committing new wave state. The gain/mix implementation must honor a pending release or fault ramp even when wave history becomes invalid. See [migration order](../implementation/ELECTROBOW_MIGRATION_PLAN.md) for isolated checks before routing this tick to a host.
