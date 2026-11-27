# S2a nonlinear bow contact · 26 November 2026

**Ready for Review 1. Independent approval is pending.** Starting revision `0110e1fe513bc603491edccef68865d899d42d33`, branch `feature/bowed-string-engine`, clean entry worktree. This record covers the working-tree increment on that revision. The review counterexample now returns Slide(+), j=.09026513359496 m/s and r=.030000277554192455 m/s. It took six segment probes. [Review finding](REVIEW_00_1_SPECIFICATION.md) · [M3 contract](../../architecture/BOWED_STRING_MATHEMATICS.md#m3-force-and-scattering-at-a-massless-point)

## Changed files

| Files relative to repository root | Change |
|---|---|
| `rust/dsp/src/instrument/friction.rs`, `rust/dsp/src/instrument/friction/tests.rs` | Private Rust junction, wave scattering, V02 and allocation tests |
| `rust/dsp/src/instrument/mod.rs` | Declare the isolated module, with a local dead-code allowance until S2b consumes it |
| `rust/dsp/src/ffi/mod.rs`, `rust/dsp/src/ffi/tests.rs` | Test-only access to the existing thread-local allocator counter |
| `tools/validation/contact_contract.py`, `tools/validation/contact_replay.rs` | Independent rational intersection comparison, fold neighborhoods, negative controls and offline adapter that compiles the actual instrument source |
| `tools/validation/string_oracle.py` | Corrected eager scalar reference |
| `docs/architecture/BOWED_STRING_MATHEMATICS.md`, `docs/implementation/DSP_VALIDATION_PLAN.md` | Corrected M3b, V02 and embedded executable low-E reference |
| `docs/implementation/ELECTROBOW_MIGRATION_PLAN.md` | V10/V11 delivered-output and V19 legacy-handle observation points |
| `docs/implementation/status/{SPECIFICATION_SIGNOFF,DECISION_LOG,IMPLEMENTATION_LEDGER,STAGE_01_CONTACT_IMPLEMENTATION}.md` | Current disposition, decision, measurements and replay |

## Interface and units

`FrictionCurve::prepare()` stores ten dimensionless f64 coefficients for the fixed M3a slip knots. The table is read-only and can be shared. Call `curve.solve(ContactInput { bow_speed: V, from_nut: a, from_bridge: b, k }, previous)` for one tick. Here d=(V−a)−b and k=F_N/(2Z), both in m/s. The method takes the previous `Mode` by value. `Direction` restricts the sign of `Mode::Slide` to Positive or Negative.

`Result<ContactResult, ContactFault>` carries either the complete finite sample or a fault. Success returns the next mode, signed slip r, velocity increment j, F=2Zj in N at Z=1 N·s/m, string velocity v=a+b+j, outgoing waves B=a+j and N=b+j, and the segment-probe count. The increment j has velocity units. The dissipated power is Fr and net outward wave power is Fv. On lift, j=0, r=d and the returned Stick clears branch memory.

```text
Reject nonfinite d or k, negative k or nonfinite static limit k*mu[0].
On lift return free scattering. On zero mismatch choose Stick.
If previous is not Slide(sign(d)) and |d| is within the static limit, choose Stick.
At the last loaded knot use q[9]. Above it use |d| - k*mu[9].
Otherwise visit the nine affine segments from highest slip to lowest.
    On the first ordered enclosure left <= |d| <= right:
        At the right endpoint (also a coincident pair), use the upper knot.
        Otherwise compute u = (|d| - left)/(right - left), then q = q_i + span*u.
    On exhaustion, use the M3b clamped static fallback within its margin, else Fault.
Apply the M3b q <= 1e-12 slip floor after selecting the root.
On Slide set r = copysign(q,d), j = d-r. On ordinary Stick set r=0 and j=d.
Check F, v, B and N for finiteness before returning the sample.
```

## Floating-point and branch evidence

Loads use a rounded binary64 multiplication followed by a rounded addition. At the reviewed width of 5.000028169277471e−13 m/s, a mismatch change δh moves the interior solution by (.04−.02)δh/width. One ULP of this h moves q by about 5.55e−7 m/s. The explicit f64 operations yield .030000277554192455. Exact rational interpolation **between the stored endpoint loads**, rounded once, gives .030000277554192458, one ULP away. Reconstructing the loads themselves with exact products and sums on the same prepared coefficients gives .030000644659464174. Bit-level reproduction across platforms depends on the prepared exponential values and on providing the same rounded mismatch.

The Python oracle enumerates intersections forward, including decreasing slopes, coincident intervals and the constant tail. It applies static priority and slope admissibility separately and maximizes the eligible slip. Near the fold, nearby load differences and knot spans are exact subtractions. Three rounded operations on the positive interpolation terms and rounding the oracle result allow a four-ULP slip comparison. Reconstruction from d−j has an additional rounding allowance of one ULP each for j and d−j. Maximum observed slip difference was one ULP and the minimum dissipation was zero watts. Every case also passes the original 1e−10 m/s signed and branch-specific residuals, 1e−10 W flux, 2e−10 N force-bound allowance and −1e−12 W dissipation limit. Static uses its force interval. [V02 comparison contract](../DSP_VALIDATION_PLAN.md#v02-stored-graph-comparison)

| Executed observation | Result |
|---|---|
| Rust V02 scattering sweep with V=.10, a=.13 and b=V−a−d | 80,500 cases, both directions with carried history plus independent Stick/Slide± cases. Checked signed mismatch, slip, both outgoing waves, continuity, force, sliding equality, static interval, flux, dissipation, repeatability and ≤9 probes. |
| Exact-mismatch Rust vs Python graph | 80,500 standard + 4,374 fold rows agreed in mode, explicit slip and reconstructed slip. The adapter uses a=b=0, V=d to preserve exact knot/adjacent query bits. The fold rows cover all nine segments, both signs, three previous modes, three target widths, adjacent k and adjacent endpoint/interior queries. Counts include repeated endpoints. |
| Worked d=.14, k=.10 | Previous Stick gives F=.280 N, r=0. Previous Slide(+) gives F=.10895552132981196 N, r=.08552223933509404 m/s. At −.14 a prior positive slide sticks. At −.20 it reverses into Slide(−). Lift clears that history. |
| Degeneracies and faults | Checked signed zero, tiny positive load and slip-floor neighbors, static/slide history, upper endpoint .08 on the flat fold, h₉ and both adjacent queries, both tail signs, NaN, infinities, negative k, and overflow in mismatch, static limit, force, outgoing wave and velocity. Fault returns no result payload. Reusing the preceding mode and prepared table reproduces the good sample. |
| Negative controls | The old cutoff returns Stick on the review input. Always preferring admissible static returns Stick at .14/.10. Selecting the lower intersection at .14/.10 returns Slide(+) with r=.0040796917426262. All three pass the physical predicates on their fixture. Mode/slip comparison rejects all three, including a separate run on temporary mutated Rust copies. |
| Work and memory | Maximum observed 9 probes. Static-priority, lift and tail shortcuts take 0. Only ten f64 coefficients (80 bytes) per prepared curve, shared constant knots and scalar temporaries. The fixed descending loop reuses the preceding endpoint. At most one division for the chosen segment. No locks, allocation, I/O or transcendental calls in solve. The existing test allocator counted zero alloc/realloc/dealloc operations across 16,004 sweep ticks plus good/fault calls. Integrated callback timing belongs to V21. |

The absolute physical error measurements cover the stated fixtures. For arbitrarily large finite mismatches, subtraction in j=d−r can lose a small force increment. M3a supplies the normal control range and the consuming guide owns the M6 wave limits.

**Failed checks and disposition.** The starting oracle reproduced the reported false Stick while its physical checks passed. The corrected root and negative controls cover that failure. An initial fault test used d=+1e308, k=1e308 with previous Slide(+). That input takes the constant tail and its force remains finite. Reversing d selects admissible static and actually overflows 2j. The corrected fixture returns Fault. All final suites below passed.

## Eager reference waveguide regression

Compared the old script at the starting revision and the corrected script on the pinned numerical stack. Both passed the 90-case spectral, harmonic and RMS gates. No published measurement changed at its reported precision. The fraction-first evaluation changes ordinary rounding on some segments. For example, minimum η went from .9902886844312765 to .9902886844312764. No expected value was updated.

| Reference observation | Corrected measurement |
|---|---|
| Nominal 196 Hz, 48 kHz, R=1.8 | 196 slip onsets and duty .13475 in the third second. The sixth second also contains 196 onsets and 6,468 sliding samples. |
| Low E, 82.41 Hz, final second of six | .22813106186949889 m/s AC RMS driven, .00014419916804091405 lifted at 1 s |
| Seven-second twins, 65.41 / 82.41 / 440 Hz | Lifted −68.3827 / −86.3086 / −471.7872 dB, driven +.02321 / −.15515 / −.00187 dB |
| 440 Hz pressure ratios 1.2 / 1.8 / 2.4 | 1.5–6 kHz energy fractions .09993469 / .11370373 / .12217341. AC RMS .21897902 / .22214375 / .22462838 m/s |
| 440 Hz speeds .06 / .16 m/s | AC RMS .13390287 / .35483538 m/s |
| Paired per-sample branch and bridge traces | 108 renders, 27,437,400 samples. Zero changed branch tags and maximum absolute bridge difference 9.04832e−15 m/s. |

## Replay

Run from the repository root. Measured on Apple M2 Pro, macOS 26.5.1 arm64, rustc 1.100.0, CMake 4.4.3, Python 3.9.6, NumPy 2.0.2 and SciPy 1.13.1. The existing baseline Python environment also has Pedalboard and SoundFile. The contact comparison uses Python's standard library. The test driver reads text offline and is separate from the DSP library. [Baseline environment and native setup](IMPLEMENTATION_LEDGER.md#build-and-baseline-commands)

```sh
. "$HOME/.cargo/env"
py=build/spec-reconciliation/py/bin/python
mkdir -p build/contact-stage
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
rustc --edition 2024 -O tools/validation/contact_replay.rs -o build/contact-stage/contact_replay
"$py" tools/validation/contact_contract.py --rust build/contact-stage/contact_replay
git show 0110e1f:tools/validation/string_oracle.py > build/contact-stage/string_before.py
"$py" build/contact-stage/string_before.py
"$py" tools/validation/string_oracle.py
sed -n '/^```python$/,/^```$/p' docs/implementation/DSP_VALIDATION_PLAN.md | sed '1d;$d' | "$py"
cmake -S . -B build -DJUCE_PATH="$PWD/JUCE" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
"$py" tools/validation/plugin_baseline.py
codesign --verify --deep --strict build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
git diff --check
```

Cargo formatting and Clippy exited 0. The locked offline workspace passed 64 unit tests (five new contact tests), three integration tests and zero doctests. The embedded six-second reference passed both assertions. Release configure, build and all four CTests passed. CMake replaced the intermediate bundle signature during relink and signed the completed bundle. Final strict signature verification succeeded.

The JUCE processor still calls `eb_instrument_process`, whose existing ingestion and `Instrument::render` process the six-control path. The only change in the instrument parent is a private module declaration and its allowance. The rebuilt VST3 and ABI fixed-input payload hashes matched their ledger entries. Worst neutral error was 2.9802322387695312e−8 FS across the 13 baseline rates. The non-neutral ABI/VST3 difference was 1.1712931e−38 FS. The current rebuilt Mach-O SHA-256 is `3b7d9413743150fc70622741993e32b18f88c276057d31a7339096c7db54e7e6`. [Active entry point](../../../rust/dsp/src/ffi/instrument.rs) · [Recorded waveform hashes](IMPLEMENTATION_LEDGER.md#observed-results)

To replay the additional paired branch trace, save the block below as `build/contact-stage/compare_traces.py` and run `"$py" build/contact-stage/compare_traces.py` after extracting the original script above. It loads the trace definition from each file ahead of its top-level experiment loop.

```python
from pathlib import Path
from itertools import product
import numpy as np

def load_trace(path):
    ns = {}
    exec(Path(path).read_text().split("min_eta = 1")[0], ns)
    return ns["trace"]

before = load_trace("build/contact-stage/string_before.py")
after = load_trace("tools/validation/string_oracle.py")
grid = product((32000,44100,48000,96000,192000),(65.41,82.41,196,440,880,1318.51),(1.2,1.8,2.4))
fixtures = {(rate, f, r, .1, False, 3) for rate, f, r in grid}
fixtures.update((48000,f,1.8,.1,lift,7) for f,lift in product((65.41,82.41,440),(False,True)))
fixtures.update((48000,f,1.8,.1,lift,6) for f,lift in product((65.41,82.41,196,440,1318.51),(False,True)))
fixtures.update((48000,440,1.8,v,False,3) for v in (.06,.16))
n = 0
peak = 0
for case in sorted(fixtures):
    p0, b0 = before(*case)
    p1, b1 = after(*case)
    assert np.array_equal(b0, b1), case
    n += len(b0)
    peak = max(peak, np.max(np.abs(p0-p1)))
    if case == (48000,196,1.8,.1,False,6):
        slips = b1[-48000:] != 0
        print("sixth second 196 Hz onsets/samples", np.sum(slips & (b1[-48001:-1]==0)), slips.sum())
print("paired histories, samples, max bridge difference",len(fixtures),n,peak)
```

## S2b integration boundary

After Review 1 and a passing S2a fractional ring, prepare one curve outside sample processing. Store one mode per voice. Initialize idle memory to Stick and seed M4 launch at Slide(Positive). On each tick complete the M2 reads, then pass the incident waves, current V, k (zero for lift) and the lane's previous mode to solve. On success apply the M6 4 m/s guards to both outgoing waves, pickup and loss state. Commit both wave writes, FIR history and returned mode together only after all checks pass. On Fault invalidate the affected epoch before writes. S2b owns the guarded guide, absolute circuit timing and eager/lazy trajectory comparison. S4 owns the audible fault fade. S2b can use prescribed Hz/V/k while the separate production pitch-selection blocker remains open.
