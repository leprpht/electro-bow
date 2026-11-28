# Stage V1.1 · Rust validation closure

**Submitted for focused independent Review V1.1.** The proposed cutover gives the standalone Rust validation workspace authority over the migrated qualification suites after independent acceptance. Existing stage exits retain their reviewed authority until that decision. [Review V1](REVIEW_V1_RUST_VALIDATION_MIGRATION.md) · [Equivalence matrix](../VALIDATION_RUST_EQUIVALENCE_MATRIX.md)

Entry was a clean, synchronized `feature/bowed-string-engine` at `3905767c0488f382b91aa64cebc4d7e8f1356e8e`. Final implementation and qualification revision is `000bd410e49800cce4b6bb69c0b04ef89a318854`. Its direct successor contains the review documentation. The validation executable SHA-256 was `e4a37d5376257438e945731dfd0641c24cd43d1476cd5ebd988706fa116caf7b`. Both aggregates used the same release build, built-source digest `1e3243fc9c3f08b98479c102670218cdd8bfbaba8772e3efbf56d805eccb5c3b`, Rust 1.100.0 and target `aarch64-apple-darwin`. Evidence below is relative to the ignored `build/stage-v1-1/` directory unless indicated otherwise.

## V1-1 · Stage 08 archive

A separate ZIP reader compared the frozen archive and Review V1's Rust output. It checked all 17 names in order, compared every uncompressed member and read the differing gain as two binary64 words. The 14 WAVs, attribution and listening instructions matched byte-for-byte. Of the seven gains in `listening-gains.json`, only `techs-P2-60-held` differed. See `archive.log`. A second member-by-member check on the final fresh producer `run-qQpZe2` reproduced both digests and gain words (`verified-member-audit.log`).

| Archive | SHA-256 | Gain | Binary64 word as unsigned decimal |
|---|---|---:|---:|
| Frozen Stage 08 | `90222fb6bd1590713f287527607771d1db7d1f36122da5d0020bad70d96c9809` | 0.4811158254466993 | 4602338632126721134 |
| Measured Rust | `0f6a5e212c3d751702ae6e3c16bf8d43db45fc1095b2ef1d05e3a9c42dd5a89c` | 0.48111582544669923 | 4602338632126721133 |

The frozen ZIP remains the canonical replay artifact. The producer still computes the gain from its measured anchor window and uses that gain for matched rendering. The artifact audit accepts exact equality or this specific pair of whole-archive digests. It also checks the contracted member inventory and hashes, exact gain keys, six identical gain words and the one downward ULP. The frozen differential reads both ZIPs, checks entry order and compares actual member bytes. Pinning the complete Rust ZIP also binds JSON formatting and container metadata. The aggregate admits this disposition only for Stage 08 `listening.zip`. Stage 08.2 and O2 continue to require byte-identical archives. [Artifact audit](../../../tools/validation-rs/src/identity/artifacts.rs) · [Frozen comparison](../../../tools/validation-rs/src/identity/study.rs)

## V1-2 · Recording eligibility and repeatability

The Review V1 recoder copied the frozen synthetic cell and converted only A1 to IEEE float64, updating only its container hash in the manifest. The frozen SciPy recording preflight accepted eight takes with zero failures. The entry Rust executable rejected A1 at “bits per sample is not 32”. The corrected public `recordings` command accepted all eight. A1 remained mono at 48 kHz, attack sample 0, interval [0, 17280). The decoded and segment SHA-256 remained `5e63567f8ed1c17dbf12cb85ca74b2974b5f1e084f9dc32d69761e8460654d07`, capture `a9b8dc0b1aaa686b9a7b7a1a20900d9b9e169d2fe05406cbde6c54add6b02fe1`. The new original-container digest was `97bdcf101727e55381ce35dcaaa0d2e52cef10bb5c2a43588f3f32419355883d`. See `python-preflight.log`, `before/recordings/run-perPoH/recordings.json` and `verified-recordings.log`.

The Rust binary64 reader decodes finite sample words directly into f64. Existing PCM scaling, float32 widening, mono/rate eligibility, clipping, original/decoded hashing, capture and duplicate checks remain in their existing paths. A permanent word-level test covers a sample between adjacent binary32 values, signed zero, truncation and NaN. A second test exercises the original reader’s big-endian extensible IEEE subformat. The disk-backed eight-take control compares complete preflight and scoring data after accounting for the manifest and container hashes. The public `repeatability` exit remains 1 on this cell for the single withheld wet signed cue. DI and wet withheld margins were 6.272484490268843 and 2.0510330924123563 dB. [Recording controls](../../../tools/validation-rs/src/recordings.rs)

## V1-3 · Schema 3 ingestion

A fresh copy of Review V1's passed fixtures run `run-CVrSUI` has inputs, rustc, target and profile removed with the new provenance SHA-256 in both current and terminal receipts. The entry inspector returned 0 and reported passed. The corrected inspector returned 1 at “missing rustc identity”. The permanent CLI test creates a new passing fixtures suite, copies it, performs both coherent hash updates, and tests the four-field removal and each single-field removal. See `reproduce-old-inspect.log`, `verified-reject.log` and `final-test-validation.log`.

Schema 3 inspection validates run, suite, revision, executable and built-source identity, compiler, target, profile and the complete runtime path-to-digest map against the checkout used for inspection. The producing executable additionally verifies its embedded build fields. Aggregate publication and inspection reopen the run-local child receipts and verify their schema, revision, executable, run ID, elapsed time and evidence digest. They bind the copied runner, comparison inventory, I1 evidence sources, listening sources and artifact links. A second copied O2 control changed a child ID in both inventory and evidence, rehashed both artifacts in both parent receipts, and failed at “run-local producer inventory” (`reject-stale-link.log`). Strict duplicate-key/finite JSON and file digest checks remain active. Historical schema 2 fixtures at `162fd101` passed the public inspector (`schema2.log`), and fresh aggregate children require schema 3. [Receipt reader](../../../tools/validation-rs/src/publication.rs) · [Aggregate checks](../../../tools/validation-rs/src/qualification.rs) · [Permanent counterexample](../../../tools/validation-rs/tests/publication.rs)

## Complete qualification

| Python-blocked command | Exit | Final evidence |
|---|---:|---|
| `o2-qualification` | 0 | `verified/o2-qualification/run-xBmvSp`, 143.25 s. Seven fresh children. All 54 negative controls rejected. O2 ZIP SHA-256 `39dd496030d731b3d8bbf39c15dd420de61d116f89fa1bb6b61eb6483c497712`. |
| `qualification build/validation-migration/reference` | 0 | `verified/qualification/run-NVSkmb`, 726.59 s. All 43 regression producers and all 14 frozen comparisons passed. The unresolved difference inventory is empty. |
| `inspect qualification`, `inspect o2-qualification` | 0, 0 | Both complete published receipts passed reinspection. A separate run of Review V1’s link audit matched all 57 child receipts, 14 comparisons, 14 I1 evidence producer IDs and 105 named artifacts (`verified-links.log`). |

Of the 105 artifacts, 73 match byte-for-byte, 31 bind invocation-local renderer identities, and one uses the scoped Stage 08 disposition. The Stage 08.2 ZIP retained SHA-256 `34041868c3b5a7e1737b89e28e03bcebfa2af9827a4b6c894d3958bbb98d91d6`. All 40,350 pitch rows retained their classifications, including the existing octave misses. The joined comparison graded 62,088 fields with maximum scaled error 1.76e−15. The 108 revised stationary depth checks passed, all nine historical unsafe cases failed, and the retained candidate still missed exact withdrawal at five rates. All four native CTests passed.

The three defective complete-entry variants failed at their nominated predicates of `bank joined sample column 10` (half-product), `surviving substituted original M5` and `surviving rolled wet valid tags`. All 16 original I1 evidence substitutions also rejected. The fresh per-variant parent and child receipts are recorded in the aggregate’s `i1-transaction-mutants` evidence.

An earlier O2 run on the initial implementation passed. Its full run stopped at the built-source identity check when the extensible RIFX correction changed the source tree. The final runs above used the revised binary throughout.

The execution sandbox used the Review V1 process-exec denial profile. Invoking `/usr/bin/python3 --version` under the same profile exited 71 with “Operation not permitted”. The historical Python preflight was a separate reproduction outside that sandbox. Rust qualification consumed frozen data from the original checkout at `fcc272f8ec7a0472917c1e787edbd2918dcc3ae0`. Reproduce from the repository root on the review host with its configured native build.

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
m=tools/validation-rs/Cargo.toml
v=tools/validation-rs/target/release/electrobow-validation
cargo build --release --locked --offline --manifest-path "$m"
export ELECTROBOW_VALIDATION_OUT="$PWD/build/stage-v1-1/replay"
profile=build/review-v1/no-python.sb
/usr/bin/sandbox-exec -f "$profile" "$v" o2-qualification
/usr/bin/sandbox-exec -f "$profile" "$v" qualification build/validation-migration/reference
/usr/bin/sandbox-exec -f "$profile" "$v" inspect qualification
```

Formatting, locked offline strict Clippy across all targets and workspace tests passed in both Cargo workspaces. Root had 89 passes and one isolated timing probe ignored. Validation had eight passes. Its additional packaged audition test passed when explicitly invoked in release mode under the Python block (`build/validation-migration/packaged-audition/run-lBHsuc`).

## Authority and independent review

Review V1.1 should independently reproduce the ZIP/member and ULP checks, same-sample recording and coherently rehashed receipt, then verify the two complete exits. On acceptance, use the Rust qualification entry points for permanent validation. Keep the historical Python and independent offline C++ sources frozen in place with no synchronization obligation. The existing native adapter, offline C++ epoch and packaged host continue to exercise their actual-code boundaries.

S2a/S2b, M4/M6, O1, isolated O2 and injected baseline S4a retain their reviewed scope. The existing A2 misses and the Review 8.2 hold on isolated I1 implementation stand. Signed delivery, exact withdrawal, layout ownership, empirical source identity, owner listening, live tracking and gesture, supported wet/S5, integration and release retain their separate gates. The comparison against the frozen revision found no changes to production DSP, routing, public ABI, JUCE parameters, presets, UI, fallback or historical validation sources. [Review 8.2 authorization](REVIEW_08_2_I1_STREAMING_QUALIFICATION.md#final-authorization)
