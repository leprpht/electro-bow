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

## Dependency and arithmetic decisions

The standalone Cargo.lock pins all transitive versions. The all-target metadata graph resolves 93 packages including development and target-specific packages. Cargo metadata records source and license per package. The locked toolchain is Rust 1.100.0, edition 2024. Validation uses the host allocator and filesystem. The shipping lockfile remains independent.

| Dependency | Reason and numerical boundary |
|---|---|
| rustfft 6.4.1 | Complex f64 forward FFT uses negative phase and no normalization. Explicitly truncate to the one-sided spectrum, keep Nyquist and divide by transform length after inverse convolution. [RustFFT normalization](https://docs.rs/rustfft/6.4.1/rustfft/) |
| num-rational 0.4.2, num-traits 0.2.19 | Convert stored binary64 line endpoints and queries to exact rationals. Select all admissible intersections before rounding the chosen answer. [Rational arithmetic](https://docs.rs/num-rational/0.4.2/num_rational/) |
| nalgebra 0.34.1 | Rectangular QR on the actual complete window provides a separate solver from the C++ packed streaming normal equations. [QR decomposition](https://docs.rs/nalgebra/0.34.1/nalgebra/linalg/struct.QR.html) |
| hound 3.5.1 | Read WAV metadata and integer/float payloads. The Stage 03 writer recreates the original float RIFF/fact layout when container bytes are contractual. [Hound](https://docs.rs/hound/3.5.1/hound/) |
| serde, serde_json, csv | Schema-bound records, round-trip decimal input and streaming pitch rows. A custom recursive visitor rejects duplicate keys and overflowing/invalid numbers. [serde_json features](https://docs.rs/crate/serde_json/1.0.145/features) |
| sha2, tempfile, libloading | Streaming SHA-256, isolated builds and atomic artifacts, checked native offline ABI. [SHA-2](https://docs.rs/sha2/0.10.9/sha2/) · [Native library binding](https://docs.rs/libloading/0.8.9/libloading/) |

Use standard library argument handling for the finite suite inventory. Authoritative timing invokes the original callback probe. Its frame counts, warmup, batches, terminal tests and pass thresholds remain in the original contract. Test threshold decisions at the real boundary. Binary32 conversion occurs only at the declared audio or ABI step. Freeze ties-to-even, exact interval endpoints, DC exclusion, subnormal eligibility and signed-zero conventions per suite. The stationary smoother uses the multiply-add order observed in the pinned SciPy build. Preserve failed trial receipts when a numerical discrepancy is investigated. The matrix records qualified surfaces and pending comparisons. [Equivalence matrix](VALIDATION_RUST_EQUIVALENCE_MATRIX.md)

The listening writer uses zip 2.4.2 with its zlib backend. The historical archive fixes entry order, timestamps, compression and file attributes. Compare the complete ZIP, key, individual WAV containers and decoded sample identities separately. Use `cargo metadata --locked --offline --format-version 1 --manifest-path tools/validation-rs/Cargo.toml` for the resolved versions, features and license expressions.

The locked metadata declares MIT or Apache 2.0 choices for RustFFT, the num crates, Serde, serde_json, SHA-2 and tempfile. Hound and nalgebra declare Apache 2.0, libloading declares ISC, zip declares MIT and csv declares Unlicense/MIT. The graph also includes Unicode data terms and target-specific compiler support. The highest declared minimum Rust version among direct dependencies is nalgebra's 1.87. The tested toolchain is 1.100. The native host still requires the existing JUCE license and toolchain. No validation crate is linked into the shipping plugin.

## Retirement and historical replay

The inventory columns `replacement`, `equivalence_evidence`, `migration_status`, `historical_dependencies` and `retirement` form the per-file proposed retirement manifest. Keep all original owners and historical reviews in this submission. After independent approval, remove only approved superseded implementation rows in a dedicated cutover. Retain permanent attributed audio, existing native/ABI tests and actual-code Rust replay boundaries. A C++ adapter that hosts JUCE or measures the reviewed kernel remains necessary until the corresponding independent comparison is approved. Unclosed family rows retain the existing executable authority. The historical revision and pinned replay command remain available after any future cutover. [Per-file retirement manifest](validation-inventory.tsv) · [Frozen replay instructions](VALIDATION_RUST_GUIDE.md#frozen-comparison-and-review)

### Exceptions submitted for independent review

The historical Stage 04 identity/gesture listening experiment, string-parameter exploration and ledger replay retain their entry programs so reviewers can reproduce the reports cited by earlier design decisions. The current offline study, moving identity, stationary V17, original observation and electrical reports have separate Rust owners. The original NumPy/SciPy extractors run only when rebuilding frozen evidence. The host and offline C++ epoch exercise actual native boundaries. Real controlled takes, listening responses and native platform attestations require their respective external owners.

The independent harmonic projector uses a symmetric window and centered sample time. On macOS the original environment's non-conjugating complex dot reduction is available from Accelerate. Linux and Windows use the scalar implementation. In very weak harmonics different reduction orders can affect phase eligibility or reported phase. Independent platform differentials are still required. The original thresholds apply to every path.
