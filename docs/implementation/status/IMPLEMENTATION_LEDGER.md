# Implementation ledger

## 2026-11-27 · Review 2 qualifies the isolated S2a ring and Review 1 validator maintenance

**S2a ring complete at `98d5ac14c84a62fbf25341a34798cd36cd5c5412`.** Independent actual-Rust checks passed 6,561,460 taps against an eager, absolute-time history across ten rates, including the full 100 ms downward-retune fixture. Worst tap error was 1.11e−16 m/s. The impulse weights were exact, all six compiled ring controls were rejected, and the ring allocation test measured zero heap operations in debug and release. The review-local passive circuit built with the Rust rings passed all 21 V03 pairs with worst absolute delay error .001770988 samples. The .003-sample gate rejected zero delay, both one-sample errors and reversed weights.

**Review 1 validator maintenance complete.** The signed slip-floor counterexamples and their neighbors passed exact mode grading. Target-prepared coefficient provenance and the bounded exact-tie search passed. Both tested targets passed 84,886 contact differential queries and 43,580 additional Decimal-oracle queries each. All three compiled V02 branch controls were rejected. The qualified contact solve is unchanged. Rust regressions, native Release CTest (4/4), strict VST3 signature and packaged payload parity passed.

**S2b prescribed-frequency waveguide implementation authorized.** S2b owns the actual guide's V03 passive timing and full evolving eager/lazy wave, pickup and state comparison during retune, V04–V07 guide behavior, V08 guarded lane-wide commit and fault invalidation, V09 guide/DC stationarity, V10/V11 prescribed trajectories and V20 guide source behavior across rates. Production routing, live pitch tracking and release are not authorized. [Independent review and remaining observations](REVIEW_02_RING.md)

## 2026-11-27 · S2a ring awaits independent Review 2

Clean entry at `02f9a27`. The isolated Rust ring at `98d5ac1` passed all seven ring tests. Its 100 ms retune fixtures compared 576,688 individual taps against chronological histories across eight rates. The 42 measured arm impulses had maximum phase-delay error .001066610 samples rounded up. The allocator counted zero operations during sample processing and epoch changes. All six ring mutations failed the intended tests.

The offline cut-loop impulse measured worst absolute delay error .001770988 samples across 21 pairs. Zero delay, either one-sample error and reversed bridge weights failed the .003-sample gate. S2b still owns the actual Rust passive circuit, the complete evolving eager/lazy waves and pickup during retune, and lane-wide M6 commits. Review 2 precedes S2b implementation.

The Review 1 floor and tie corrections passed 84,874 original and 12 added Rust/graph comparisons on arm64 macOS and x86-64 macOS under Rosetta. The qualified contact arithmetic is unchanged. Native debug and release passed 71 unit and 3 integration tests each, CTest passed 4/4, and the rebuilt ABI/VST3 fixed-input payloads matched baseline. [Stage evidence, integration contract and replay](STAGE_02_RING_IMPLEMENTATION.md)

## 2026-11-27 · Review 1 qualifies isolated S2a nonlinear contact

**S2a contact complete at `e112274782bbd53148239345b691f7801bdac8aa`.** The independent review reproduced the signed near-fold failure on the parent and verified the corrected upper root on the actual Rust solver. It reran 80,500 nonzero-incident-wave Rust V02 cases, all 84,874 exact-mismatch V02 and fold comparisons, and an additional 43,580 Decimal-enumerated cases. Mode and slip passed on the reviewed stack. The maximum lookup was nine segment probes and the isolated solver allocator recorded zero operations. Three compiled Rust branch mutations failed the differential comparison while passing their physical residuals.

The parent and current eager references each passed 90 spectral/harmonic/RMS cases. The paired trace comparison matched all 27,437,400 branch tags. The embedded low-E RMS matched the standalone reference. Rust format, Clippy, debug and release tests (64 unit and three integration per profile), native Release CTest (4/4), VST3 signature and the packaged ABI/VST3 payload hashes passed. The active audio path, public ABI, JUCE parameters, presets and legacy filter-bank path remain at baseline. The review changed only this ledger and the review document.

Two MINOR validator issues remain for expanded replay. Exact-root rounding can differ from the prescribed binary64 slip-floor decision, and a single-load exact-tie fixture depends on the prepared coefficient bits. Neither requires a solver change for the qualified range. The S2a fractional-delay ring may proceed and still owns its V03 exit. S2b prescribed-frequency guide work is authorized after that ring passes. Guarded Rust guide trajectories and fault commits, integrated recovery, callback budgets, production routing and listening are pending downstream requirements. A2 pitch selection remains open independently. [Review findings, reruns and stage decisions](REVIEW_01_CONTACT.md)

## 2026-11-26 · S2a nonlinear contact implementation handoff

Clean entry at `0110e1fe513bc603491edccef68865d899d42d33`. The private Rust instrument module implements M3a/M3b, scattering and pre-commit Fault. M3b now admits all enclosing, increasing stored f64 load pairs, evaluates the fraction first and resolves coincident loads to the upper endpoint. The review case selects Slide(+) with r=.030000277554192455 m/s. The migration matrix now names the delivered S6R V10/V11 measurements and both active and legacy V19 ABI observations.

V02 passed 84,874 Rust versus independent graph comparisons, including the original 80,500 cases, and rejected the three negative controls. Maximum lookup was nine segments. The prepared allocator probe counted zero heap operations. All 90 reference spectra/RMS cases passed. The paired reference renders retained their branch sequences and published precision. Cargo formatting, Clippy, 64 unit and 3 integration tests, the native Release build and 4/4 CTest passed. The rebuilt packaged and ABI fixed-pluck payloads matched the shipping baseline. The JUCE render still calls the existing instrument graph. Review 1 and S2a fractional-ring qualification precede S2b. [Stage report, files and replay](STAGE_01_CONTACT_IMPLEMENTATION.md) · [Decision](DECISION_LOG.md) · [Migration ownership](../ELECTROBOW_MIGRATION_PLAN.md#validation-ownership)

## 2026-11-26 · Review of Prompt 0.1

**Prompt 1 is authorized to implement the isolated Rust nonlinear bow-contact solver.** Reviewed `91b58c5..bdc8072`. The 80,500 standard V02 cases agreed with a separately written intersection search. A narrow, increasing segment inside the playing range exposes a branch error shared by M3b and its oracle. Resolve that case and record the correction before closing V02. The ring may proceed independently. Prescribed-Hz waveguide work can follow passing contact and ring tests. [Review findings and stage decisions](REVIEW_00_1_SPECIFICATION.md)

The original ledger replay reproduced 399/399 octave errors. Fitted selection recovered all 399 on the exact parent samples. A fresh run of all 6,725 expanded fixtures matched the reference manifest. The 110 Hz dominant-second input still returned 220.055 Hz on all 200 frames at each tested level. At 711.111111 Hz, coverage fell from 8/8 accurate estimates to 0/8. Production selection and its live-input consumers remain on hold. The review also records false passes in the pitch checker and V03 phase probe, and the missing delivered-output and legacy-ABI observations in the staged matrix.

V08's five sample counts and fault-tick endpoints checked out. A native JUCE driver reproduced the tail overflow and verified the proposed H predicate on an isolated test build. The locked Rust suite passed 59 unit and 3 integration tests, Release CTest passed 4/4, and the existing packaged render matched its baseline hashes. This review adds the report and this ledger entry. Specification changes are deferred to the named corrections. [Evidence, replay and authorization by stage](REVIEW_00_1_SPECIFICATION.md#authorization-by-implementation-stage)

## 2026-11-24 · Independent specification sign-off

**Reviewed base:** `91b58c5c1d32ee20406aac3a0ec73d478008a0d6`. Entry worktree clean. `git diff --name-status 20d4ee56..HEAD` showed only the three Prompt 0 status files. Production Rust, C/C++, public header, build files, presets and UI remain at baseline. All current code changes are offline validation tools.

**Changed documents.** Mathematics M3b/M6, target A1–A4, reference research YIN step 5, migration stages/matrix, validation V02/V03/V08/V12/V18–V20 and fixture conventions, guide rule 6, and the three status records. Added [SPECIFICATION_SIGNOFF.md](SPECIFICATION_SIGNOFF.md). The historical source audit remains pinned. [Decisions](DECISION_LOG.md) · [V01–V22 ownership](../ELECTROBOW_MIGRATION_PLAN.md#validation-ownership)

**Equations and files.** No production equations implemented. Offline `tools/validation/pitch_candidates.rs` evaluates integer/fitted troughs and raw-d vertices. `pitch_study.py` constructs waveforms, direct dτ, HP/FIR, RMS and eligibility, and writes paired measurements. `check_pitch_results.py` checks the known numerical successes and failures and hashes the fixture manifest. `contact_contract.py` enumerates admissible friction graph intersections and verifies flux, sample-count arithmetic and the tail boundary. `ledger_pitch.py`, `ring_oracle.py`, `string_oracle.py` and `plugin_baseline.py` make Prompt 0's diagnostic fixtures directly executable. [Replay commands and environment](SPECIFICATION_SIGNOFF.md#replay-and-provenance)

| Command from repository root (py is the interpreter in the replay instructions) | Observed result |
|---|---|
| `build/spec-reconciliation/py/bin/python build/spec-reconciliation/ledger_pitch.py` (extracted from the ledger at the reviewed revision) | 112 clean cases passed; baseline counterexamples 200/200 and 199/199 lower-octave estimates with zero rejects. Standalone equivalent in tools/validation. |
| `rustc --edition 2024 -O tools/validation/pitch_candidates.rs -o build/spec-signoff/pitch_candidates`; `"$py" tools/validation/pitch_study.py`; `"$py" tools/validation/check_pitch_results.py --reference-hashes` | 6,725 fixtures, three selection rules and both start/follow outputs. Fitted-first recovered both reported cases; 66,890/66,890 accepted across the named grid and 39,600/39,600 for the extended 1/k sweep, zero gross errors on both. Dominant-second sweep 3,576 gross/3,760 eligible, 3,728 accepted. The sign-off lists per-cohort, DI and dynamic results. |
| `python3 tools/validation/contact_contract.py` and `python3 tools/validation/ring_oracle.py` | 80,500 finite branch/flux checks, worked and degenerate vectors, fade and extreme-rate arithmetic passed. Three 743-slot eager/lazy wraps agreed exactly. The worst passive one-circuit delay error was .001771 samples across 21 rate/frequency pairs, below .003. |
| `"$py" tools/validation/string_oracle.py` | 90 spectral/RMS cases, minimum η=.990289. Nominal 196 onsets, duty .13475. Low-E final RMS .22813106 m/s continuously driven and .00014420 lifted. |
| `cargo test --workspace --locked --offline` | 59 Rust unit + 3 integration passed, 0 failed, 0 doctests. |
| `ctest --test-dir build -C Release --output-on-failure`; `"$py" tools/validation/plugin_baseline.py` | Existing native Release tree 4/4 passed (31.30 s). Packaged/ABI neutral max error 2.98023e−8 FS, non-neutral difference 1.17129e−38 FS. Reused the existing binary and verified fixed-input/output payload hashes against Prompt 0. |

**Unresolved:** a bounded production selector for the remaining harmonic-mixture and fine-pitch failures, then Rust implementation parity, combined gesture/launch behavior, host guard implementation, performance and listening. **Next safe change:** isolated Rust M3a/M3b and V02. The next increment after that is M2 ring/V03. [Prompt and work permissions](SPECIFICATION_SIGNOFF.md#independent-implementation-boundaries)

## 2026-11-23 · Reconcile the specification with the shipping processor

| Item | Record |
|---|---|
| Base revision | `20d4ee56e8f614e6544bf3456698fa5d3c096887`, branch `feature/bowed-string-engine`. Status documentation is a single working-tree change on that base. |
| Audited revision | `aa1be375020f81b223171cc8feed52b50dccb7d2` is an ancestor. Intervening changes add the seven design documents and one PNG. |
| Files changed | [SPECIFICATION_RECONCILIATION.md](SPECIFICATION_RECONCILIATION.md), [DECISION_LOG.md](DECISION_LOG.md), and this ledger. Captures, the Python environment and temporary offline probes live under ignored `build/spec-reconciliation/`. |
| Equations implemented in production | None. The offline probes evaluate M2–M4 and A1–A2, and measure M6 work/lift consequences. The current product has no Rust bow contact, waveguide, pitch acquisition or gesture controller. |
| Outcome | Migration step 1 has a production baseline for neutral, non-neutral, ABI, preset, bypass, allocation, block, CPU and packaged replay comparisons. A2 has a reproducible octave error between grid notes. V02 and V08 contain inconsistent literal predicates. An extreme positive rate can overflow the current host tail formula. The reconciliation assigns remaining measurements and fixtures to their appropriate test boundaries. |
| Next safe task | Preserve the two counterexamples as proposed V12 fixtures and review A2 candidate selection, V02 static/sliding residuals, V08 sample rounding and the staged gate ownership. Review the extreme-rate host edge separately. After agreement, step 2 can implement the isolated Rust contact and ring against V02/V03. Keep tracker redesign and production routing pending their respective reviews. |
| Unqualified work | Cross-platform binaries, a live DAW deadline run, per-callback percentiles, full target allocation/denormal measurements, live/spatial/paired-pickup recordings and V22 human listening. See individual V01–V22 dispositions. |

## Build and baseline commands

Run from the repository root. This run used macOS 26.5.1 arm64 on an Apple M2 Pro, Apple clang 17.0.0, CMake 4.4.3 Unix Makefiles, local JUCE 9.0.2, rustc 1.100.0 (0ef1bc386). Existing UI dependencies and dist assets satisfied CMake configure; the build regenerated embedded assets. For a fresh tree prepare JUCE and run `(cd UI && npm ci && npm run build)` before configure. Direct Cargo tests used the test profile. CMake used native Release, BUILD_TESTING=ON, the release Rust archive and host-target Cargo tests. [CMake setup](../../../CMakeLists.txt#L14) · [Rust build](../../../cmake/RustDsp.cmake#L60) · [Repository test wrappers and CI commands](../../../README.md#tests)

```sh
. "$HOME/.cargo/env"
git rev-parse HEAD
git merge-base --is-ancestor aa1be375 HEAD
git diff --name-status aa1be375..HEAD
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cmake -S . -B build -DJUCE_PATH="$PWD/JUCE" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
rg 'CPU stereo' build/Testing/Temporary/LastTest.log
bundle=build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
nm -gU "$bundle/Contents/MacOS/ElectroBow" | rg '_GetPluginFactory|_eb_'
shasum -a 256 "$bundle/Contents/MacOS/ElectroBow"
```

The repository also provides `python3 run_tests.py --verbose` for configure/build/CTest and `python3 run_tests.py --no-build --list` for discovery. CI builds with `build.py` and runs CTest with `--no-tests=error`.

For the offline binary tests the system Python 3.9.6 had arm64 wheels available. These are local measurement dependencies. The plugin uses the existing dependency-free Rust crate.

```sh
audit=build/spec-reconciliation
mkdir -p "$audit"
/usr/bin/python3 -m venv "$audit/py"
"$audit/py/bin/python" -m pip install --only-binary=:all: pedalboard==0.9.17 soundfile==0.13.1 numpy==2.0.2 scipy==1.13.1
"$audit/py/bin/python" tools/render_guitar_comparisons.py "$bundle" "$audit/recorded"
shasum -a 256 "$audit"/recorded/*.wav
sed -n '/^```python$/,/^```$/p' docs/implementation/DSP_VALIDATION_PLAN.md | sed '1d;$d' | python3
```

The Python renderer uses the absolute VST3 path, raw DI gain, four explicit preset/input combinations and 127-frame processing, then reloads state and compares 17/1024-frame replays. The current tool adds 1 s silence to its four recordings. The synthetic baseline probe below adds a full 1.3 s after its final impulse. [Renderer, lines 43–72](../../../tools/render_guitar_comparisons.py#L43)

## Observed results

| Command or measurement | Result on this checkout |
|---|---|
| Git ancestry and diff above | Ancestor check exit 0. Eight added documentation/figure files. No difference in source, public header, vendors, tests or build inputs between pin and base. |
| Cargo fmt and Clippy above | Both exited 0. Formatting check passed. Clippy reported no warnings. |
| Cargo test | Exit 0. 59 unit and 3 integration passed, 0 failed, 0 ignored, 0 doctests. Includes zero Rust heap operations across 1000 prepared active-ABI blocks and reset on the existing DC fixture. |
| CMake configure and build | Both exited 0. Built native Release VST3, final ad-hoc signing/verification and test executables. Module manifest identifies ElectroBow 1.6.0 Fx, component CID ABCDEF019182FAEB456C6362456C6277. Mach-O SHA-256 `e2b752927d025a1fe3c90aa093e8bf115685858034e824f027e650d221135fa6`. Export listing contains GetPluginFactory, all five eb_instrument_* functions and all fourteen legacy functions. |
| CTest | Exit 0, 4/4 passed. ElectroBowRustTests 4.99 s, ElectroBowFilterBankTests 1.11 s, ElectroBowPitchDetectorTests .46 s, ElectroBowEditorSmokeTests 2.05 s. Total 8.62 s. |
| Existing Release processor benchmark inside CTest | Stereo 48 kHz, blocks 16/64/256/1024: median .218825/.215758/.213136/.212999% of one core. At 64 frames, 44.95 ns/frame. Seven one-second active-input batches after 100 warmup blocks, including refill and dispatch. Reference V21 deadline percentiles are still outstanding. [Benchmark](../../../tests/FilterBankTests.cpp#L699) |
| Recorded VST3 renderer | Exit 0, four dry/wet pairs. Finite, peak ≤.980001 FS, zero reported latency, reloaded output within 1e−6 FS of the original. Input/output hashes below. |
| Published low-E reference block | Exit 0, both six-second assertions passed for final-second mean-free uncolored velocity. Expected .228131 m/s driven and .000144 m/s lifted, each within .000002. |
| Fixed V01 numeric probe (`baseline.py` below) | 13 rates (1/2/4/8/22.05/32/44.1/48/88.2/96/176.4/192/384 kHz), mono and anti-phase stereo. No random input. C ABI separate and in-place, zero-frame null calls between chunks, and packaged VST3. Worst neutral error 2.98023224e−8 FS, limit 2e−7; reported latency 0. Native CTest checks bypass and v1–v5 separately. |
| Non-neutral cross-check from same probe | Mono 48 kHz/127, controls (50,.5,.5,.5,.5,.5), initial .1 s silent, 2 s 110 Hz pluck (peak .35, decay .28 s), .5 s silent, .25 FS impulse, 1.3 s silent. Max difference from dry .2894026 FS. Packaged VST3 vs its own eb_instrument_process max 1.1712931e−38 FS. Final .30 s peak exactly 0 in both. Tiny differences are at subnormal scale; the JUCE callback installs a denormal guard. |
| Extreme-rate tail calculation | `python3 -c 'print(1200.0/1e-307)'` printed `inf`. The processor rate predicate, host rate setter and wrapper’s infinite-tail branch are identified in the reconciliation. No extreme-rate DAW render was attempted. |
| Independent indexing and contact arithmetic (`math_probe.py`) | 100001 u values at highest normalized frequency: max absolute phase error .00108095188 samples per arm. A 16-slot impulse probe matched lags 1, 3.25 and 14 over four wraps. Independent eager vs lazy reads agree exactly across three 743-slot wraps, nonzero stale storage, launch FIR offsets and a 440→65 Hz retune at sample 100. Upper q=.0855222393351, j=.0544777606649. The static counterexample to a universal sliding residual gives .025 m/s. |
| Isolated spectrum and energy (`reference.py`) | Instrumented the published Python oracle for three-second renders, all 90 V04 spectra/RMS comparisons passed. Minimum η=.990289. Third second at nominal 196 Hz had 196 slip onsets, duty .13475. Seven-second lift at 65.41/82.41/440 Hz changed RMS −68.38/−86.31/−471.79 dB versus early window, continuously driven +.023/−.155/−.002 dB. At 440 Hz pressure-band fractions .09993/.11370/.12217. Speed RMS .13390/.35484 m/s. Research calculations only. |
| Independent stationary tracker probe (`pitch_probe.py`) | Direct f64 differences, HP/FIR and decimation; 112 named rate/note/spectrum combinations at peak .35 FS met clean stationary error bounds with no missing frames. Two additional valid inputs selected the lower octave on 200/200 and 199/199 frames. Their median absolute errors were 1200.03 and 1199.85 cents. This probe grades the A2 estimator by known input frequency; it omits the voice controller, noise and pluck acquisition. |

The digests below identify the exact audio passed to and returned from processing, before PCM24 file quantization. They hash contiguous little-endian f32 sample payloads. The recorded pairs use the tool’s named excerpts, taper, silence padding and controls. To reproduce their payload hashes, run the renderer with its final WAV writer replaced by a checksum reporter for this Python invocation.

```sh
"$audit/py/bin/python" - <<'PYTHON'
import hashlib
import sys
from pathlib import Path
import tools.render_guitar_comparisons as renderer
def hash_audio(path, audio, rate, **kwargs):
    print(Path(path).stem,rate,hashlib.sha256(audio.astype('<f4').tobytes()).hexdigest())
renderer.sf.write = hash_audio
sys.argv = ['render','build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3',
            'build/spec-reconciliation/recorded']
renderer.main()
PYTHON
```

| Fixture | Input SHA-256 | Wet SHA-256 |
|---|---|---|
| Fixed pluck/impulse, 48 kHz | `5206d315d476ce363fb99c0e90884b68f599a3a3e4626e33bfad5c7322588709` | ABI `c4f241531583ac391ec9419a8c21fc7b5f27465e78d93dfcb0aaa64e12fae6de`, VST3 `c7251cd11c37048e8cbd2a645897bcd14e0b87dfec1f650c2797bd5cb5658a94` |
| Low E swell, 44.1 kHz | `f8fe17cdb6a5d3666211b88c0e2736327c0e5d9377723a03879be60c1a2e78f7` | `7451485a58995ce2822486f74ee7044b62b849291c2a3a68dcf417505b867f6e` |
| Staccato, 44.1 kHz | `c930baa6a1a51a55e87f2b6a5a1e5c3cbdecc87fe781635cf2d8c5277a9c814a` | `62ef8979ec0d219b332e1fbaafc3ef649d1af5928614167b85db156940a87b85` |
| Layered D, 44.1 kHz | `04cc6a57a91bb197c60d820c9d95c4e9f0499e42e99fee24470c76241ab50909` | `fc34b0d58f53b35bf916384f5500e42b39a527ab15a9488352c22fdccd7c8ae0` |
| Riff, 48 kHz | `a41b87b4de6e0bbe28bb269683146da9ca0e0deaaf8c2af3a534604abf792898` | `ca30a37f34e6ad71e9e8f0e2993ffde1fbb40a933f46dcbc60ff66773e65a9a0` |

## Replay the audit probes

The pitch probe takes the first decimated output at zero-based host index D−1 and the first hop at analysis index H−1. Its six harmonics use amplitudes 1/k and phases .13k². The target leaves these fixture choices partly open. Both counterexamples have a qualifying trough each hop, so the established-track fallback cannot change the selected octave.

The original four probes are now standalone tools. Use the replay commands in [specification sign-off](SPECIFICATION_SIGNOFF.md#replay-and-provenance). The macOS packaged baseline also requires the Release bundle and optional Pedalboard package.

```sh
py=build/spec-reconciliation/py/bin/python
"$py" tools/validation/plugin_baseline.py
"$py" tools/validation/ring_oracle.py
"$py" tools/validation/string_oracle.py
"$py" tools/validation/ledger_pitch.py
```
