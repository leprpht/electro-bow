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
