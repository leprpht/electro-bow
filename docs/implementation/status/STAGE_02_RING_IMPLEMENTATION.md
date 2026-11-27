# S2a fractional-delay ring · 27 November 2026

**Awaiting independent Review 2.** The isolated Rust ring passed its sample-level checks. The offline cut-loop circuit passed the corrected V03 measurement. Measuring the actual passive Rust circuit and comparing the complete driven Rust trajectories remain S2b work.

| Git record | Revision |
|---|---|
| Clean entry, `feature/bowed-string-engine` | `02f9a27a7001c81366e955bfd214877ff2f962da` |
| Final implementation, tests, validators and specification corrections | `98d5ac14c84a62fbf25341a34798cd36cd5c5412` |
| Final handoff commit | The direct child adding this report, ledger and decision. Resolve its full SHA with `git log -1 --diff-filter=A --format=%H -- docs/implementation/status/STAGE_02_RING_IMPLEMENTATION.md`. |

## Index and storage contract

At the beginning of logical tick n, w is the next slot to overwrite. Each arm holds outgoing velocity U in m/s. After launch and before the first commit, n=w=count=0. For capacity C, w=n mod C and count=min(n,C). No ever-growing n is required in storage. For delay D=m+u, m=floor(D), 0≤u<1, the output is (1−u)U[n−m]+uU[n−m−1]. For FIR preload at tick −s, shift both integer lags by s after splitting D. This preserves the fraction of D. A lag l≤count maps to (w−l) mod C. For l>count evaluate the launch seed at count−l, which equals n−l wherever that branch is reachable. An invalid epoch returns zero for legal reads. No interpolation uses the current outgoing sample.

The general storage domain is 1≤D≤C. Integer reads use only lag m. At D=C the oldest sample occupies w itself and is still readable before commit. A fractional D just below C needs lags C−1 and C. A shifted read must fit both nonzero-weight neighbors in the same domain. As D approaches an integer k from below its older weight approaches one, and from above its newer weight approaches one. Both limits are U[n−k].

Delaying the two terms gives H(z)=z^(−m)((1−u)+u z^(−1)). Its magnitude is sqrt(1−4u(1−u)sin²(ω/2)) and its phase continuous from DC is −mω−atan2(u sin ω,1−u+u cos ω) below Nyquist. Phase delay is the negative of this phase divided by ω, with DC limit m+u. At nonzero frequency the fractional contribution generally differs from u. Phase at a transfer zero is undefined.

M2 fixes β=.127, P=fs/f, 32000≤fs≤192000 and 65≤f≤1320. The bridge needs βP−1, the nut (1−β)P, pickup βP/2 and the oldest FIR preload βP−1+2. The oldest integer neighbor of any such read is ceil(effective delay). With Pmax=fs/65, the largest required lag is bounded by ceil(Pmax)+1, including preload. M2 explicitly provisions C=ceil(Pmax)+4 per arm, including interpolation and guard space. At fs=32000, f=1320 the minimum bridge and pickup delays are approximately 2.079 and 1.539 samples. Both physical arms and preload fit without rounding capacity to a power of two.

## Interface and S2b integration

1. Prepare one private `DelayRing::prepare(fs, Arm::Bridge)` and one `Arm::Nut` per lane off the audio thread. Unsupported or nonfinite rates and failed reservations return `None`. Each ring starts invalid. `invalidate()` also serves as O(1) transport reset. It resets the cursor, count and seed, immediately hiding all old slots.
2. At a confirmed launch with zero wet gain call `launch(hz,speed)` on both using the same in-range controls. Rejection preserves the existing epoch. A successful launch resets both indices and installs P and A=V/β. Until a lag has been written, evaluate S(t)=t−floor(t)−.5. The bridge returns A·S((count−lag)/P), the nut −A·S((count−lag)/P+1−β). The nut evaluates phase in the eager string reference's order. No buffer fill occurs. Use `bridge.read_at(D_B,1)` and `bridge.read_at(D_B,2)` for initial FIR x₁ and x₂. Set the contact mode to Slide(+).
3. On every live tick, compute the current P. Read x from `bridge.read(βP−1)`, a from `−nut.read((1−β)P)` and p from `bridge.read(βP/2)`. Reads borrow immutably. Apply the negative bridge FIR once and use the previous mode in the qualified contact solve. Keep the original epoch on pitch changes.
4. Apply all M6 finiteness and magnitude guards to the prospective sample, including pickup and loss state. Each `stage(value)` separately checks a live epoch and a finite outgoing value in [−4,4] m/s. Hold both returned `PendingWrite` values. Until `commit(self)` a token is only an exclusive borrow and scalar. Drop both on failure. After every guard passes, commit both tokens, the FIR state and returned contact mode in the same caller-controlled tick. These final stores are infallible under the private cursor invariants. The ring has no lane or host clock.
5. On fault, discard pending writes, invalidate both affected arm histories before rendering and hand the fault to the lane controller. The caller owns lane-wide atomicity, other state, cached wet products, the fade and reacquisition. Released or idle history reads zero and rejects writes. The S2b caller must advance its own clock and FIR/contact state only for a successful wave tick.

## Ring observations and independent reference

The Rust fixture eagerly evaluates a chronological vector for times −C through −1 and appends every outgoing sample. For a fractional tap it starts at vector index len−ceil(D), then interpolates forward from that older value to its successor. The reference uses direct absolute positions throughout. It carries every prescribed write, independently of the ring cursor and seed age. Its launch prefix uses Euclidean remainder and the M4 arm equations. Test values use m/s.

- The analytic impulse at 48 kHz gave the exact binary64 weights .75 at sample 3 and .25 at sample 4, zero elsewhere for D=3.25. D=1, C−2, C−.25 and C also matched their complete analytic impulses through four wraps. No early sample or residual tail appeared. A separately written ramp exercised both capacity neighbors and the oldest slot across five wraps.
- At 32, 44.1, 44.1505, 48, 88.2, 96, 176.4 and 192 kHz, each arm ran for ceil(.100 fs) ticks. At sample 100 the frequency dropped from 440 to 65 Hz while old launch history was still needed. Subsequent frequencies swept up to 1320 Hz and down to 65. Independent extra taps crossed the representable neighbors of an integer and jumped between minimum and near-maximum delay. The constant, ramp, alternating-sign and xorshift32 (13,17,5, seed 1) writes passed 576,688 sample-by-sample comparisons across 144,172 arm ticks, plus FIR preload checks. Test clock granularity was one sample.
- Integer reads and dyadic impulse weights require equality. For the prescribed fractional histories let W=max(|older|,|newer|), ε=2⁻⁵² and unit roundoff e=ε/2. The binary64 delay fractions and their complements here are exactly representable. In production, absolute product errors sum to at most eW and the sum incurs at most e(1+e)W. In the forward reference, the difference and scaled difference have magnitude at most 2W before rounding, and the exact interpolant has magnitude at most W. Their three roundings cost at most (5e+O(e²))W. The suite bounds the difference by 8εW, including higher-order terms for these normal-range fixtures. Even at W=4 this is at most 7.11e−15 m/s. The original V03 tap and trajectory limits remain 1e−10 and 1e−9 m/s.
- Idle, repeated wrap, invalidation and relaunch checks covered stale nonzero storage at every integer lag. Invalid taps, extreme offsets, invalid controls, discarded writes, both guard endpoints and a failing second arm preserved previously committed history. A relaunched nut using a different P,V matched a fresh eager history until it fully covered the buffer.

### V03 absolute phase delay

| Observation boundary | Executed result |
|---|---|
| Actual isolated Rust ring | At the seven V03 rates and three fundamentals, measured the two individual arm impulses with cleared prehistory. The first nonzero sample supplied the absolute integer arrival. Maximum phase-delay error from requested D across the 42 responses was .001066609089 samples, below the M2 .0011 per-arm bound. |
| Offline M2 cut-loop circuit | Convolved the bridge and nut two-tap impulses and the signed three-tap bridge FIR, retaining both reflections. Across 21 pairs maximum absolute error from P was .001770987909 samples at 32 kHz / 1318.51 Hz, within .003. |
| S2b actual Rust guide | Pending. Inject at the cut with contact lifted, capture the first circuit's returning impulse with its time origin intact, and grade its absolute phase delay against P at the V03 frequencies/rates. Compare both complete closed-loop wave histories and pickup per sample against eager initialization throughout the 100 ms downward-retune fixture, including FIR preload and state evolution. |

For the sparse offline impulse, let k be the first nonzero arrival measured from injection. Compute R=Σh[t]exp(−iω(t−k)) over the measured support, then Dabs=k−arg(R)/ω. The residual positive taps span at most four samples and their phase spread is less than π up to the highest test frequency. Their sum therefore follows a continuous phase from DC within that sector. This determines the winding from the impulse's time origin independently of expected P. A centroid would measure DC delay and would miss fractional phase error at the fundamental.

| Offline mutation | Largest absolute error in samples | Gate |
|---|---:|---|
| Constant .98 at sample zero | 2935.330989 | Failed .003 |
| Omit one integer sample | 1.001771 | Failed .003 |
| Add one integer sample | 1.000766 | Failed .003 |
| Reverse the bridge fractional weights | .981963 | Failed .003 |

### Rust mutation controls

Each temporary copy compiled successfully and failed the named assertion in the original numerical test. The ordering control moved the caller's commit ahead of the reads while preserving the expected impulse. The other five changed ring code. The harness requires the selected test to execute and fail at its assertion site.

| Mutation | Rejecting Rust test |
|---|---|
| Write before reading the current tick | `impulse_before_commit` |
| Read the physical slot one sample newer | `impulse_before_commit` |
| Reverse the two interpolation weights | `impulse_before_commit` |
| Wrap the write index one slot early | `full_capacity_neighbor_across_wraps` |
| Replace lag C by C−1 for the fractional neighbor | `full_capacity_neighbor_across_wraps` |
| Retain the old seed on invalidation | `invalidation_and_relaunch_hide_stale_slots` |

## Real-time cost and memory

Preparation reserves and sizes the samples, then stores them in a boxed f64 slice. A read checks the delay before conversion and uses at most two integer-history queries. Each stored lookup uses one bounded conditional wrap. Each negative-history lookup uses bounded arithmetic, one division and floor. Integer reads skip the zero-weight neighbor. Staging checks the scalar and epoch. Commit stores once and advances a bounded cursor and saturated count. Read, stage, commit, launch and invalidate have no loops, locks, I/O, shared mutable globals, resize or transcendental functions. Division and floor during negative-history evaluation are part of M4.

The existing thread-local allocator counted zero alloc/realloc/dealloc operations over three resets and relaunches, 35,496 fractional reads and commits at 192 kHz, plus idle and fault operations, in debug and release. On native arm64 the ring metadata measured 72 bytes. The table includes sample payload and metadata and excludes allocator bookkeeping and downstream FIR/contact state.

| Rate | Slots per arm | Sample bytes per arm | Two arms including metadata | Two stereo lanes including metadata |
|---|---:|---:|---:|---:|
| 32 kHz | 497 | 3,976 | 8,096 | 16,192 |
| 192 kHz | 2,958 | 23,664 | 47,472 | 94,944 |

V18 integrated reset/allocation, denormals and host capacity, and V21 packaged deadline/stress measurements remain pending.

## Review 1 validator maintenance

The replay adapter exports the actual ten binary64 preparation words through `--coefficients`. The only addition to the production friction source is a read-only accessor. The oracle checks μ₀ exactly, independently evaluates each exponential at 80 decimal digits on the binary64 argument, brackets its nearest binary64 value by one neighbor each way, and propagates those bounds through the M3a positive multiplications and sums. This bounds a one-ULP exp variation in the preparation check. The solve comparison then uses the measured words. Both tested Rust targets produced identical words.

The oracle still enumerates ascending, descending and coincident segment intersections in forward order, includes the tail, and maximizes admissible slip. Only within four ULPs of the floor does it consult the selected segment's fraction-first operation order for the discrete floor decision. All modes are graded exactly. Sliding slip retains its independent root reference, four-ULP comparison and original reconstructed-slip allowance.

At |d|=1.7431847738615466e−12, k=4.5041501448100105e−13, the rounded independent root is 1.0000000000000002e−12. The normative segment result is exactly 1e−12. Rust and the corrected oracle both stick for either same-sign history. The lower adjacent magnitude sticks and the upper adjacent magnitude slides with the incident sign. The separate Rust tiny-load floor regression also passed.

The target tie fixtures search at most 64 representable k values on either side of the algebraic center for coincident stored endpoints. They exercise both signs at and around the exact fold. The named narrow and exact Review 1 values are also replayed when the reference segment coefficients match. On this stack the center k=.3486472674315129, h=.2726217499930773 yields signed slip magnitudes .08. The original 80,500 standard and 4,374 fold differentials plus six floor and six explicit tie queries passed on each tested target. Maximum explicit slip error was one ULP and maximum probe count was nine. All three contact negative controls failed mode/slip comparison while passing the physical predicates. The existing Review 1 temporary compiled mutation runner reproduced those rejections on native arm64.

## Regression and replay

Measured on Apple M2 Pro, macOS 26.5.1, rustc 1.100.0, CMake 4.4.3, Python 3.9.6, NumPy 2.0.2 and SciPy 1.13.1. Native arm64 ran the complete suite. The x86_64-apple-darwin Rust release suite and contact adapter executed under Rosetta on the same machine, using the native Python oracle. Native Intel hardware, Linux and Windows were untested.

| Check | Observed result |
|---|---|
| Cargo fmt, Clippy with denied warnings, debug and release | Passed. Each native profile ran 71 unit and 3 integration tests, zero failures. Rosetta release also passed 71 + 3. |
| Offline eager string and embedded low E | 90 spectral/RMS cases passed, minimum η=.9902886844312764. At 196 Hz the third second had 196 onsets and duty .13475. Six-second low-E AC RMS was .22813106186949889 m/s driven and .00014419916804091405 lifted. Both embedded assertions passed. |
| Native Release and CTest | Configure and build passed. CTest passed 4/4, including Rust, FilterBank/processor, PitchDetector and EditorSmoke. The build replaced the intermediate signature and the final strict bundle verification passed. |
| Packaged regression | Across 13 rates the worst neutral error was 2.9802322387695312e−8 FS. The non-neutral ABI/VST3 difference was 1.1712931e−38 FS. Fixed input, ABI and VST3 payload SHA-256 values matched the ledger, with zero tail peaks. |
| Shipping surfaces | JUCE still calls `eb_instrument_process` and the existing `Instrument::render`. The instrument change is a private module declaration. The public header, FFI, legacy processing, host parameters, presets, UI and routing have identical sources to entry. The rebuilt binary exports the VST3 factory, five active and fourteen legacy C functions. |

Run from the repository root with the baseline Python environment. The additional compiled contact mutation runner below the block was available in this checkout's ignored review directory.

```sh
. "$HOME/.cargo/env"
py=build/spec-reconciliation/py/bin/python
mkdir -p build/ring-stage
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test --workspace --release --locked --offline
cargo test --lib instrument::ring --locked --offline -- --nocapture
cargo test --lib tiny_positive_load_preserves_the_m3b_slip_floor --locked --offline
"$py" tools/validation/ring_mutations.py
"$py" tools/validation/ring_oracle.py
rustc --edition 2024 -O tools/validation/contact_replay.rs -o build/ring-stage/contact_replay
"$py" tools/validation/contact_contract.py
"$py" tools/validation/contact_contract.py --rust build/ring-stage/contact_replay
rustc --edition 2024 --target x86_64-apple-darwin -O tools/validation/contact_replay.rs -o build/ring-stage/contact_replay_x86
"$py" tools/validation/contact_contract.py --rust build/ring-stage/contact_replay_x86
cargo test --workspace --release --locked --offline --target x86_64-apple-darwin
"$py" tools/validation/string_oracle.py
sed -n '/^```python$/,/^```$/p' docs/implementation/DSP_VALIDATION_PLAN.md | sed '1d;$d' | "$py"
cmake -S . -B build -DJUCE_PATH="$PWD/JUCE" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
bundle=build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
codesign --verify --deep --strict "$bundle"
"$py" tools/validation/plugin_baseline.py
nm -gU "$bundle/Contents/MacOS/ElectroBow" | rg '_GetPluginFactory|_eb_'
git diff --check
```

Also executed `"$py" build/review-01/mutants.py`. The tracked contact replay includes the three independent branch negative controls. In developing the ring mutation runner, the missing-neighbor test failed with the numerical mismatch D=496.25, .015625 versus .01171875. The first runner incorrectly expected the word “assertion” in every Rust diagnostic. The corrected runner checked the named failed test and its source location and accepted all six expected failures.

## Changed files

| Paths relative to the root | Purpose |
|---|---|
| `rust/dsp/src/instrument/{mod.rs,ring.rs,ring/tests.rs}` | Private ring declaration, implementation and seven tests |
| `rust/dsp/src/instrument/{friction.rs,friction/tests.rs}` | Read-only preparation access and portable signed fold fixtures |
| `tools/validation/{contact_replay.rs,contact_contract.py}` | Coefficient transport, independent preparation check and floor/tie grading |
| `tools/validation/{ring_oracle.py,ring_mutations.py}` | Absolute cut-loop measurement, 100 ms chronological reference and compiled ring mutations |
| `docs/architecture/BOWED_STRING_MATHEMATICS.md`, `docs/implementation/{DSP_VALIDATION_PLAN,ELECTROBOW_MIGRATION_PLAN}.md` | Reviewed exact-fold fixture and V02/V03 method and ownership |
| `docs/implementation/status/{STAGE_02_RING_IMPLEMENTATION,IMPLEMENTATION_LEDGER,DECISION_LOG}.md` | Evidence, decision and handoff for independent Review 2 |
