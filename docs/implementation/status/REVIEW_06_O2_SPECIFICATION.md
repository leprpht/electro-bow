# Review 6 · O2 electrical pickup specification

**27 November 2026 · Authorize one private, provisional Rust O2 increment.** The equations, units and lane transaction are sufficiently specified. The isolated acceptance contract includes the scoring corrections below. Electrical calibration, public Noise semantics, owner listening and integrated headroom remain open. S2a, S2b and O1 retain qualification. This review changes status documentation only.

## Reviewed revisions and scope

| Revision or state | Review record |
|---|---|
| Review 5 entry authority | `85b0aa10c5ee3b19f09637c0d78d8d5023bdc9d2`, also the local remote-tracking tip |
| Prompt 6 specification and fixture | `36b85e5218a4541506b6d308c9ca495af44ba656` |
| Intermediate executable edits | `6372c1838aca6b5cc4c561d03b05544a83fee94b`, `7414fbc8ac73bc5b21771bbc7e2e9c8e49cdf3d9` |
| Final executable evidence | `a515db76285e3dbc507d0bc483d2ccf9116ea8c1` |
| Reviewed submission | `d53bc11b144f3cd62104b3ed72952708ec26ce83`, branch `feature/bowed-string-engine`, five commits ahead. Index and working tree clean, including untracked inventory. Standard ignored build, dependency and cache paths were present. |

Read the specification, matrix, stage submission, Reviews 4 and 5, parent acoustic specification, M1–M7, both acoustic plans, original DSP validation, ledger and decision log. Inspected the entire 16-file delta, the Rust guide/ring and shipping direct/Output interfaces, both O2 Python tools, O1 reporter changes, fixture manifests, raw and JSON measurements and actual ZIP contents. The tracked delta is documentation, four offline Python files, a trace manifest, attribution/checksum maintenance and one source WAV. The source, test, build and public-surface diffs are empty. The qualified contact, ring, guide, O1 and M5 are identical to Review 5. The shipping graph, legacy/fallback paths, Rust/C ABI, JUCE parameters, presets and UI are identical. No production O2, I1, live tracking or articulation was introduced. [Complete reviewed diff](../../../build/review-06/reviewed.diff) · [Stage 06 submission](STAGE_06_O2_SPECIFICATION.md) · [Review 5](REVIEW_05_O1_OBSERVATION.md) · [Review 4](REVIEW_04_ACOUSTIC_RESEARCH.md)

## Independent mathematical findings

The input is signed transverse velocity in m/s. At each O1 station the direct nut-arm ray minus the reflected history gives total transverse velocity. The weighted aperture is `(left+4*center+right)/6`. Bounded outgoing samples and their convex interpolator give a two-ray bound of ±8 m/s. The historical M5 bridge sample has its own blocker and .85 conversion. It is a separate FS diagnostic.

- **DC blocker.** Place a zero at $z=1$ and a decaying pole $c=\exp(-2\pi20/f_s)$. With $w=z^{-1}$, $B(z)=c(1-w)/(1-cw)$, yielding $h_n=c[(h_{n-1}+v_n)-v_{n-1}]$ in the specified binary64 order. The impulse has $b_B[0]=c$, and $b_B[n]=-(1-c)c^n$ for $n\ge1$. Its exact unit-step response is $c^{n+1}$ and its impulse absolute sum is $2c$. DC cancels and Nyquist gain is $2c/(1+c)$. The exponential pins the decay constant. The exact half-power frequency relative to the Nyquist plateau is $f_s\arctan((1-c)/(1+c))/\pi$, approximately 20 Hz.
- **Load and prewarp.** A unity-DC second-order Butterworth factor is $L_a(s)=1/(1+\sqrt{2}u+u^2)$ with $u=s/\omega_0$. Substitution of $s=2f_s(1-w)/(1+w)$ gives $\Omega_0=2\pi3200/f_s$, $K=\tan(\Omega_0/2)$ and $\omega_0=2f_sK$ rad/s. Expanding numerator $K^2(1+w)^2$ and denominator $K^2(1+w)^2+\sqrt{2}K(1-w^2)+(1-w)^2$ gives $D=1+\sqrt{2}K+K^2$, $b=(K^2,2K^2,K^2)/D$, $a_1=2(K^2-1)/D$, $a_2=(1-\sqrt{2}K+K^2)/D$.
- **Poles and update.** The normalized analog poles are $p=-1/\sqrt{2}\pm j/\sqrt{2}$. The digital poles $(1+Kp)/(1-Kp)$ have radius $\sqrt{(1-\sqrt{2}K+K^2)/(1+\sqrt{2}K+K^2)}<1$. The blocker pole also stays in $(0,1)$ over the full rate interval. For denominator $1+a_1w+a_2w^2$, use $l_n=b_0a_n+z_1$, $z'_1=b_1a_n-a_1l_n+z_2$, $z'_2=b_2a_n-a_2l_n$. Substituting the previous states recovers the difference equation with negative feedback terms. Compute candidates from old states and commit together. Zero all four histories at prepare/reset and confirmed launch. The first seeded sample has $h_0=cv_0$ and $l_0=b_0(.85h_0/\sqrt{2})$.
- **Response, phase and delay.** Let $\Omega=2\pi f/f_s$ and $r=\tan(\Omega/2)/K$. Then $L=1/(1-r^2+j\sqrt{2}r)$, $|L|=(1+r^4)^{-1/2}$ and $\arg L=-\operatorname{atan2}(\sqrt{2}r,1-r^2)$. The load has unity DC, −3.0103 dB and −90° at 3.2 kHz and a double zero at $z=-1$. The normalized cascade is $H=B[(1-m)L+m]$ with $m=.025\,\mathrm{depth}$. Its phase is $\arg B+\arg[(1-m)L+m]$ modulo $2\pi$. For $0<\Omega<\pi$, $\arg B=\pi/2-\Omega/2-\operatorname{atan2}(c\sin\Omega,1-c\cos\Omega)$. Delay in seconds is $-\operatorname{Im}(H'_\Omega/H)/f_s$ where eligible. At depth zero, add the blocker term $(1-c^2)/(2(1-2c\cos\Omega+c^2))$ to the load term $\sqrt{2}(1+r^2)(dr/d\Omega)/(1+r^4)$ in samples, with $dr/d\Omega=\sec^2(\Omega/2)/(2K)$. At positive fixed depth use the derivative of the complex blend. DC and depth-zero Nyquist have undefined phase and group delay. At positive depth Nyquist gain is $m\,2c/(1+c)$. A changing depth is a bounded time-varying blend.
- **Detail, conversion and gain.** Multiply $h_n$ by .85 FS/(m/s) and divide by $\sqrt{2}$ to obtain $a_n$. The load states $z_1,z_2$, load output $l$ and delivered $q=l+m(a-l)$ are FS. $U=\sqrt{2}q$ is the common-gain probe. The two terms in the convex blend come from the same signal and retain their phase relationship. Depth doubling doubles the residual at unchanged history. Active tails continue to propagate and a cleared silent lane stays zero. No fixed delay is inserted on the dry path. Stability plus the blocker and load impulse absolute sums give the stated conservative BIBO bounds. [Proposed equations and tap ownership](../../architecture/ELECTROBOW_O2_PICKUP_RESPONSE_SPECIFICATION.md)

**INFORMATIONAL · Filter math and numeric representation.** Prepare six finite binary64 coefficients once off the render path for each finite $f_s$ in [32000,192000] Hz. Keep them immutable and scoped to that stream's state. The sample path uses ordinary ordered add/subtract, multiply and one constant division as specified. The signs, state unit, initialization and guard are implementable directly. At most nine multiplies, eight adds/subtracts and one constant division per live lane, plus finite/range checks and four state stores. Stereo O2 payload is 112 bytes excluding layout, shared smoothing and controller caches. These are static estimates. Actual allocator, denormal and local runtime measurements belong to the Rust increment. [Preparation and feasibility](../../architecture/ELECTROBOW_O2_PICKUP_RESPONSE_SPECIFICATION.md#preparation-and-derivation)

## Electrical reference and calibration

**MAJOR · Instrument-specific calibration remains open.** I inspected Helmuth Lemme's original Fig. 2 and p. 31. The 1972 Fender Stratocaster bridge pickup is given as 2.2 H, loaded by 470 pF and 200 kΩ, the parallel combination of 250 kΩ volume and 1 MΩ input with the tone pot absent. The analyzer excites the pickup with a transmitting coil and plots output in dBFS over about 100 Hz–10 kHz. The reference shows a rise and approximately 4.5 kHz resonance. The 48 kHz candidate load is −3.0103 dB at 3.2 kHz, −7.12 dB at 4.5 kHz and monotonic with depth zero. The reference supplies evidence for load-dependent voicing and gives a different shape from this candidate around the proposed corner. String motion, pickup height, switch/load identity of the target DI, absolute flux sensitivity, interface gain and a measured phase trace are unavailable. The old 2014 PDF URL returned 404 via direct fetch. The author's article is accessible from the current Planet Z copy. [Lemme, Fig. 2 and p. 31](https://www.planetz.com/wp-content/uploads/2026/01/Pickup_Measuring_Techniques.pdf) · [Paiva, Pakarinen and Välimäki](https://aes.org/publications/elibrary/elibrary-page/?id=16551) · [Horton and Moore](https://guitarelectroscience.com/wp-content/uploads/2020/07/Horton.pdf)

The earlier H2–H6 fit of one low-E performance constrains an effective aperture in those windows, up to about 494 Hz. No electrical cutoff fitting or held-out electrical validation occurred. Approve 20 Hz, 3200 Hz, $Q=1/\sqrt{2}$, .85 FS/(m/s) and $1/\sqrt{2}$ as fixed provisional engineering values for the isolated comparison. Pickup-specific claims require a controlled target transfer and, for absolute sensitivity, a calibrated velocity-to-output measurement with the declared recording gain. Product voicing and trim require an owner decision. The isolated increment can generate comparable audio while these decisions remain open. [Stage 06 reference and limits](STAGE_06_O2_SPECIFICATION.md#electrical-and-physical-references)

## Gain and headroom

The O2 return contains conversion and fixed trim once, before sign and capture magnitude. For the eventual neutral identity path use $w=\sigma Gq$, with $G=\operatorname{clip}(A_{\mathrm{pk}}/.14,.08,1.4)$ and one captured sign. On the supported future path form $P=Ew$ once and mix $y=(1-CE)d+CP$. Character sets the mix weight. Out acts on the finite f32 lane mix through the internal settled target $t=2\,\operatorname{clamp}(\mathrm{public\ Out},0,1)$, with existing 10 ms full-range slew. The linked protector sees $t\,\mathrm{f64}(\mathrm{f32}(y))$ per lane. Inspect the scalar stages at their named taps. The engineering inverse trim is telemetry. Listening-file multipliers stay out of engineering and wet gates. [Tap table](../../architecture/ELECTROBOW_O2_PICKUP_RESPONSE_SPECIFICATION.md#measurement-taps-and-gain-ownership) · [Shipping Output](../../../rust/dsp/src/output/mod.rs)

From a fresh Rust O1 render at 48 kHz, 440 Hz, $V=.16$, $R=1.8$, the independent filter gives **1.005418823 FS common** and **.710938468 FS delivered**, over all three seconds including seed. Unity E/Character, G=1.4 and internal Out=2 imply **1.990627709 FS** and **6.155279 dB** instantaneous demand relative to .98. The first two are measured isolated waveform peaks. The latter two are calculated scalar implications.

I reproduced all 384 corner rows with separately formed poles and quadratic identity stress sections, within $1.59\times10^{-13}$ FS per row and $4.14\times10^{-13}$ dB of the submitted peak/demand. The model used settled binary64 gains, $E=1$ and $C=0/1$. The dry corner is the .35 FS pluck with Attack=0. It peaks at .700 FS at maximum Out. All Out=0 rows give zero.

| Injected fixed vector | Largest implied pre-protection FS | Demand dB |
|---|---:|---:|
| Neutral | 1.996430 | 6.180559 |
| All +5 dB | 3.315087 | 10.585378 |
| All −5 dB | 1.699587 | 4.782345 |
| H1 −5 dB / H2 +5 dB | 2.125602 | 6.725118 |

**MAJOR · Nominal V08 remains an integrated gate.** The proposed fixture gives 211200 padded f32 frames and exactly 144000 eligible frames per mono/stereo layout and depth. The input hash is `cfc6d9342aa505dd7a53bd0ae531f717bec007042918ada9949a2117342a3d09`. Use the exact generated bytes, or closed-form $\theta[n]=2\pi f(n/f_s)$ in the runner. Repeated f64 addition changes 13,584 f32 samples by at most $7.45\times10^{-9}$ FS and changes the hash. State that implementation convention in the matrix. The live gate requires real supported launch, measured E and captured G, and counts one frame wherever actual linked f64 $g<1$, with a separate count of $p>.98$. At most 1439 eligible frames may be active, zero nominal faults are allowed, and the full stream must meet .980001 FS. The deterministic injected V/R/G/E path remains a routing control. Stress rows test safety at extreme G/Out separately.

At 48 kHz, forcing one hypothetical $p=1.001$ FS request yields $g_0=.98/1.001$. With $p<.98$ thereafter, the exact shipping recurrence reaches a binary64 fixed point of .9999999999998668 after 61,599 updates and continues counting active. An early nominal request can therefore exhaust the gate through recovery alone. Preserve exact counting and the shipping recurrence. At S6R record the first request, recovery and both counts and fail the run if it exceeds the budget. Any change to activity semantics or protection requires explicit approval. No estimated corner, neutral observed peak or shipping regression can qualify candidate V08. [Nominal test and recovery](../O2_ACCEPTANCE_MATRIX.md#nominal-limiter-use-and-integrated-recovery) · [Independent results](../../../build/review-06/independent.json)

## V17 and acceptance findings

The authoritative DSP plan is unchanged. Use the ordinary M5 converted bridge $u$ on the same raw guide tick stream as denominator of final wet over original bridge. Grade $w/u$ with fixed $G=1,\ \sigma=+1$, same bow, stationary final second, including delivery trim in $w$, with direct absent and no Out or protection. Grade the absolute level difference against 3 dB using stationary uncentered RMS. Separately log $v/N$ in m/s, $l/a=L/h$ in equal units, detail $q/l$, common gain $U/u$ and identity-only $i(q)/q$. No denominator receives per-note normalization or a compensating trim. The ratio floor follows division with a valid denominator. With silence or sub-floor denominators return null and mark a required gate failed. Harmonic energy uses mean-removed one-second Hann windows with 8× padding, with a once-counted union of positive M5-fundamental harmonic masks strictly below Nyquist and all non-DC power of wet in the denominator. Keep factory η≥.85 and grade the strongest component independently. The factory residual is time-aligned subtraction of final wet before RMS divided by off-depth final wet, with a limit of −28 dB. The isolated sweep normalizes out only the known conversion. The downstream sweep takes the original bridge drive as the reference. Keep actual Movement/Saturation endpoints, 32 kHz/1318.51 Hz Noise=0 with 96→32 diagnosis, measurable effects on supported notes and factory η and residual obligations. [Signed-off V17](../DSP_VALIDATION_PLAN.md#automated-cases-and-acceptance-criteria) · [O2 tap matrix](../O2_ACCEPTANCE_MATRIX.md#levels-windows-and-eligibility)

All 42 submitted neutral rows satisfy the applicable measured predicates, including the high-speed witness rows. Delivery over M5 is −2.7375 to −.8107 dB, common gain +.2728 to +2.1996 dB, minimum factory η .996679 and maximum residual −36.48 dB. At 32 kHz the specified high-note neutral η is .996620. Independent sweeps give maxima −.02064/−.02330/−.02557 dB, depth 0/.5/1 respectively. The 17 submitted negative controls rejected their named deviations. All nine identity stress failures survive per-row aggregation. The inversion at 196 Hz moves the strongest component to 391.837 Hz while wet/M5 is +.433 dB. Across low/middle/high stationary notes the all-up wet/M5 levels are +3.63/+3.59/+3.53 dB. All-down gives −4.82/−5.06/−5.34 dB. A scalar shift that brings either level vector into bounds pushes the other farther outside on the same fixture. A scalar also preserves spectral order and cannot cure fundamental inversion. These failures retain the I1/S5 hold. [Measured rows](../../../build/review-06/independent.json) · [Submitted strict JSON](../../../build/review-06/results.json)

| Severity and exact location | Reproducible finding and consequence | Required correction |
|---|---|---|
| **MAJOR · Strongest component**, `guide_qualification.py:66–76`, `o2_qualification.py:85–86,147–177` | At 48 kHz, unit 196 Hz M5 and wet $0.5\sin(2\pi196n/f_s)+0.6(-1)^n$ pass level (−.132 dB) and strongest fundamental (reported 196 Hz). The largest non-DC FFT bin is 24 kHz. The shared `find_peaks` helper omits the endpoint. At target 440 Hz a tone 15.04 cents flat also passes the implementation's symmetric-Hz tolerance. | Use a separate O2/V17 scorer including positive Nyquist when selecting the largest non-DC bin. Interpolate interior bins only and retain exact Nyquist as an endpoint. Pass when either absolute frequency error ≤2 native bins or absolute cents error ≤15. Verify the bridge anchor near the known target before using it for masks. Treat an ineligible reference as a failed required row. Preserve qualified guide scoring. Add both negatives. |
| **MAJOR · Coverage and sweep**, `o2_qualification.py:412–448,625–679`, matrix lines 3 and 22 | Removing the 48 kHz/196 Hz/.10/1.8/Noise=.5 row and returning a +8 dB sweep peak at 3 kHz still produces `neutral_candidate_passed=true` and a zero exit. Existing false predicates are retained, but the aggregator grades only returned V17 keys and never consumes sweep flags. | Enumerate the exact expected key and applicable-predicate sets for the isolated bench, stationary/witness, sweep, corner and lifecycle fixtures. Reject missing required measurements, applicable nulls and recorded failures. A sweep peak above +6 dB holds the electrical row for review, including an eligible endpoint maximum. Preserve frequency, height and contiguous −3 dB width or null when an edge truncates it. Add omitted-row and reported-peak negatives. |
| **MAJOR · Final-wet noise residual**, `o2_qualification.py:145–174`, matrix line 21 | With uncolored q unchanged and corrected factory wet $w(.5)=w(0)+.1\sin(2\pi6151n/f_s)$ on the unit 196 Hz fixture, the row reports −240 dB and passes. The actual final-wet residual is −20 dB. The neutral isolated rows use q=w, so their reported results stand. | Before scoring final colored V17, require both aligned final wet waveforms under identical injected/captured settings except Noise. Compute residual using $\operatorname{RMS}(w(.5)-w(0))/\operatorname{RMS}(w(0))$. Keep the local q subtraction as an O2 diagnostic. Add the colored negative. |
| **MINOR · Electrical and transient reporting**, `o2_response.py:98–167`, matrix lines 29–35 | All depths are exercised for complex impulse response. Explicit phase/delay rows are only at depth zero. The long-step continuation uses SciPy for its load, and its settling check scores that load output. The release played in the bench is physical lift at $E=1$. | At isolated Rust exit publish actual per-depth phase/delay including nulls at zeros, and exercise actual Rust for all 60 s, checking every depth after one second, retained input and terminal memories. Give physical-lift and harness-only release/disposal observations separate labels. Bind gain/mutation checks to actual Rust taps, including $a$ and $q$. |

The independent scorer retains the same Hann, interpolation, time window, ratios and masks on the actual stationary traces. Including Nyquist in the peak search leaves the submitted waveform conclusions intact. The review's controls exercise the submitted row and aggregate functions directly. The corrected semantics above apply to the next increment's qualification. The signed-off V17 level, harmonic, noise and sweep requirements retain their original values. [Control probe](../../../build/review-06/controls.py) · [Control results](../../../build/review-06/controls.log)

## Rates, high band and lifecycle measurements

The reproduced filter uses independently generated analog poles, bilinear pole mapping and a separate SciPy polynomial realization. The long step computes the specified scalar blocker order for every sample. The load step and raw observation processing use the independent realization. The tables cover all seven V20 rates. The submitted runner additionally passed the two nearest interior boundary values, 44150.5 and 48100 Hz. O1 input came from fresh audited Rust renders with one launch, no fault and one epoch on every observation run. Registered traces matched their frozen digests. The independent FFT error below is absolute complex gain after undoing the fixed conversion, taking the worst of depths 0/.5/1.

| Hz | Blocker pole | Load radius | Cutoff Hz | Max complex error | Depth-zero delay at 440 Hz, ms | Load at 12 kHz, dB |
|---:|---:|---:|---:|---:|---:|---:|
| 32000 | .996081 | .642496 | 3200 | 1.54e−14 | .08574 | −34.84 |
| 44100 | .997155 | .724683 | 3200 | 1.67e−14 | .08684 | −27.81 |
| 48000 | .997385 | .743821 | 3200 | 2.51e−14 | .08703 | −26.91 |
| 88200 | .998576 | .851137 | 3200 | 4.49e−14 | .08775 | −24.01 |
| 96000 | .998692 | .862354 | 3200 | 6.27e−14 | .08779 | −23.84 |
| 176400 | .999288 | .922566 | 3200 | 6.54e−14 | .08797 | −23.23 |
| 192000 | .999346 | .928627 | 3200 | 1.53e−13 | .08799 | −23.19 |

Maximum independent cutoff error was $3.2\times10^{-12}$ Hz and terminal impulse output/blocked residual $4.50\times10^{-31}$ or smaller. Cascade DC error was below $8.7\times10^{-14}$ on the finite impulse sum. At 48 kHz the unit step peaks at .617155 FS loaded. All rates and depths return exact zero from fresh zero history and zero input. All long-step outputs at depths 0/.5/1 after one second are below $10^{-10}$ FS. The largest 60 s blocker residual is $1.70\times10^{-13}$ m/s with input still 1 m/s. At 48 kHz neutral normalized Nyquist gain is within $10^{-16}$ of zero. At depths .5 and 1 it is .012484 and .024967. Neutral cascade delay at 20/82.41/196/3200/12000 Hz is 4.048/.512/.152/.0728/.00656 ms. At 12 kHz depth 1 gives −.0171 ms in the very attenuated interference band. This is a local phase derivative of a causal blend. The per-depth phase/delay record uses null at DC and at neutral Nyquist. The impulse-moment and closed-form derivative agree within $3.59\times10^{-13}$ s at eligible frequencies. [Independent bench](../../../build/review-06/independent.json) · [Submitted 11-rate bench](../../../build/stage-06/results.json)

At 1318.51 Hz I used the same bow speed and load as H, polyphase anti-alias conversion of each higher-rate **render** to 32 kHz, Kaiser β=10 and the interior [1.5,2.5) s. The analysis band is (0,14 kHz], with upper power [6,14] kHz. The 96 and 192 kHz native guide renders are separate realizations at their own rates. The other three qualification rates extend Stage 06's four high-note diagnostics.

| Source Hz | Raw upper power % | Neutral loaded upper power % | Loaded nonharmonic % | Loaded centroid Hz |
|---:|---:|---:|---:|---:|
| 32000 | .7622 | .0176 | .3380 | 1783.5 |
| 44100 | .9256 | .0196 | .0876 | 1854.4 |
| 48000 | 1.3376 | .0259 | .0826 | 1860.5 |
| 88200 | 3.2444 | .0941 | .0351 | 1956.8 |
| 96000 | 2.9274 | .0895 | .0581 | 1955.3 |
| 176400 | 4.5354 | .1262 | .0481 | 1978.4 |
| 192000 | 4.1741 | .1202 | .0364 | 1976.2 |

Native 32 kHz loaded H1–H4 relative to the independently rendered 96→32 comparison are −.39/−1.45/−3.67/−7.42 dB. Relative to 192→32 they are −.43/−1.58/−3.99/−8.03 dB. Fractional interpolation, effective damping, nonlinear regime and the warped discrete electrical response all vary with rate. An off-order residual can contain leakage or sidebands, and foldover can land on an order admitted by the union. The 1% upper-power flag is diagnostic. These comparisons cannot assign the differences to aliasing or pass integrated V20. At 440 Hz the ≥6 kHz power fractions at depth 0/.5/1 are .0327/.0292/.0260%. A larger carrier return can reduce this band by interference even with increasing Nyquist transmission. This supports a bounded carrier-detail description. Audibility and preference are owner decisions.

For both 48 kHz seeded pitches the first delivered sample is −.01308935 FS at depth zero. The 10 ms windows already reach nearly steady RMS. A physical lift at 1 s cuts work and leaves the guide and O2 running.

| Pitch Hz | 0–10 ms RMS FS | .5–1 s RMS FS | First 25 ms after lift, RMS FS | 6–7 s RMS FS |
|---:|---:|---:|---:|---:|
| 82.41 | .16633 | .17194 | .16808 | 2.071e−5 |
| 440 | .17070 | .16707 | .15557 | 1.732e−22 |

The 440 Hz late absolute magnitude survives in the results. Its phase and normalized partial comparisons are ineligible. O1's joint per-partial checks require both windows voiced and both amplitudes greater than $10^{-12}$ in their own units, then mask before dividing. The 0 and $10^{-22}$ synthetic tails produce strict JSON nulls and a voiced numerator with zero outgoing denominator also yields null. Forging a relative phase fails the submitted negative control. The first-sample O2 measurement has time-domain statistics only. Retain that distinction in Rust replay, and keep thresholds in reporting rather than signal state. The Review 5 near-silence issue is closed. Review 5's migration issue is also closed. Rows 2a, 2b and 4a now identify completed O1, O2 held on submission, and separately authorized baseline S4a. This decision supersedes the submission status only for the private provisional increment. [O1 reporter](../../../tools/validation/o1_qualification.py#L42) · [Acoustic migration](../ACOUSTIC_MIGRATION_PLAN.md)

## Rust interface, lifecycle and fault boundary

**INFORMATIONAL · Implementable lane transaction.** The actual private method is `Guide::tick_observed(&mut self, hz: f64, speed: f64, k: f64) -> Result<Option<(Sample, Observation)>, Fault>`. It calls the existing advancing transaction once, which commits both rings, loss, contact branch and M5 state before constructing the owned `Observation` from the returned period. `Observation::velocity(self) -> f64` supplies the signed O1 operand. Capture this value from the same successful tick. A cleared valid idle returns `Ok(None)`. An `Err(Fault)` invalidates the guide itself before any new wave state commits and supplies no observation. The returned `Sample::audio` retains its M5 diagnostic meaning. [Actual interface](../../../rust/dsp/src/instrument/waveguide.rs#L160) · [Guide commit](../../../rust/dsp/src/instrument/waveguide.rs#L245)

Use private `O2Coefficients::prepare(fs: f64) -> Result<Self, PrepareError>`, `O2Lane::reset(&mut self)` and `O2Lane::tick(&mut self, coeff: &O2Coefficients, v: f64, noise: f64) -> Result<f64, OutputFault>`. Preparation and output faults are distinct typed errors. Scope coefficient construction to prepare and share one immutable coefficient set across the stream's lanes. A lane starts with four positive zeros. At entry require finite input in [−8,8] m/s, finite depth in [0,1], and finite old states. Check finite $h,a,l,z'_1,z'_2,q$ before publishing four states and returning q. A failure returns without changing any lane state. The caller owns epoch invalidation and reset. Pass a bounded numeric depth from the isolated harness. Eventual shared smoothing updates once per frame after first-target initialization, with the existing 15 ms coefficient and rejection of nonfinite control snapshots. It lives above the lanes. Launch, release and lane disposal do not restart shared smoothing or Output. Transport reset clears both and zero-frame calls advance no clocks. [O2 lane contract](../../architecture/ELECTROBOW_O2_PICKUP_RESPONSE_SPECIFICATION.md#per-lane-sample-and-lifecycle)

| Event / ownership | Required sample behavior |
|---|---|
| Prepare/transport reset/idle | Zero local histories and caches. Idle skips O2 and emits $E=P=0$. Reset may safely repeat. |
| Confirmed launch | Install M4 and reset O2 under $E=0$ before the first observed tick. A failed launch follows the physical fault path and requires no additional guide tick. |
| Successful observed tick | O1 and unchanged M5 come from that commit. Compute and validate O2 from the owned O1 result. Cache $E,P=Ew$ only after the complete wet proposal including finite f32 mix is known to be valid. |
| Compatible reacquisition and retune | Continue histories. O2 has no pitch-dependent coefficient or independent ring reads. |
| Lift and commanded Release | $k=0$ on the first release tick. The live filter continues during the fade. Completed release disposes state and invalidates that epoch. |
| Pre-commit physical fault | Use the guide's already-invalidated epoch. Reset O2 and emit the first cached-fade decrement immediately. |
| Post-commit O2 or wet-output fault | Retain the fact that the physical tick already committed. Invalidate that epoch once, without replay. Reset O2, even if a later wet-stage check failed after committing the filter. |
| Cached fade and fresh acquisition | Latch the preceding finite $E$ and $P$ before clearing other lane-local state. At the detection tick start $s=(N-1)/N$, then continue $s=(N-1-i)/N$ up to $i=N-1$. Emit $sE,sP$ and use the product directly in A4. Clear caches at zero and require fresh acquisition. Skip physical/wet work while fading so faults cannot restart or extend it. The other lane progresses before the final linked Output. |

Independent rounding gives $N=160,221,221,240,960$ at 32/44.1/44.15/48/192 kHz, zero at detection plus $N-1$. At 44.1 kHz the first multiplier is $220/221$ and the forced zero occurs 220 samples later. Both probes and audible recovery must distinguish pre-commit and post-commit counts. The isolated harness can exercise dirty reset, observed-frame split/whole parity, failed input/state with no partial commit, silent idle, post-commit invalidation and fresh launch from zero. The future controller owns real cache, envelope and stereo recovery. A4 must consume the cached product directly, without reapplying E. The existing host denormal guard and the specified local near-denormal benchmark remain distinct from signal eligibility. Any future integrated failure of the direct path needs its existing sanitization. A wet fade assumes a finite direct sample. [M6](../../architecture/BOWED_STRING_MATHEMATICS.md#m6-energy-release-and-floating-point-limits) · [V08 fault tests](../O2_ACCEPTANCE_MATRIX.md#nominal-limiter-use-and-integrated-recovery)

## Listening and owner status

**MAJOR · Acoustic and public control approval remain pending.** I verified the actual ZIP, WAV samples, key and scaling. The three stationary M5/O2 pairs, 440 Hz detail-off/factory pair and low-E lift have original common-gain engineering copies. Matched copies use a fixed scalar from [.2,.8) s, reach approximately .10 FS RMS with peaks .1913–.2669 FS and match the documented engineering scalars within $2.72\times10^{-8}$ FS after f32 rounding. The reference copy in the ZIP is byte-identical to the licensed fixture. The fresh preview fetch matches the source hash and AudioToolbox decode reproduces all 77,176 samples of the 44.1 kHz mono slice exactly. It uses the left channel and source gain. Measured early/middle/late RMS is .03806/.11734/.02671 FS. The uploader identifies a viola bow, Stratocaster copy and Lace pickups. Interface, pickup position, loading, bow reversal, initial contact and passive lift times are unknown. McCormick's pack documents a separate Stratocaster and three round robins. Raw release samples require an account and were unavailable. [Stage 06 listening ZIP](../../../build/stage-06/listening.zip) · [Credits](../../../tools/validation/fixtures/acoustic/ATTRIBUTION.md) · [Vaisvil recording and license](https://freesound.org/people/vaisvil/sounds/103565/) · [McCormick pack](https://www.pianobook.co.uk/packs/bowed-electric-guitar-with-release/)

This is enough material for a preliminary owner screen of pickup-led direction, buzz, sustain/lift usefulness and the small factory-detail change without waiting for plugin integration. It gives one uncontrolled physical phrase at low E. The compared stages change both geometry and load, and the detail pair tests only one pitch at off versus factory. It cannot establish instrument calibration, generalized attack/release, identity or final Noise behavior. For an anonymous trial use the matched directory and keep the engineering labels and key out of view until ratings are recorded. I inspected the samples and provenance. No human listening was conducted in this review. The recorded owner decisions remain the pickup-led target preserving DI traits and prior feedback of approximate buzzing on raw M5. Owner judgments on O2 clips, trim, voicing and the proposed Noise mapping remain unrecorded. Proposed supported-path Saturation and Movement semantics also retain their existing gates. A dated owner preference and artifact/detail decision on these clips is still required before product approval. A sensed velocity branch with no independent noise source is the exact provisional engineering meaning of numeric O2 depth. Public mapping remains held. [Prior owner evidence](REVIEW_03_WAVEGUIDE.md#b-acoustic-and-perceptual-assessment) · [Owner screen](../ACOUSTIC_VALIDATION_PLAN.md#compact-human-architecture-gate)


## Regression and provenance

Executed from the repository root on arm64 macOS 26.5.1 (25F80), Rust 1.100.0, Python 3.9.6, NumPy 2.0.2 and SciPy 1.13.1. I copied the submitted JSON, ZIP and key to ignored `build/review-06/` before rerunning the maintained tools. The regenerated JSON agrees with the submission aside from the recorded HEAD, and the ZIP matches byte-for-byte. The independent probe uses freshly rendered O1/M5 observations, independently specified Butterworth ZPK mapping, polynomial load, FFT/sweep reduction and expanded quadratic stress filters. The control probe calls the submitted scorer directly. The test commands were as follows. [Independent script](../../../build/review-06/independent.py) · [Independent log](../../../build/review-06/independent.log) · [Counterexample script](../../../build/review-06/controls.py)

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1
py=build/spec-reconciliation/py/bin/python

git status --porcelain=v1 -uall
git diff --check 85b0aa1..d53bc11
git diff 85b0aa1..d53bc11 > build/review-06/reviewed.diff

cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test --workspace --release --locked --offline
cargo test --workspace --release --locked --offline --target x86_64-apple-darwin
cargo test --release --lib isolated_read_cost --locked --offline -- --ignored --nocapture

rustc --edition 2024 -O tools/validation/contact_replay.rs -o build/review-06/contact
"$py" tools/validation/contact_contract.py --rust build/review-06/contact
"$py" tools/validation/ring_oracle.py
"$py" tools/validation/ring_mutations.py
"$py" tools/validation/ledger_pitch.py

"$py" tools/validation/guide_qualification.py
"$py" tools/validation/guide_negative_controls.py
"$py" tools/validation/guide_mutations.py
build/stage-03/guide-render --stress
"$py" tools/validation/acoustic_observation.py
"$py" tools/validation/o1_qualification.py
"$py" tools/validation/o1_mutations.py
"$py" tools/validation/o1_state_parity.py
"$py" tools/validation/acoustic_identity.py
shasum -a 256 -c tools/validation/fixtures/acoustic/SHA256SUMS
"$py" tools/validation/o2_qualification.py
unzip -tq build/stage-06/listening.zip
shasum -a 256 build/{stage-06,review-06}/listening.zip

PYTHONPATH=tools/validation "$py" -c 'from pathlib import Path; from guide_audit_build import build; build(Path("build/review-06/render"))'
"$py" build/review-06/independent.py
"$py" build/review-06/controls.py
curl -fsSL https://cdn.freesound.org/previews/103/103565_1735967-hq.mp3 -o build/review-06/vaisvil-preview.mp3
shasum -a 256 build/review-06/vaisvil-preview.mp3
afconvert -f WAVE -d LEF32 build/review-06/vaisvil-preview.mp3 build/review-06/vaisvil-preview.wav

cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
bundle=build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
codesign --verify --deep --strict "$bundle"
shasum -a 256 "$bundle/Contents/MacOS/ElectroBow"
"$py" tools/validation/plugin_baseline.py
```

| Check | Actual result |
|---|---|
| Git and artifact hygiene | Full diff and whitespace check passed. Existing 24 audio checksums retained and one permanent attributed sample added. All 25 verified, including the 77,176-frame phrase, SHA-256 `645ed8f4298842e01c752a20b7a8e52653c7fa959879fbf3f954f9fe31a757b1`. Generated WAVs, ZIP/key, traces, JSON, scripts and transient evidence stay in ignored build outputs. The listening ZIP hash is `39dd496030d731b3d8bbf39c15dd420de61d116f89fa1bb6b61eb6483c497712`. |
| Cargo format, Clippy and tests | Format and Clippy passed. Debug, native Release and translated x86_64 Release each passed 84 unit and 3 integration tests, with one intentionally ignored O1 timing test. That test passed separately. |
| Contact, ring, guide and O1 | Contact matched 84,886 finite Rust comparisons, max nine probes and three rejected branch controls. Ring oracle and six mutations passed. Guide 152/152, hashes match, replay controls and eleven guide mutations rejected. Ten-minute stress 28,800,000 ticks, 600 launches and zero faults. All 46,242,000 original telemetry rows matched observed vs unobserved exactly. The O1 reconstruction maximum was $4.441\times10^{-16}$ m/s. All 12 O1 mutations were rejected, and 13,820 entry-state rows matched bitwise. The A2 replay passed 112 named clean cases and still reproduced the 200/200 and 199/199 octave errors. |
| Acoustic/offline and review probes | Both Stage 04 acoustic tools completed. Maintained O2 replay passed 11 rate rows, 42 neutral rows and all 17 expected negative controls, retaining nine failed identity cases. Independent witness, corners and bench reproduced. Independent false passes in the submitted scorer are listed above. |
| Native build and packaged baseline | Release build, CTest 4/4 and deep/strict signature passed. Mach-O SHA-256 `28dbe9a2c3209da8be9746d905577ff0ea7fd101147f8c051bf467a8ed45db78`. The 13-rate baseline reproduced the previous input, ABI and VST3 digests. Worst neutral delta $2.98023224\times10^{-8}$ FS, wet ABI-to-package delta $1.1712931\times10^{-38}$ FS, zero fixture tail peaks. |

[Cargo](../../../build/review-06/cargo.log) · [Retention](../../../build/review-06/retention.log) · [Offline](../../../build/review-06/offline.log) · [Native and packaged](../../../build/review-06/native.log)

Native Intel, Windows and Linux execution, physical target calibration, documented owner listening, compiled O2 parity and O2-specific callback/allocator results were unavailable at this boundary. Integrated V08/V17–V21, host candidate routing and formal V22 remain for their authorized stages.

## Authorization by boundary

| Decision | Disposition |
|---|---|
| S2a contact and ring, S2b guide | **Retain qualification.** Actual regressions, qualified trajectories and source parity stand. |
| O1 | **Retain isolated qualification.** Owned same-tick observations and original M5 bits are preserved. |
| O2 mathematical specification | **Approve for a provisional isolated component.** Fixed 20 Hz blocker, prewarped 3.2 kHz Butterworth, current aperture velocity input and bounded carrier return. |
| O2 electrical calibration | **Open.** Reference supports load-dependence and shows a different electrical shape. No target cutoff, Q or physical sensitivity is calibrated. |
| O2 gain staging | **Approve isolated tap order and fixed constants.** Product gain intent and actual nominal/extreme linked behavior remain open. |
| O2 acceptance matrix | **Approve the isolated contract with the required scorer corrections in this review.** The submitted executable alone cannot qualify Rust. The original V17 denominator, thresholds and downstream owners stand. |
| O2 detail semantics | **Approve numeric depth in isolation. Hold public Noise mapping.** Perceptual usefulness, owner acceptance and integrated control behavior remain open. |
| O2 isolated Rust implementation | **Authorize one increment.** Scope and required exit below. Implementation qualification requires its own review. |
| Acoustic perceptual approval | **Pending.** Pickup-led reference direction is already selected. No dated owner listening result for O2. |
| I1 source identity | **BLOCKER.** Realized strongest-fundamental and level failures, clocked capture/publication ownership and controlled held-out source repeats remain unresolved. |
| S3/A2 | **BLOCKER.** Existing bounded analyzer work retains its scope. Final selector and live tracking remain held. |
| Baseline S4a | **Retain its independent authorization.** Inject frames into the original A3, qualified guide/M4 and M6 cached lifecycle. Use M5 or qualified isolated O1 as the test observation. |
| Amended gesture and S4b | **Hold.** Command ordering, controlled gesture evidence, owner screen, A2 clearance and S4a qualification remain required. |
| Supported S5 wet processing | **Hold.** Requires isolated O2 qualification, corrected I1, articulation, owner acceptance, original V17 and applicable V20. |
| Production integration | **Hold S6R/H/P.** Previously authorized legacy/crossover parity and independent H guard retain their separate scope. |
| Release qualification | **Hold S7.** Requires integrated numerical, host, real-time, cross-platform and V22 evidence. |

## Exact next authorized increment

Implement only the private `instrument/o2.rs` component and the isolated replay and test boundary. Prepare the six coefficients, own four f64 states per lane, consume the same successful `Guide::tick_observed` result and return delivered q. Use injected bounded depth. Limit its connection to private test/replay harnesses. Preserve original guide/M5 samples and epochs bitwise. Keep controller, I1, public controls and shipping render outside this increment.

At the isolated exit, require per-case measurements with the corrected coverage, peak and sweep scorer. Require the existing $10^{-9}$ normalized complex-gain tolerance, $10^{-6}$ Hz cutoff and $10^{-8}$ s eligible delay tolerance, and compare actual Rust $h,a,l,q$ to independent time-domain values within $10^{-9}$ absolute in their declared units. Exercise all seven rates and the four interior/uneven rates in the matrix, invalid preparations, depth endpoints and factory, zero/impulse/long step, whole and split sample schedules, dirty reset, pause at idle, lift, disposal and fresh epoch. Exercise nonfinite/range input and dirty-state faults, verify no state publication on failure, and demonstrate one successful guide tick followed by output invalidation with no replay. Record the retained expected I1 failures as downstream evidence. Run zero-allocation and local near-denormal checks and preserve shipping regressions. Publish the Rust differential and submit for isolated qualification. The existing listening package is already usable for the separate owner decision.
