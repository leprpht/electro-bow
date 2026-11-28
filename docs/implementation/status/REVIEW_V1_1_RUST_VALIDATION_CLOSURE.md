# Review V1.1 · Final Rust validation closure

**Accept V1-1, V1-2 and V1-3. Rust is ElectroBow’s authoritative validation infrastructure and the default for all future permanent validation.** Review 8.2 continues to govern DSP qualification and implementation authority. [Review 8.2 authorization](REVIEW_08_2_I1_STREAMING_QUALIFICATION.md#final-authorization)

## Reviewed revisions

Reviewed `3905767c0488f382b91aa64cebc4d7e8f1356e8e..dd1371029fbb3fa5f7b676ec2da2e07d3f495b60` on a clean `feature/bowed-string-engine`. The range includes the three corrections at `261095a81d4069bdd5e9cb72335812906a889cd7`, the extensible big-endian float64 correction at `000bd410e49800cce4b6bb69c0b04ef89a318854`, and the submitted evidence at `dd13710`. The validation source and contracts at the last two revisions are identical. The older counterexamples used the Review V1 executable at `07a115d9597fa83f5626d5a658fcfed6b8a9981a`. Frozen comparisons used the clean historical checkout at `fcc272f8ec7a0472917c1e787edbd2918dcc3ae0`. [Review V1 findings](REVIEW_V1_RUST_VALIDATION_MIGRATION.md) · [Submitted corrections](STAGE_V1_1_RUST_VALIDATION_CLOSURE.md) · [Equivalence matrix](../VALIDATION_RUST_EQUIVALENCE_MATRIX.md)

The release binary SHA-256 was `e4a37d5376257438e945731dfd0641c24cd43d1476cd5ebd988706fa116caf7b`. Review logs, receipts and temporary controls are under ignored `build/review-v1-1/` unless otherwise noted. See `targeted.log` for the three counterexamples and `final-audit.log` for the aggregate cross-checks.

## V1-1 · Stage 08 archive

An independent ZIP reader compared the 17 names in order and every uncompressed member. All 14 WAVs, attribution and listening instructions were byte-identical. The sole differing member was `listening-gains.json`, where six gains were bitwise identical and the Rust measurement for `techs-P2-60-held` was exactly one binary64 ULP lower. The producer calculates the gain from the measured anchor window, writes it to the ZIP and uses the same value for matched rendering. The artifact audit records both gains.

| Stage 08 archive | SHA-256 | Gain | Binary64 word |
|---|---|---:|---:|
| Frozen canonical archive | `90222fb6bd1590713f287527607771d1db7d1f36122da5d0020bad70d96c9809` | 0.4811158254466993 | 4602338632126721134 |
| Fresh Rust archive | `0f6a5e212c3d751702ae6e3c16bf8d43db45fc1095b2ef1d05e3a9c42dd5a89c` | 0.48111582544669923 | 4602338632126721133 |

The original ZIP remains at its frozen path and digest. The historical differential and the artifact audit admit only the canonical ZIP or this exact second digest, along with the contracted member and gain checks. The aggregate exception names only Stage 08 `listening.zip` and this digest pair. I changed an unrelated gain and separately changed a WAV payload byte in copied producer runs. Each control updated the archive declaration, artifact hashes and both receipts. Each reached `i1-study-compare` and exited 1 at `measured Stage 08 archive digest`. The copied original Rust archive passed the same entry point. O2 and Stage 08.2 still require whole-archive equality with their frozen ZIPs. [Artifact audit](../../../tools/validation-rs/src/identity/artifacts.rs) · [Frozen comparator](../../../tools/validation-rs/src/identity/study.rs)

## V1-2 · Recording eligibility and identity

I widened only A1 in a copy of the frozen synthetic cell from IEEE float32 to float64 and updated its original-container hash. The frozen SciPy preflight accepted eight takes with zero failures. Review V1’s Rust entry exited 1 with `A1: Ill-formed WAVE file: bits per sample is not 32`. The corrected `recordings` command exited 0 for both containers and accepted eight takes each. The complete preflight reports agreed after accounting for the original-container and manifest digests. A1 remained mono at 48 kHz with attack sample 0 and interval [0, 17280).

| A1 identity | SHA-256 |
|---|---|
| Decoded audio and segment in either container | `5e63567f8ed1c17dbf12cb85ca74b2974b5f1e084f9dc32d69761e8460654d07` |
| Capture in either container | `a9b8dc0b1aaa686b9a7b7a1a20900d9b9e169d2fe05406cbde6c54add6b02fe1` |
| Original float64 container | `97bdcf101727e55381ce35dcaaa0d2e52cef10bb5c2a43588f3f32419355883d` |

Each `repeatability` command still exited 1 for the withheld wet signed cue. Both had 6.272484490268843 dB DI and 2.0510330924123563 dB wet withheld margins. Putting the float32 copy of A1 in the A2 slot beside float64 A1 still rejected duplicate decoded recordings, duplicate captures and overlapping segments. The fresh aggregate also exercised separate originals, shared-session eligibility and chronological attack controls. The change preserves the PCM and float32 paths and the sample-rate, channel, finite-sample and clipping gates. Cargo tests covered native float64 sample precision, signed zero, truncation, nonfinite data and extensible big-endian decoding. [Preflight and controls](../../../tools/validation-rs/src/recordings.rs) · [Float64 decoder](../../../tools/validation-rs/src/recordings/float64.rs)

## V1-3 · Schema 3 provenance

I removed `inputs`, `rustc`, `target` and `profile` from a copied passing schema 3 fixtures run and recalculated the SHA-256 in both `current.json` and `receipt.json`. The prior `inspect fixtures` returned 0 and reported `passed`. The corrected command exited 1 at `missing rustc identity`. Each field also rejected when removed individually. Deleting the `Cargo.toml` inventory entry or substituting its digest rejected at `runtime input inventory or digest mismatch`, with both receipts rehashed.

A schema-only downgrade of the modern receipt was rejected by the historical revision gate. An unchanged historical schema 2 fixtures receipt at `7347e82a43ffbec152109b1a42476b6adc552dce` passed. Fresh aggregate children require schema 3. Schema 3 inspection checks the runtime path/digest map against the inspection checkout and checks embedded build fields when the producing executable is used.

A clean copy of the O2 aggregate passed inspection. Changing a child run ID in both the inventory and the parent evidence and rehashing those files in both parent receipts failed at `run-local producer inventory`. The inspector reopens the run-local child receipts, checks the parent executable and compares the linked evidence digests, revisions and run IDs. The complete entry also validates the comparison, listening-source and I1 producer/artifact links. [Schema reader](../../../tools/validation-rs/src/publication.rs) · [Aggregate link validation](../../../tools/validation-rs/src/qualification.rs)

## Complete qualification

The macOS process-exec profile at `build/review-v1/no-python.sb` was inherited by the Rust runner and every child. An attempt to execute `/usr/bin/python3 --version` under the same profile exited 71 with `Operation not permitted`. The SciPy preflight control and independent ZIP and receipt audits ran separately from qualification.

| Python-blocked command | Exit | Fresh evidence |
|---|---:|---|
| `qualification build/validation-migration/reference` | 0 | `rust/qualification/run-bxbQr5`, 738.85 s. All 43 regression producers and 14 frozen comparisons passed. The unresolved artifact inventory is empty. |
| `o2-qualification` | 0 | `rust/o2-qualification/run-MDKMxB`, 142.81 s. All seven producers passed and all 54 electrical negative controls rejected. |
| `inspect qualification` and `inspect o2-qualification` | 0 each | The published aggregates passed reinspection. |

Both commands used the same release executable, built-source SHA-256 `1e3243fc9c3f08b98479c102670218cdd8bfbaba8772e3efbf56d805eccb5c3b` and Rust 1.100.0 for `aarch64-apple-darwin`. Both provenance records matched the independently hashed 131-file runtime inventory. The independent receipt audit matched all 57 child receipts, 14 comparison links, 14 I1 evidence producer links and 105 named artifact links. Of the 105 artifacts, 73 were byte-identical, 31 retained the measured fields while binding invocation-local renderer identities, and one used the Stage 08 disposition. The ZIP member attributes, archive comments and compressed byte streams outside the gain manifest also matched. O2 and Stage 08.2 archives matched their frozen originals byte-for-byte with SHA-256 `39dd496030d731b3d8bbf39c15dd420de61d116f89fa1bb6b61eb6483c497712` and `34041868c3b5a7e1737b89e28e03bcebfa2af9827a4b6c894d3958bbb98d91d6` respectively.

The differential preserved all 40,350 pitch classifications and checked 62,088 joined fields with maximum scaled error 1.76e−15. All 108 revised stationary depth checks passed, the nine historical unsafe cases failed as expected and the retained candidate still missed exact withdrawal at five rates. All 16 original I1 evidence substitutions rejected. The defective complete entries failed at their registered predicates `bank joined sample column 10`, `surviving substituted original M5` and `surviving rolled wet valid tags`. [Existing differential and mutation evidence](REVIEW_V1_RUST_VALIDATION_MIGRATION.md#representative-negative-controls)

Both Cargo workspaces passed formatting, locked offline strict Clippy across all targets, and locked offline workspace tests. The root workspace passed 89 tests with the isolated timing probe ignored. The validation workspace passed eight tests. Its separate packaged audition passed in Release under the Python block. The aggregate passed all four native CTests. [Qualification commands](../VALIDATION_RUST_GUIDE.md#qualification-commands)

## Production behavior and qualification limits

The diff from `fcc272f8` through the reviewed head is empty for production Rust, native plugin source, UI, CMake, root Cargo manifests and lockfile, release workflow, original validation sources and shipping scripts. The locked root Cargo graph contains only `electrobow-dsp`. Fresh native and packaged suites exercised CTest, the ABI and the shipping VST3. The recorded guitar test reproduced the eight frozen PCM24 files and recalled the saved state sample-identically at 17 and 1,024 frames after rendering at 127 frames.

This review ran on arm64 macOS 26.5.1 with Rust 1.100.0 and the configured Release native build. Platform-specific hosted results for Windows, Linux and native Intel were outside the evidence available for this closure. Recording identity used the frozen synthetic cell. Real controlled recordings and owner listening remain pending. Schema 3 inspection uses a checkout with the recorded runtime input inventory. The Python execution block covers qualification and its child processes. Regenerating the historical reference data uses the pinned historical Python environment. Existing build and release wrappers retain their original behavior.

## Cutover and roadmap

Use Rust for all future permanent validation. Retain every existing Python validation file and independent offline C++ validation file in place for historical comparison and replay. The standalone Rust validation workspace owns qualification and requires no parallel maintenance of historical implementations. Keep the native adapter, existing offline C++ candidate and packaged host at their actual-code boundaries.

S2a/S2b, M4/M6, isolated O1/O2 and injected baseline S4a retain their reviewed scope. Preserve the known A2 misses, single withheld wet signed-cue failure in the synthetic cell, nine historical unsafe I1 rows and exact-withdrawal misses at five rates. The Review 8.2 holds on signed delivery and final A4, exact withdrawal, prepared layout and strict dependent evidence remain in force at their specified boundaries. Final A2 and live tracking, empirical source identity, owner listening, amended gesture/S4b, supported S5, integration and release retain their separate gates.

Resume Review 8.2’s single offline transaction and qualification revision. Grade signed pre-link output and cached fault fade from actual samples, enforce exact permission withdrawal, freeze complete field ownership and close strict dependent evidence ingestion. Re-run the full nominal and defective exits, including the half-product and late-neutral witnesses at the producing boundary. [Next authorized increment](REVIEW_08_2_I1_STREAMING_QUALIFICATION.md#final-authorization)
