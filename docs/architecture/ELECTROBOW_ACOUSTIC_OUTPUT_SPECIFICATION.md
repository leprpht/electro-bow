# Proposed magnetic observation and acoustic output

**Status, 27 November 2026.** The owner selected a pickup-led bowed electric guitar retaining incoming DI character. This is one proposed implementation for independent review. The approved [M1–M7](BOWED_STRING_MATHEMATICS.md) and [A1–A4](ELECTROBOW_TARGET_ARCHITECTURE.md) govern existing qualification. Geometry, bandwidth, feature compression and gesture durations below are reproducible engineering settings. Their perceptual calibration is open. [Measured evidence](../research/BOWED_TIMBRE_ACOUSTIC_RESEARCH.md) · [Source study](../research/GUITAR_TIMBRE_PRESERVATION.md)

## Signal flow and clocks

```text
sanitized signed guitar, per lane
 ├─ original complementary bands ─ linked Attack ──────────────── direct d
 ├─ A1 raw levels, orientation, onset, A2 continuous Hz/Q ── A3 voice command
 └─ pre-swell HP + identity decimator ─ partial envelopes ─ twelve held gains
                                                             │
command Hz,V,R ── qualified M2–M4 contact ⇄ ring histories     │
                         ↓ read-only nut-arm velocity         │
                spatial pickup → DC block → loaded response → harmonic gains
                                       → captured G, sign → wet w
d, w → A4 character / wet envelope → stereo-linked Out and protector → host
```

Use one lane per original channel and sample order independent of host block size. Input bands, A1 fast/slow, brightness, peak buckets, noise floor, onset/mute confirmation, A2 Hz/quality interfaces, pitch limits, M2 log-frequency slew and M6 fault fade follow the named existing specifications. Use accepted continuous Hz only. A2's final selector is blocked. Inject frames at this boundary in isolation. Keep the 32–192 kHz inclusive range, prior low/high-rate fallback, neutral reconstruction, v5 state writer/older readers, six controls, ABI, bypass and zero compensable latency. Planned host-tail work remains S6H.

## O1. Reconstruct the sensed quantity

Let s=x/L increase from the bridge (0) to rigid nut (1), bow at β=.127. At tick n, P=fs/f[n] in samples. N is the outgoing velocity **toward the nut**, telemetry column 3 (zero based). For one station on the nut side, let d₁=(s−β)P/2, d₂=(2−s−β)P/2. The second ray has gone to the nut and returned with velocity sign −1. Use v(s,n)=I(N,n−d₁)−I(N,n−d₂) in m/s. At the nut the two reads cancel. At the bow the short read is current outgoing N and the long read is the negative of incoming a, giving N+a. This construction avoids assigning the bridge FIR to a fictitious half-trip. Under a bend this is the guide's instantaneous normalized coordinate model. It approximates a pickup at an effective fraction of vibrating length. A fixed guitar pickup moves in fractional coordinates when a player changes frets, and DI cannot identify the string and fret uniquely.

Use Simpson weights (1,4,1)/6 at s=(.162,.174,.186). For each d=m+u, I=(1−u)N[n−m]+uN[n−m−1]. Reuse the committed nut ring and its original launch history. **After a successful tick**, its next write refers to n+1. Use integer lags floor(d)+1 and floor(d)+2 with the original fraction of d, including d<1. The successful transaction supplies P. At 32 kHz/1320 Hz the minimum d is .424 samples. The longest d is .8555P, and its shifted neighbor fits ceil(fs/65)+4. The ring's saturated count and retained seed already implement negative time. Accumulate vₚ=Σwᵢv(sᵢ,n) in f64. Convex interpolation and |N|≤4 imply |vₚ|≤8 m/s. No new wave writes, ongoing oscillator, feedback coupling or prefilled buffer. At the isolated O1 boundary a valid cleared tick returns no sample, and an invalid control or physical fault returns M6 Fault with no observation. Fixed O1 reads on a successful live tick are infallible. Future output stages own their post-commit fault handling and silent presentation. [Isolated contract](../implementation/status/STAGE_05_O1_OBSERVATION_IMPLEMENTATION.md)

## O2. Loading, detail and unit conversion

Review 5 qualified O1. The [Stage 06 O2 contract](ELECTROBOW_O2_PICKUP_RESPONSE_SPECIFICATION.md) freezes the proposed per-lane filter, units, lifecycle and post-commit fault behavior for Review 6. Its [acceptance matrix](../implementation/O2_ACCEPTANCE_MATRIX.md) retains the original V17 denominator. Owner calibration and Noise semantics remain open.

The sign of the coil and connection is a convention. Choose positive sensed velocity for positive DI orientation, apply captured sign once at the end. Without flux sensitivity or recorded voltage calibration use .85 FS/(m/s) for the common-gain data. Use a further fixed 1/√2 nominal gain for delivered pickup (equal-power two-wave convention and headroom, independent of note). Equal power of the two rays is an approximation. Publish the common-gain result before this scalar as well. No automatic normalization.

For vₚ initialise previous input and DC state to zero at silent launch. With c=exp(−2π20/fs), h[n]=c(h[n−1]+vₚ[n]−vₚ[n−1]). Set a=.85h/√2. Electrical response is a damped two-pole low pass L(s)=ω₀²/(s²+ω₀s/Q+ω₀²), nominal fₑ=3200 Hz, Q=1/√2. A coil inductance and shunt capacitance can produce ω₀≈1/√(LC), with winding and external loads setting damping. These constants describe a smooth loaded surrogate. No measured guitar electronics or radiation impulse is embedded. At fs prewarp g=tan(πfₑ/fs), D=1+g/Q+g², b=(g²,2g²,g²)/D, a₁=2(g²−1)/D, a₂=(1−g/Q+g²)/D. Define DFII-transposed l=b₀a+z₁, z₁←b₁a−a₁l+z₂, z₂←b₂a−a₂l using the old z₂. Zero states on reset/launch. Both poles strictly inside the unit circle by bilinear mapping of the LHP. Pass DC in L, reject DC in the preceding blocker.

Noise η∈[0,1] admits a bounded fraction of the actual high-frequency bow detail suppressed by the load, q=l+.025η(a−l). The .025 factor limits the unfiltered parallel path to −32 dB at full scale depth, before the source-dependent filters. Measure the **relative** noise-on residual under V17. This knob injects no independent energy into a silent input. The choice to assign it the sound's intrinsic rough detail needs owner control review.

## I1. Causal source feature and confidence

**Historical design reviewed in Reviews 4–7.** The following feature equations are the starting point. Stage 08 supersedes the independent output gain, capture clock and publication rules in this section and the execution sketch below with one proposed coupled law and realized-output veto. The amended gesture remains separately held. [Current I1 proposal](ELECTROBOW_I1_SOURCE_IDENTITY_SPECIFICATION.md)

Per lane retain the A1 30 Hz HP output of original sanitized guitar (FS). Use a second analysis rate F=fs/Dᵢ with Dᵢ=floor(fs/16000)≥2, F∈[16000,24000). Share the HP computation. Apply A1's unity-DC Blackman sinc formula with Dᵢ, cutoff .34F and length 32Dᵢ+1. Emit decimated value j after host sample (j+1)Dᵢ−1. Identity hop H=max(1,floor(.005F+.5)); frame ends at j=(i+1)H−1. Reserve Mmax=ceil(4F/65)+1 past decimated samples plus H+2 for scheduled work. No spectral transform touches the direct audio.

On a hop snapshot the most recent confirmed/compatible pitch f in Hz, A3 support, level, event timestamp, floor N₀ in FS and epoch. Use only complete, same-gesture data. Reject frames unless supported, Q≥.74, A≥T_keep and the first sample center of the window (subtract FIR center delay 16/F) is ≥25 ms after the event. Require all A3 pitch frames over this window, including the last accepted pitch at/before its start, to have support and to lie within 25 cents of the current f. Otherwise retain the previous vector. This veto limits mistuning of upper partials during fast bends/vibrato. Start with zero vector (neutral). Before a new event is confirmed, collect only for the pending gesture in separate state. A stale frame from the old epoch cannot update the new voice. Timer and angle wraps use stream-relative integer sample indices. Silence before start is excluded.

Set M=floor(4F/f+.5), K=min(12,floor(.28F/f)). Let xₘ be the M samples in time order (oldest first), μ their arithmetic mean, hₘ=.5−.5cos(2πm/(M−1)). Define

```text
A_k = 2 |Σ h_m (x_m−μ) exp(−2π i k f m/F)| / Σ h_m       k=1…K  [peak FS]
ε = max(1e−5 FS, 3N₀, .015 max_k A_k)
r_k = clip((A_k−ε)/(3ε),0,1)
b_k = 20 log10(k max(ε,A_k) / 1 FS)
δ_k = b_k − Σ r_i b_i / Σ r_i
```

Require at least two positive r, finite denominator and all finite samples. Otherwise skip update. Projection phasors may use unit complex rotation restarted at m=0 for each frame. A Hann sinusoid can use one bounded rotation for its phase. Calculate both the projection of x and of unity against each modulated window, then subtract μ times the latter at finalization to avoid a second pass. Count all terms even if coefficients repeat. Subtraction of the weighted mean removes absolute gain. The factor k removes a nominal 1/k reference tilt, so a spectrally neutral saw-like source sets nearly equal gains. This slope is a reference for compression, not a law for plucked strings.

For k≤K set Tₖ=clip(.45 rₖδₖ,−5,+5) dB; set remaining entries through 12 to zero. These are **harmonic-order** features. In log amplitude this is a compressed source envelope and a scalar normalization; 45% contrast and ±5 dB endpoints limit imposition of plucked excitation. Zero reliability contributes a neutral target; a wholly unreliable frame freezes the vector. No gain is obtained by dividing by a measured weak partial. Deep individual nulls have limited or no transfer. No interpolation across an unobserved hole.

At feature publication, Iₖ←Iₖ+(1−exp(−H/(Fτ)))(Tₖ−Iₖ), τ=.060−.040m seconds, m=smoothed Spectral Movement. A higher setting follows genuine changes faster. Set target gains g*ₖ=10^(Iₖ/20), range [0.56234,1.77828]. Per host sample gₖ←gₖ+(1−exp(−1/(.015fs)))(g*ₖ−gₖ). At launch initialize g to the prepared pending gain, otherwise unity, under zero wet gain. Hold/Release freeze I and complete slews. A changing knob alone cannot refresh support. Reset I=0, g*=g=1 on disposal. A failed feature estimate skips only that frame. Invalid wet filter state invokes M6 lane recovery.

### Apply the feature to the bowed sound

Use the current **guide** pitch f on every sample (including M2 retune slews). For k=1…12, νₖ=kf Hz, a frequency taper tₖ=clip((.28F−νₖ)/(.03F),0,1) fades correction across the edge of reliable analysis. Define ρ=exp(−2π·.22f/fs), pₖ=ρ exp(i2πνₖ/fs). Maintain two complex states per harmonic. In this order:

```text
z1[k] ← p[k] z1[k] + (1−ρ) q
z2[k] ← p[k] z2[k] + (1−ρ) z1[k]         // updated first stage
w = σ G [ q + Σ (g[k]−1) t[k] · 2 Re(z2[k]) ]
```

Initialize z1,z2 to zero on silent launch/reset and clear on disposal. Run all twelve stages while the lane is live so that a partial crossing the analysis band edge has continuous history. Active frequencies are below Nyquist. Higher unused rotations have zero taper. For fixed f each complex stage has transfer (1−ρ)/(1−pₖz⁻¹); use the square and conjugate image for the real output. Doubling compensates a real sinusoid's two-sided spectrum. Tails and conjugate images mean the realised harmonic gain is close to, rather than identically, gₖ. The bandwidth parameter .22f separates adjacent orders; a neighboring positive-frequency response has approximately .22²/(1+.22²) magnitude after both poles at low normalized frequency. This is continuous frequency interpolation by overlapping filters. No pitch-locked resynthesis, input phase replacement or delay on unity-gain q. Test measured complex response including overlap and conjugate terms. Centered harmonic phases, pressure variation and transients still come from the guide. For time-varying f the complex rotations remain contractions: |z₁|≤sup|q|, |z₂|≤sup|q|, hence |w|≤G(1+24·.779)sup|q| even under switching. This conservative bound proves boundedness, not acceptable headroom. The fixed output weights bound the individual correction. Measure the actual sum including limiter demand.

Use one sin/cos pair at 2πf/fs, obtain 12 unit rotations by successive complex multiplication restarting at order 1 each sample. One real exponential gives ρ per sample during slew; cache all coefficients while f is unchanged. Double-precision roundoff in twelve rotations is negligible relative to 1−ρ at 65 Hz/192 kHz. Keep f64 state, bound every command and reject nonfinite wet results before caching. At 65 Hz the center baseband time constant per stage is about 11.1 ms; the two-stage correction can lag the direct pickup transients by ~22 ms. At unity weights its contribution is zero. Changes in partial gains introduce no phase reset.

## A3 gesture amendment

Use [A3's onset, polarity, confirmation, thresholds, U, U_live, B_live, G and transition table](ELECTROBOW_TARGET_ARCHITECTURE.md#a3-articulation-and-hold-policy). They define all non-gesture timers and mixed-state precedence. Capture smoothed Attack in ms, U, B₀ and sign on launch. Let T=max(.008,.001 Attack, .020+.060(1−U)) seconds, and L=round_half_up(fs T), at least one sample. More energetic, short-Attack plucks reach sustained bow sooner. On tick i starting at i=0, u=min(i/L,1), S=u²(3−2u). Compute A3 endpoint targets V*=.06+.10U_live m/s and R*=1.2+1.2 clip(.50 Saturation+.25U_live+.25B_live,0,1). Initialize endpoint smoothers to the captured targets, follow subsequent targets with the A3 15 ms one-pole. Command V=.06+(V*smoothed−.06)S, R=1.2+(R*smoothed−1.2)S, k=RV/1.65. Launch M4 at **.06 m/s** under E=0, using the existing launch algorithm. Pressure corresponds to F_N=2Zk. The seed is an established orbit at a valid lower speed. This trajectory supplies actual changing contact forces during the entry. It cannot recover microscopic pre-Helmholtz formation from a pick.

E begins at zero and follows the signed-off linear A3 rise over max(8 ms, current Attack), including automation; the independently captured T describes harmonic evolution. Hold freezes the endpoints, completes existing ramps, retains geometry, polarity, timbre and last Hz. Ordinary reacquisition stays in the same epoch. Same-pitch repick/new confirmed legato target goes through 5 ms fade with pending acquisition; mute 35 ms, unsupported expiry 100 ms. Lift k=0 on the very first Release sample. E fades from its current value to zero. A3's 750 ms, 800 ms hard silence and 60 ms loud-unvoiced limits still apply. At zero dispose of wave and all lane-local wet states. A soft legato start with no pluck uses its confirmed-capture timestamp as the identity event anchor. New starts cannot acquire color from the previous note. True force, speed, rosin noise and player bow direction are unobservable here. No random bow gesture.

## Execution, storage and budget

```text
prepare outside callback: validate dimensions; reserve rings, per-lane features and scratch
for each host frame in order:
    sanitize input before overlapping output can be written; preserve existing pre-Attack bands
    update linked direct Attack; advance A1/A2 and identity decimators on raw channels
    apply A3 priority and confirmed events; invalidate pending jobs with obsolete gesture ids
    finish any due feature job; publish only if its lane/gesture is still supported
    snapshot next due identity frame; schedule its bounded budget across next hop
    advance voice E, pitch, V,R; run launch or lift if commanded
    call Guide::tick_observed once, receiving M5 and O1 on live success
    on live success: use returned O1 velocity for proposed DC, load, detail, then held I1 and σG
    mix (1−Character·E)d + Character·(Ew); cache last finite E and Ew per lane
    on physical error: guide already invalidated before commit, begin M6 cached fade
    on post-commit output fault: invalidate lane, reset output state, begin same detection-tick fade
    finish both lanes, existing linked Out, host bypass; zero frames advance nothing
```

Publish a completed analysis job on the **next identity hop** before taking the new snapshot. Job time stamps retain the original window end. On Hold, Release or epoch mismatch discard publication. On each host tick process at most ceil(Mmax/H_host_min) window indices per job, where H_host_min=H·Dᵢ. During capture, read immutable indices from the oversized circular history; the H+2 reserve prevents overwrite. Accumulate all K terms per index, bounded K≤12. Start the first work on snapshot tick. Finalize on the next hop (no block boundary dependency). Store enough support events for ceil((4/65+.010)/.004)+2 frames, prune by timestamp, reset invalid timestamps. For pending acquisitions that overlap a release, retain a single new job indexed by the pending gesture; the old color is already frozen.

| Rust location | Contract after independent approval |
|---|---|
| Existing `friction.rs`, `ring.rs`, `waveguide.rs` | Qualified `Guide::tick_observed` delivers owned M5/O1 samples from the same successful tick. O1 uses the shared ring interpolator. A valid cleared tick has no observation. |
| Proposed `instrument/o2.rs` | Stage 06 specifies four per-lane f64 scalars and the q output in FS. I1 and its complex states require a separate design decision. |
| New `instrument/identity.rs` | `Identity::prepare(fs), push(hp), observe(frame,gesture), advance()->Option<GainFrame>, reset()`; preallocated FIR/histories, twelve dB states, support stamps, one projection job. `GainFrame{gesture,end,g:[f64;12]}`. No heaps in launch, hop or reset. |
| Future `analysis.rs`, `pitch.rs`, `voice.rs` | A1/A2 frame contains end, Hz/Q, fast/slow/brightness. Inject it for S4a. Voice owns ramps, E, timers, support/epoch, pending and current color, polarity and cached finite product. |
| `instrument/mod.rs`, `filter_bank/mod.rs`, `attack/mod.rs`, `output/mod.rs` | Stream owns scratch and both lanes, existing input splitter, linked direct swell and protector. Keep `body.rs` on the existing unsupported-rate fallback. Existing Out 0…2, 10 ms full-range trim slew, .98 FS ceiling and 50 ms linked recovery. No ABI field, runtime library, C++ processing or host state additions. |

Numbers below are projected incremental work over the existing qualified guide and planned A1/A2. Six fractional reads with seed fallback, weighted sum, DC, load and detail use order 30 scalar operations. Each of twelve orders has two complex multiply/add operations and an output gain (about 16–22 scalar operations/order/sample). On pitch ramps add twelve rotation products, one sin/cos pair and one exp per lane/sample. Neutral identity can use the same path. FIR costs (32Dᵢ+1)/Dᵢ ≈32 MACs/host sample. At 48 kHz F=16 kHz, M≤985, K≤12: ≤11,820 complex projections terms/hop/lane, ≤2.37 million terms/s. At F<24 kHz, M≤1477, ≤17,724 terms/hop, <3.56 million/s/lane. Each term uses two complex moment accumulations and one rotation (order 12–16 scalar arithmetic ops). Over a host sample ≤ceil(Mmax/(H Dᵢ))·12 terms, ≤84 terms conservatively at the minimum Dᵢ, plus a fixed K-sized hop finalization. Bound FIR and A2 coincidence explicitly for B=1. Stereo doubles lane arithmetic. This budget deserves a deadline measurement before implementing live tracking.

At max F, f64 identity history ≤(1478+120+2)·8=12.8 kB, decimator coefficients/history ≤2·385·8=6.2 kB at max Dᵢ=12, projections, rotations, event history and control states budget <4 kB more/lane. No working array proportional to host block beyond existing declared scratch. Two guide rings per lane are additional (≈47.4 kB at 192 kHz); sum actual allocated capacities, reconcile with V18/V21. All new nonfinite parameters use existing FFI rejection. On reset clear live flags, validity counts, clocks, FIR/HP/filter state and jobs, zero inactive states below 1e−30. No callback allocation, I/O, blocking, locks, worker dependency or unbounded search.

Direct transport latency = 0. Identity feature center age ≈16/F+(M−1)/(2F)+H/F, plus up to a hop of event alignment and smoothing. At 65 Hz approximately 37 ms center-to-publication, earliest post-event acquisition ~25+62+1+5…10 ms. Wet can begin at neutral color earlier. A1/A2 wet onset still has its own ~40–55 ms clean-input acquisition and attack ramp. Electrical group delay and ray travel act only on wet. Measure direct onset, wet 10/90%, tracking and group delay separately. Full-graph and host timing remain open. [Validation budgets](../implementation/DSP_VALIDATION_PLAN.md#budget-and-latency-accounting)
