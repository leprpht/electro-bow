# V1 validation migration

Entry was a clean checkout of `feature/bowed-string-engine` at `fcc272f8ec7a0472917c1e787edbd2918dcc3ae0`. Stage 08.2 is the latest implementation submission. Review 8.2 owns the qualification boundary. The root Cargo workspace contains the edition-2024, dependency-free `electrobow-dsp` static library and rlib. CMake and disposable historical mutation builds consume its locked offline workspace.

## Inventory and ownership

The [tab-separated inventory](validation-inventory.tsv) lists individual tracked owners, entry functions/tests, purpose, input dependencies, outputs, reverse Python imports, qualification document, historical uses, entry-source hash and retirement disposition. Scope includes project qualification, the Rust and C++ harnesses and adapters, CI, UI regression, native orchestration and the permanent corpus. Third-party library self-tests belong to their upstream projects. Category numbers denote qualification (1), mathematical reference (2), actual-code adapter (3), regression (4), negative control (5), experiment (6), timing (7), artifact (8), historical replay (9) and superseded (10). Sharing an algorithm name does not establish redundancy.

| Family | Inputs and math | Required output and original controls |
|---|---|---|
| Contact | Target-prepared binary64 coefficient words, independent stored-graph intersections and exact barycentric interpolation | 80,500 sweep + 4,374 fold + 12 floor/tie rows, branch and signed slip, ≤4 ULP, flux and dissipation. Cutoff, static and lower-root sources must compile and diverge. |
| Ring | Fixed read-before-write impulse, chronological seed history, delay changes and wrap | Absolute circuit phase ≤.003 samples, exact eager/lazy values and named compiled ring regressions. |
| Guide | Prescribed Hz, speed and pressure, eager independent evolution | M4/5/6, 152 physical cases, canonical traces/audio, contacts and epochs, stress and per-mutation test panic. |
| O1 | Committed nut emissions, current period and original seed | Absolute-time rays, three velocities and aperture ≤1e−9 m/s. Physical and M5 words exact. Historical state, cost and observer defects. |
| O2 | Actual O1, independent warped analog transfer and pole convolution | Sweep, impulse, depths, V17, lifecycle, high band, timing, canonical samples and publication. Exact required fields/windows/controls in O2 acceptance matrix. |
| A1/A2 and controller | Causal DI and injected events, decimation, lag scan and selection | Named passes and octave misses, CSV case inventory, lifecycle tests. Final selector held. |
| I1 | Producer-owned electrical paths and original M5, offline bank, host-time basis, independent QR | 108 stationary corrected passes, nine historical failures, 700 moving cells on each path, signed and cache outputs, lifecycle, mutation and receipt controls. Review 8.2 records the surviving defects. |
| Audio and recordings | Canonical files, shared-session intervals, attack logs, float and integer sample domains | File and decoded sample digests, engineering gain, independent matching gains, listening key/archive, provenance and controlled holdout. |
| Native | Existing ABI and actually packaged VST3, native JUCE/CTest | Exports, measured latency, partition independence, preset compatibility, wet, neutral and tail regression. |

Exact seeds, input schedules, field names, threshold predicates, fixture manifests and failure sites reside in the file identified by each entry digest at the entry revision. Historical report commands keep their original meaning. Generated traces and old report copies belong under ignored build paths.

## Architecture and sequence

Use a separate Cargo workspace at `tools/validation-rs`. The root manifest and lockfile are inputs to both the native build and historical mutation scripts that copy only `rust/dsp` into temporary roots. A nested workspace preserves those callers and keeps all validation dependencies outside the shipping dependency graph. Production execution occurs in existing replay processes. Oracle calculations have no Cargo dependency on the DSP crate.

Freeze original executable outputs and fixtures, verify numeric and evidence primitives, then port the carrier, observation, electrical, analysis/controller and identity suites. Qualify each normal exit and its real defective variants before advancing authority. Preserve the offline C++ kernels through independently approved equivalence. Retire only through a subsequent Review V1 cutover. The [equivalence matrix](VALIDATION_RUST_EQUIVALENCE_MATRIX.md) records which work has reached each boundary.
