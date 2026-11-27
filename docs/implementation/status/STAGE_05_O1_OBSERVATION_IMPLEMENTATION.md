# Stage 05 · Isolated Rust O1 spatial velocity

## Contract recorded before implementation

Entry HEAD `0c98f608a0d46e29693320f36e85f173cd3faefe`, branch `feature/bowed-string-engine`, clean index and working tree. Review 4 §G authorizes a private three-station total-velocity observation and its isolated qualification. Geometry is an effective reference setting. Product calibration and the full acoustic specification remain provisional.

| Guide operation | O1 result and ownership |
|---|---|
| Prepared or cleared, valid idle controls | No sample. A future controller owns silent output. |
| Successful launch | Opens the existing M4 epoch under zero wet gain. No observation until the first successful tick. |
| Active tick | The observed-tick entry point advances once. Both writes and scalar state commit first. Its private transaction supplies the actual current period alongside the M5 sample. O1 reads the committed nut ring inside that same exclusive call. |
| Lift or ordinary release | Each successful live tick, including k=0, produces one observation. The controller retains the epoch during release and invalidates at zero gain. |
| Invalid controls or failed physical guard/stage | Existing M6 fault and invalidation. No sample and no observer call. Future controller owns cached detection-tick recovery. |
| Invalidation or transport reset | Existing invalidate clears both logical histories. No independent observer state or reset clock. |
| Relaunch | Both original seed parameters and valid age belong to the new launch. Old physical slots become accessible only after replacement writes. |

There is no caller-supplied observation pitch, no latest-sample getter and no deferred read. A returned scalar snapshot remains valid as a value for its original tick. Fixed geometry, legal period, finite M4 seed and bounded committed waves suffice for an infallible observation. The guide cannot roll back a committed tick. Later fallible output stages would need their own post-commit error and controller recovery contract.

## Submission and revisions

**Awaiting independent Review 5.** The final DSP, fixture grid, observer replay and compiled negative controls are at `9342622a9144e6316af6a89b2b49250affe2be4b`. The final executable qualification revision is `027a7a6a9b71d9c8bc5e463e419053e02af49f17`, which adds the entry-revision state comparison. The following documentation commit contains the handoff, research corrections, ledger and decision log. Resolve its revision with `git log -1 --format=%H --follow -- docs/implementation/status/STAGE_05_O1_OBSERVATION_IMPLEMENTATION.md` on the submitted checkout.

| Changed files relative to entry | Purpose |
|---|---|
| `rust/dsp/src/instrument/{waveguide,ring}.rs` | Observed tick, two-ray station, aperture, shared interpolation with integer cursor shift |
| `rust/dsp/src/instrument/waveguide/{tests,o1_tests}.rs` | Independent trajectories and observer oracles, endpoints, lifecycle, expired seed, partition, allocation, timing |
| `tools/validation/{guide_render.rs,acoustic_observation.py,o1_qualification.py}` | Extended Rust rows, separately callable Stage 04 physical velocity, observed-mode replay and measurements |
| `tools/validation/{o1_mutations.py,o1_state.rs,o1_state_parity.py,fixtures/o1_sha256.tsv}` | Compiled defective observers, direct private-state comparison to entry, retained physical trace manifest |
| `docs/architecture/ELECTROBOW_ACOUSTIC_OUTPUT_SPECIFICATION.md` | Exact integer shift and isolated successful/empty/fault contract |
| `docs/research/{BOWED_TIMBRE_ACOUSTIC_RESEARCH,GUITAR_TIMBRE_PRESERVATION}.md`, `docs/implementation/ACOUSTIC_VALIDATION_PLAN.md`, `status/STAGE_04_ACOUSTIC_RESEARCH.md` | Scatter uses the original lower-band sum to drive the upper excitations. The replay measured 1.3070025 dB for the same-pitch-string broad ablation. The attack comparison shares an E schedule. The acoustic tracking helper is auxiliary. |
| This file, `status/IMPLEMENTATION_LEDGER.md`, `status/DECISION_LOG.md` under `docs/implementation` | Results, dependency status and review handoff |

## Geometry, time and units

Distance increases from the bridge toward the nut. Emission from the bow to station s covers s−β. The returning wave covers (1−β)+(1−s), with one velocity inversion at the rigid nut. A unit string length takes P/2 samples. The guide recurrence gives a=−I(N,n−(1−β)P). Consequently the station reads d₁=(s−β)P/2 and d₂=(2−s−β)P/2 and returns I₁−I₂. At s=β this gives N[n]+a. At s=1 both arguments are identical and subtraction gives exact zero.

`Guide::tick_observed(hz,speed,k)` returns `Result<Option<(Sample, Observation)>, Fault>`. Use this entry point once per desired tick. The existing `tick` advances one tick independently. Both use `advance` and the same guarded transaction. The period carried out of the transaction is the local binary64 quotient used for both propagation arms. Only after both staged writes, loss, mode and M5 DC state have committed can the success closure borrow the nut ring. `Observation` owns three pairs of values, so evaluation of its scalar velocity cannot access a later epoch. Station fields retain the sign of the stored outgoing N for each ray. `Station::velocity` applies the reflection exactly once.

For physical time n−d, m=floor(d), u=d−m. The post-write cursor represents n+1. The shared interpolator computes m and u from **d**, applies integer offset 1, takes N[n−m] followed by N[n−m−1] for a fractional tap, and evaluates (1−u)·newer + u·older. An integral delay reads one value. At d<1, newer is the emission just committed. The nearest representable value below .5 is a bit-sensitive test of this decomposition. The aperture evaluates (left+4·center+right)/6 in that order using f64. Geometry s₀=.174, w=.024 gives .162, .174 and .186. All ray, station and aperture outputs are in m/s. M5 retains its separate audio conversion. Geometry denotes a normalized reference during retune. There is no fitted fret, absolute pickup position or electrical sensitivity.

## Ring capacity and bounds

P ranges from 24.24242424 to 2953.84615385 samples. Each coefficient below is positive, hence its extrema occur at 32 kHz/1320 Hz and 192 kHz/65 Hz respectively.

| s | d₁/P | d₁ range, samples | d₂/P | d₂ range, samples |
|---:|---:|---:|---:|---:|
| .162 | .0175 | .424242–51.692308 | .8555 | 20.739394–2527.015385 |
| .174 | .0235 | .569697–69.415385 | .8495 | 20.593939–2509.292308 |
| .186 | .0295 | .715152–87.138462 | .8435 | 20.448485–2491.569231 |

At every legal real rate let Q=fs/65. A contributing lag is at most ceil(.8555Q)+1 ≤ ceil(Q)+1 < C=ceil(Q)+4. The looser bound floor(d)+2 also fits. The slack exceeds rounding error throughout the domain. At 192 kHz the oldest required lag is 2529 against 2958 slots. A lag in [1,count] maps to a written physical slot. For larger lags, count<C and count−lag is a negative time belonging to this launch. M4 evaluates that time with the retained launch period and speed. Once count=C every legal lag addresses committed storage. This also explains why retune can change which samples are read without regenerating history. The executable edge test checked the predecessor, exact value and successor of all 2461 interior multiples of 65 Hz.

The finite M4 ramp has magnitude ≤.16/(2·.127)<.630 m/s. Both committed waves enter storage through the existing ±4 m/s stage guard. Linear interpolation has nonnegative weights summing to one. The difference of two such reads lies in [−8,8] m/s and the normalized positive Simpson weights preserve the bound. Powers of two in the bound and finite f64 operations give ample overflow margin. No clipping or normalization participates. No post-commit error path is required for legal O1 inputs.

## Numerical qualification

The Rust oracle materializes both M4 histories at absolute times −C through −1. It advances two independent chronological vectors and two FIR delays, enumerates the contact graph in ascending slip order, and appends only its own B and N. Its interpolator locates floor/ceil neighbors on that timeline and walks forward from the older value. A separate chronological vector appends the **actual Rust** N solely for the observer-indexing differential. Both comparisons grade all six rays, all three station differences and the weighted output on every tick at 1e−9 m/s absolute. Failures identify rate, frame, Hz, station, both delays and physical indices. The tests also check branch, retained seed, armed epoch, both cursors and valid counts. [Fixture and predicates](../../../rust/dsp/src/instrument/waveguide/o1_tests.rs)

| Measurement | Result |
|---|---:|
| 13 rates × (28 stationary corner combinations + 3 moving trajectories) | 403 fixtures |
| Committed ticks graded by both Rust oracles | 2,066,150 |
| Worst independent full-guide difference | 2.5590640717609858e−14 m/s |
| Worst observer-only chronological difference | 2.5646151868841116e−14 m/s |
| Largest aperture magnitude on this grid | 1.229885105484438 m/s |
| Missing observations / unexpected guide faults | 0 / 0 |
| Stage 03 control replay through actual observed tick | 152 fixtures, 46,242,000 ticks |
| Worst Python chronological/research difference over replay | 4.440892098500626e−16 m/s |
| Largest aperture magnitude over replay | 1.2068117288964555 m/s |
| Actual launches / faults / invalidations in replay | 152 / 0 / 152 (launch clears included) |

Rates include the seven standard rates, the inward binary64 neighbors of both rate limits, 44150.5 Hz and three adjacent values around 48100 Hz. Every stationary rate includes 65, 65.41, 82.41, 196, 440, 1318.51 and 1320 Hz at both .06/.16 m/s and R=1.2/2.4, for more than three full ring capacities per case. Each starts over saturated stale storage. Each rate also ran the complete 100 ms 440→65 Hz tick-100 retune, a continuous 65→1320→65 Hz bend with lift, and two cycles of 5 Hz, ±35-cent vibrato. The first sample, seed-only, blended seed/current storage, all-storage, both delay directions, sub-sample crossings and repeated physical wraps enter the same predicates. A cut-and-flush test additionally checks that an expired seed cannot re-excite the aperture when delay alternates between both limits.

For a sinusoidal N at order k the direct-minus-return phasor is 2i·exp(−iω(1−β)P/2)·sin(kπ(1−s)), ω=2πk/P. At integer modal orders the position envelope has magnitude |sin(kπs)|. The first positional cancellation in that envelope is near 1/.174=5.747. Integrating the spatial sine across the rectangle gives the center response times sin(z)/z, z=kπw/2. Symmetric Simpson sampling gives the center times (2+cos z)/3. The continuous aperture's first zero is 2/w=83.333, where the three-point multiplier is 1/3. A discrete fractional tap multiplies exp(−iωm) by (1−u)+u·exp(−iω). The analytic tests compare the individual ray amplitudes and phases with this complex response, a closed-form impulse triangle, constant histories, and the Simpson modal response at half-integer delays. Orders 1, 5, 6, 12, 37 and 83 are exercised below Nyquist. Bow and nut endpoint checks run on every grid frame. Actual nonlinear spectra below come from measured projections.

## Negative controls

Each disposable Rust source compiled in release mode. Qualification required the named test to execute and fail its nominated runtime predicate. Except where shown, the first failure occurred at 32 kHz, 65 Hz, frame 0. Values are actual versus reference, in m/s. [Replacements and failure predicates](../../../tools/validation/o1_mutations.py)

| Defect | First failing predicate |
|---|---|
| Single ray | Left station −.167953 vs −.395906 |
| Positive reflected-ray sign | Left station +.060000 vs −.395906 |
| Reversed total velocity | Left station +.395906 vs −.395906 |
| Read before commit | At 1320 Hz, frame 0, left direct −.395905512 vs −.395514982, d=.424242, indices 0/−1 |
| Extra integer lag | Left direct −.166993 vs −.167953 |
| Reversed fractional weights | Left direct −.168174 vs −.167953 |
| Additional bridge reflection | Left reflected history +.333746 vs +.227953 |
| Center moved to .170 | Center direct −.166063 vs −.165118 |
| Uniform aperture weights | Frame 8 aperture −.386586 vs −.388411 |
| Use current retuned P to regenerate seed during observation | Frame 100, left long ray +.219980 vs +.033919, indices −322/−321 |
| Return last observed epoch on cleared tick | Frame 1250 produced Some after explicit invalidation, expected None |
| Add 1.0 in floating point before decomposition | Impulse weight at predecessor of .5 failed original-fraction bit equality |

## Physical and M5 parity

All 12 original columns, including signed-zero bits, matched the qualified trace on every observed render frame. All **152 raw and 12 WAV hashes** matched the Review 3 corpus. SHA-256 of the sorted name/raw/WAV digest list is `a8bd257a945a47aecb31c81cf4569613b87b296db52fd5b93a568ce39eb124b3`. The maintained probes still inspected actual Guide and ring lifecycle after launch and every tick, including right context. The 2 s extra-launch defect failed at tick 96000 and the 440-as-196 fixture failed at 1399.91 cents. Both optimized Python invocation paths were rejected. Existing 11 compiled guide mutations and six ring mutations failed their nominated tests. Guide stress ran 28,800,000 ticks, 600 launches, zero faults and 1200 invalidations.

The separate entry-state differential compiled the entry friction, ring and guide from Git and the observed implementation with the same private-state probe. At 32/192 kHz it matched **13,820 rows** byte-for-byte across clears, active and lifted propagation, wraps, retunes, faults and replacement launches. It captured incidents, both outgoing waves, contact mode/probes, slip, increment, force, string velocity, x, both FIR memories, pickup, M5 audio and DC memory, both seeds, cursors, counts, newest slots, live flag and actual launch/fault counts. The row-stream SHA-256 was `3565e329514d00d05e28272120b4ca0cae3a3d000ec5da959f6496439451a51f`. The in-crate 4000-frame twin-guide test additionally compared **every physical slot** after each step and observed four launches and two induced faults. [Entry build and comparison](../../../tools/validation/o1_state_parity.py) · [State probe](../../../tools/validation/o1_state.rs)

## Local V18 and V19

The O1 call graph beyond the qualified tick is Observation::read → three Station::read → six read_committed → interpolate → at most twelve history lookups. One integer wrap branch selects a slot or the finite seed expression, which uses division and floor. Station::velocity and Observation::velocity use fixed scalar operations. These functions borrow the existing prepared storage and own only fixed-size values. Source inspection found no heap ownership or growth, deallocation, locks, blocking, I/O, unbounded iteration or transcendental functions on the observation path. The existing allocator counter measured **0** operations at 32 and 192 kHz across idle, launch, moving delays, lift, failed tick, invalidation and reset.

On an Apple M2 Pro, arm64 macOS 26.5.1, rustc 1.100.0, the standalone optimized guide and observed guide each warmed for one second and timed five ten-second runs. Each switched 196/440 Hz every 2048 samples and relaunched once per second. A separate optimized read-only microbenchmark warmed a saturated nut history and timed five batches of six reads plus aperture evaluation at the same pair of periods, with inputs and output consumed via black_box. Wall-clock batch means are in ns/sample, median [min, max].

| Rate | Guide alone | Guide with O1 | O1 alone, saturated history |
|---:|---:|---:|---:|
| 32000 | 48.306 [44.799, 50.537] | 54.659 [54.573, 55.221] | 13.659 [12.800, 16.980] |
| 48000 | 44.549 [43.739, 44.657] | 54.960 [54.855, 55.794] | 15.681 [12.690, 27.196] |
| 192000 | 44.787 [43.349, 46.386] | 55.576 [55.169, 57.472] | 12.034 [11.704, 31.301] |

The timing distributions describe isolated workloads. Callback deadlines and full V18/V21 remain downstream. Local V19 compared all six ray bits, station and aperture bits, epoch metadata, failure/empty events and absolute timestamps over 6016 frames in one-sample blocks, fixed 64-frame blocks and 114 deterministic variable blocks including empty calls. Boundaries coincided with launch, retune, lift, fault and invalidation. All rows matched exactly. ABI/host partition qualification remains downstream.

## Physical traces and measurements

The hashed [15-file trace manifest](../../../tools/validation/fixtures/o1_sha256.tsv) lists the retained 22-column traces under ignored `build/stage-05/`. The first 12 columns follow Stage 03; columns 12–20 contain direct N, reflected-history N and total velocity for left, center and right, and column 21 contains the aperture velocity. Format is row-major little-endian f64. Each file retains the corresponding raw M5 diagnostics. The [generated measurement manifest](../../../build/stage-05/manifest.json) carries all field names, source and trace hashes, lifecycle counts, samplewise maxima and twelve-order harmonic amplitudes, phases and relative envelopes where below Nyquist. Phases reference the start of the last delivered second. The retained vibratos include 250 ms right context for the auxiliary phase scorer. Spectral analysis follows sample-level comparison against the extracted Stage 04 pre-loading spatial observation. Stage 05 produced physical traces only. No listening gain was applied.

| 48 kHz steady case | Final-second RMS (m/s) | Whole-render peak (m/s) |
|---|---:|---:|
| 82.41 Hz nominal | .293011 | .733152 |
| 196 Hz nominal | .291341 | .728816 |
| 440 Hz nominal | .284173 | .743363 |
| 440 Hz R=1.2 / 2.4 | .279780 / .287389 | .721619 / .767341 |
| 440 Hz V=.06 / .16 | .171379 / .453288 | .448548 / 1.206812 |

At 440 Hz nominal the H1–H7 peak amplitudes were .2601, .2212, .1645, .1001, .0386, .0113 and .0465 m/s. Phases were 144.81°, 109.63°, 74.47°, 39.35°, 4.32°, 149.09° and 114.89°. H6 was −27.28 dBr. The center/N transfer at H6 was −11.29 dB and the aperture/center transfer was −.041 dB. The measured dip includes source spectrum, fractional delays and guide evolution. The strongest spatial cancellation near that order comes from position.

At 1318.51 Hz the 32/48/96 kHz renders measured last-second RMS .234590/.253276/.277924 m/s. After matching analysis bandwidth to 16 kHz the centroids were 2135/2349/2633 Hz and the fractions above 6 kHz were .00775/.01402/.02950. At 48 kHz the auxiliary bend/vibrato aligned RMS errors were 2.246 cents (196 Hz bend), 2.497 (196 Hz vibrato) and 1.312 (440 Hz vibrato), all at zero integer-ms lag. The original guide scorer owns V10/V11. The 440 Hz lifted trace measured .284250, .233330, .081702 and .00001450 m/s RMS in 50 ms windows beginning at .95, 1, 1.10 and 2 s. O1 perceptual acceptance and the reported buzz remain open.

## Regression and replay

Cargo format and Clippy with warnings denied passed. Workspace debug, native release and translated x86_64 release each passed 84 unit tests and 3 integration tests. The one manual timing test was run separately in native release. Contact reference and differential passed, with 84,886 finite comparisons on each target, maximum one ULP slip difference and at most nine probes. The ring oracle measured .001771 sample maximum absolute circuit arrival error. The eager string suite and embedded validation assertions passed. Acoustic research replay regenerated 15 fixtures and the identity study reproduced the 1.307 dB broad ablation. All 24 existing acoustic-input hashes verified. The hash manifest contains repository-relative paths and must be checked from the root.

Native Release built. CTest passed 4/4, including the processor and editor. The 13-rate packaged baseline measured 2.98023224e−8 FS worst neutral error, 1.1712931e−38 FS non-neutral ABI/VST3 difference and zero tail peaks. Deep strict codesign verification passed. GetPluginFactory and existing eb_* symbols were present. The rebuilt executable's SHA-256 matched the qualified package, `28dbe9a2c3209da8be9746d905577ff0ea7fd101147f8c051bf467a8ed45db78`. Input/ABI/VST3 output hashes respectively matched `5206d315d476ce363fb99c0e90884b68f599a3a3e4626e33bfad5c7322588709`, `c4f241531583ac391ec9419a8c21fc7b5f27465e78d93dfcb0aaa64e12fae6de` and `c7251cd11c37048e8cbd2a645897bcd14e0b87dfec1f650c2797bd5cb5658a94`. The entry diff excludes the shipping Rust render, public ABI, native sources, parameters and IDs, UI, presets and legacy fallback. Native Intel hardware, Linux and Windows execution were unavailable. No release or host callback gate is claimed.

Run from the repository root, with Python 3.9.6, NumPy 2.0.2, SciPy 1.13.1 and the ledger environment's packaged-plugin dependencies. Existing tools use their Stage 03/04 and baseline output directories. New measurements, binaries and disposable mutation trees use the Git-ignored Stage 05 directory. The 15-file TSV digest is `85ecdac330745653572bd43dd1f7eeebddf80f75511e10daa9a5dc14d0084464`. Replays at other revisions can change the detailed JSON manifest's provenance.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1
w=build/stage-05
mkdir -p "$w/tmp"
export TMPDIR="$PWD/$w/tmp"
py=build/spec-reconciliation/py/bin/python

cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test --workspace --release --locked --offline
cargo test --workspace --release --locked --offline --target x86_64-apple-darwin
cargo test --release --lib waveguide::tests::o1 --locked --offline -- --nocapture

rustc --edition 2024 -O tools/validation/contact_replay.rs -o "$w/contact"
rustc --edition 2024 --target x86_64-apple-darwin -O tools/validation/contact_replay.rs -o "$w/contact-x86"
"$py" tools/validation/contact_contract.py
"$py" tools/validation/contact_contract.py --rust "$w/contact"
"$py" tools/validation/contact_contract.py --rust "$w/contact-x86"
"$py" tools/validation/ring_oracle.py
"$py" tools/validation/ring_mutations.py
"$py" tools/validation/string_oracle.py
sed -n '/^```python$/,/^```$/p' docs/implementation/DSP_VALIDATION_PLAN.md | sed '1d;$d' | "$py"
"$py" tools/validation/guide_mutations.py
"$py" tools/validation/guide_qualification.py
build/stage-03/guide-render --stress
"$py" tools/validation/guide_negative_controls.py
"$py" tools/validation/acoustic_observation.py
"$py" tools/validation/acoustic_identity.py
shasum -a 256 -c tools/validation/fixtures/acoustic/SHA256SUMS
"$py" tools/validation/o1_qualification.py
"$py" tools/validation/o1_mutations.py
"$py" tools/validation/o1_state_parity.py

rustc --edition 2024 -O tools/validation/guide_render.rs -o "$w/bench-render"
"$w/bench-render" --benchmark
"$w/bench-render" --benchmark --o1
cargo test --release --lib isolated_read_cost --locked --offline -- --ignored --nocapture

cmake -S . -B build -DJUCE_PATH="$PWD/JUCE" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
bundle=build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
codesign --verify --deep --strict "$bundle"
nm -gU "$bundle/Contents/MacOS/ElectroBow" | rg '_GetPluginFactory|_eb_'
shasum -a 256 "$bundle/Contents/MacOS/ElectroBow"
"$py" tools/validation/plugin_baseline.py
git diff --check 0c98f608
```

## Future integration boundary

A future approved isolated caller can consume `observation.velocity()` from the same `Ok(Some((sample, observation)))` as its M5 telemetry. Attribute both to that control frame. A valid idle tick supplies `None`; a failed tick supplies `Fault` and requires existing M6 recovery ownership. Keep the same transaction when a later stage adds output processing. Leave any finite-product cache, envelope, mute/fade and replacement policy with the controller. A later fallible output stage would act after commit and would need to define its detection-tick result independently of a pre-commit guide fault.

O2 awaits independent O1 review, a revised gain and high-band calibration criterion and evidence for its detail branch. I1 awaits a reviewed remedy for the V17 strongest-fundamental inversion, bounded scheduling/ownership, and controlled held-out matched repeats. The proposed A3 amendment awaits gesture measurements, matched early-envelope and physical-reference evidence and an owner verdict. A2 live selection, supported S5 routing, ABI/host integration, full V18–V21 and human V22 remain downstream. The existing V17–V22 thresholds stand.
