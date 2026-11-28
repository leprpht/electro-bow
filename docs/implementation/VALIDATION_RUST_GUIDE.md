# Rust validation guide

Run from the ElectroBow repository root. Install the toolchain in `rust-toolchain.toml`. The non-shipping workspace has its own lockfile. A fresh machine needs one dependency fetch.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
manifest=tools/validation-rs/Cargo.toml
cargo fetch --locked --manifest-path "$manifest"
cargo build --release --locked --offline --manifest-path "$manifest"
v=tools/validation-rs/target/release/electrobow-validation
"$v" list
"$v" fixtures
"$v" contact
"$v" inspect contact
```

On Windows use the corresponding `.exe`. Native packaging needs a configured Release CMake tree and JUCE. Set `JUCE_PATH` when JUCE is outside the default `JUCE` directory. Historical O1 state comparison needs Git history including `0c98f608`. The C++ offline epoch comparison needs a C++17 compiler available as `clang++`. Platform availability is recorded in the [stage handoff](status/STAGE_V1_RUST_VALIDATION_MIGRATION.md).

## Workspace ownership

| Location under tools/validation-rs | Responsibility |
|---|---|
| src/main.rs, publication.rs, evidence.rs | CLI, exclusive invocation, atomic receipts, strict JSON and SHA-256 |
| src/contact/ | Exact rational interpolation, input schedules, physical grading and branch mutants |
| src/ring.rs | Chronological seed, wrap, arrivals and absolute loop-delay oracle |
| src/replay.rs, process.rs | Build disposable actual-code adapters from private sources and run native tools |
| src/observation.rs | Current-period absolute-time two-ray reference and original M5 WAV encoding |
| src/electrical/ | Pole convolution, warped analytical transfer, all-rate electrical measurements, committed carrier taps, timing and compiled electrical defects |
| src/identity/ and src/identity.rs | Independent correction bank, complete-window QR, historical offline epoch ABI, moving and stationary grading |
| src/pitch/ | Causal HP/FIR, direct-window differences, acquisition, lag and confidence diagnostics |
| src/audio.rs, numerics.rs | Canonical container and decoded identities, windows, FFT and direct complex projections |
| src/mutations.rs, contracts/ | Source edits, nominated first failures, historical state replay and fixture definitions |
| reference/ | Entry-stack fixture extraction and compact canonical hashes. These programs execute only for historical comparisons. |

The root production workspace and both CMake Cargo invocations continue to use the root lockfile. Production code has no dependency on the validation workspace. Processes running the existing Rust adapters own actual measurements. Oracle modules own expected values. The C++ epoch is the reviewed offline candidate. The full-window QR uses rectangular input samples. The independent Rust bank retains complex state separate from the epoch. The stationary grader receives the original converted M5 directly from the verified 30-column carrier. Four returned electrical paths enter the epoch, including the separately executed automated/factory path.

## Reproduction and results

| Operation | Rust subcommand |
|---|---|
| Canonical WAV bytes, metadata, normalized decoded samples | `fixtures` |
| Stored-graph vs actual solver, chronological ring | `contact`, `ring` |
| Rebuild named defective contact, ring, guide and O1 variants | `contact-mutants`, `mutations` |
| Captured physical/O1 parity, eager observed rays, canonical engineering M5 WAVs | `o1-replay` |
| Bitwise historical observed/unobserved private state | `o1-state` |
| O2 producer traces, original M5, returned taps, engineering WAVs | `o2-carrier` |
| Transfer, impulse and step, convolution and lifecycle | `o2-bench` |
| Actual callback workload receipts, exact terminal identity | `o2-timing` |
| Defective electrical lanes and harnesses | `o2-mutants` |
| Causal pitch, confidence and expected octave errors | `pitch` |
| Offline 108 stationary depths and nine unsafe cases | `i1-stationary` |
| Seven-rate motion and fourth-path delivery | `i1-motion` |
| Packed fields, 13 fitted vectors and every scheduled trial vs QR | `i1-qr` |
| Standalone bank, 17 little-endian f64 words in and six out per tick | `i1-bank-stream <rate>` |
| Native build, CTest and macOS signature/19 ABI exports | `native` |
| Run contact/ring/utility differential against historical sample words | `compare [reference-dir]` |
| All pitch rows or the original C++ four-path outputs | `pitch-compare [reference-dir]`, `i1-bank-compare [reference-dir]` |
| Physical scoring and original SciPy instantaneous pitch comparison | `guide`, `guide-compare [reference-dir]` |
| O2 acoustic windows, corners, matched WAV and listening ZIP, measured chirp and provenance negatives | `o2-acoustic`, `o2-acoustic-compare [reference-dir]`, `o2-sweep`, `o2-records` |
| Stationary source publications, independent moving decisions and source mutants | `i1-capture`, `i1-motion`, `i1-path-mutants` |
| Submitted and disputed prepared capacity, retained deadline and phase faults, surviving producer comparison | `i1-layout`, `i1-review82`, `signed-compare [reference-dir]` |
| Controlled source inventory, preregistered calibration and withheld scoring | `recordings <manifest>`, `repeatability <manifest>`, `recording-controls` |
| Actual late guide launch and all retained O1 descriptive measurements | `guide-negative`, `o1-compare [reference-dir]` after `o1-replay` |
| Full fresh O2 group and all 54 named negative controls | `o2-qualification` |
| Two independent lanes, signed delivery and M6 fault retirement | `i1-stereo` |
| Six recorded G3 sources, fifteen pair contrasts, matched historical archive | `i1-sources`, then `i1-listening` with the current `i1-motion` clips |
| Headless packaged VST3 with original dry/wet case inventory | `package` |
| Fresh nested suites, fresh suites plus historical comparisons, complete qualification gate | `regression`, `differential [reference-dir]`, `qualification` |
| Read the current invocation and check its artifact inventory | `inspect <suite>`, `artifact <suite> <relative-file>` |

Set `ELECTROBOW_VALIDATION_OUT` to use a different output directory. Default output is under `build/validation-migration/rust/<suite>/`. Each invocation has a unique directory. `current.json` identifies the current run, and `receipt.json` identifies its terminal status. Schema 3 binds run ID, source revision, executable digest, built validation source/contract digest, compiler/target/profile, runtime input inventory and artifacts. The reader accepts the earlier schema 2 snapshots. A successful run returns 0. A failed check, missing dependency, concurrent writer or malformed input exits nonzero. Inspection reconciles abandoned running receipts with the runner lock. JSON ingestion rejects nested duplicate keys, nonfinite values and overflowing literals. Optional measurements use JSON null.

Keep expected outcomes on the frozen side of a comparison. Record windows, units, signed-zero policy, FFT scaling, execution identity and eligibility rules with every fixture. Preserve actual counterexamples as executable source mutations. A failed compiler or missing input never earns a behavioral rejection. Permanent licensed audio stays at its attributed path. Generated reports, executables and audio belong under ignored build directories. Review V1 owns the cutover decision. Existing complete O2 and I1 exits continue to govern their reviewed scopes.


## Frozen comparison and review

The default comparison directory is `build/validation-migration/reference`. On arm64 macOS install the six pinned packages in the Python 3.9.6 environment named by `ELECTROBOW_REFERENCE_PYTHON`, build the original Release package, then run `sh tools/validation-rs/reference/reproduce.sh` from the project root. This checks out the entry revision, runs the historical suites, exercises the surviving half-product variant and exports comparison data. Freeze scripts are reference-side extraction tools. Run `"$v" differential` to execute both new producers and comparisons. The nested invocation uses a single copied executable throughout and writes one child inventory per aggregate. `qualification` returns nonzero while complete-exit coverage is open. [Historical environment](../../tools/validation-rs/reference/requirements-macos-arm64.txt) · [Disposition and open owners](status/STAGE_V1_RUST_VALIDATION_MIGRATION.md)
