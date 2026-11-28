# Review V1 · Rust validation migration

**Hold repository-wide authoritative cutover.** Rust is the default for future permanent validation. The fresh electrical exit passed without Python. Complete cutover remains held by the canonical artifact gate, a recording eligibility regression and incomplete schema 3 provenance validation. Existing stage exits retain their reviewed authority. Review 8.2 remains the roadmap boundary. [Review 8.2 authorization](REVIEW_08_2_I1_STREAMING_QUALIFICATION.md#final-authorization)

## Reviewed scope

- Reviewed `fcc272f8ec7a0472917c1e787edbd2918dcc3ae0..07a115d9597fa83f5626d5a658fcfed6b8a9981a` on `feature/bowed-string-engine`, with a clean tracked tree at entry. The submitted aggregate was measured at `9bc8880b7d35d0d3a237383a50f87112587e13a4`. `8f9ed63` adds the packaged recording test. The frozen comparator checkout was clean at `fcc272f8`.
- The independent workspace covers contact/ring/guide and O1, O2 electrical and acoustic qualification, A1/A2 conditioning and pitch analysis, capture and sample-clock ownership, I1 bank/projection and streaming, numerical faults, stereo M6, signed pre-link A4, evidence publication, negative controls, fixtures and native/plugin regression. The compared scopes include all eleven historical I1 reports through their mapped producers. The joined history has direct field and clock comparisons. Synthetic recording identity and runtime dependencies have the limitations below. Historical Stage 04 identity/gesture auditions and standalone parameter exploration retain their research owners. [Per-suite and report ownership](../VALIDATION_RUST_EQUIVALENCE_MATRIX.md)
- `git diff --exit-code fcc272f HEAD -- Source UI rust Cargo.toml Cargo.lock CMakeLists.txt cmake JUCE ThirdParty tests tools/validation .github/workflows/release.yml build.py run_tests.py` returned 0. Production DSP, shipping routing and fallback, ABI, JUCE parameters, presets and UI are byte-for-byte preserved in the reviewed sources. The shipping Cargo graph contains only `electrobow-dsp`. Validation owns its own dependency graph and lockfile.

## Fresh execution

Executed on arm64 macOS 26.5.1 using Rust 1.100.0, Apple Clang 17 and CMake 4.4.3. Logs, exit codes, receipts and probe files are under `build/review-v1`. Aggregate children used one copied executable, SHA-256 `e5eed1f37aca59ccf5bf23acd8874c93a95129bb563dcdf82e3ccedc01b7030c`, and produced run-local evidence.

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
m=tools/validation-rs/Cargo.toml
v=tools/validation-rs/target/release/electrobow-validation
cargo build --release --locked --offline --manifest-path "$m"
mkdir -p build/review-v1
cat >build/review-v1/no-python.sb <<'PROFILE'
(version 1)
(allow default)
(deny process-exec (regex #"/([Pp][Yy][Tt][Hh][Oo][Nn][Ww]?|[Pp][Yy][Pp][Yy])([0-9.]*)(/|$)"))
PROFILE
export ELECTROBOW_VALIDATION_OUT="$PWD/build/review-v1/rust"
/usr/bin/sandbox-exec -f build/review-v1/no-python.sb "$v" o2-qualification
/usr/bin/sandbox-exec -f build/review-v1/no-python.sb "$v" qualification build/validation-migration/reference
```

The sandbox inherited by child processes denied executable basenames matching `python`, `python3`, versioned Python, `pythonw` and `pypy`. The direct `/usr/bin/python3 --version` control returned 71 with `Operation not permitted`. The Rust runner reads frozen comparison data and compiles actual-code Rust adapters and the preserved C++ offline epoch. The original stage exits pending cutover still use Python. Existing build and release conveniences and hosted packaging CI also use Python wrappers. Direct Cargo and CMake support local development and the Rust runs.

| Command or check | Result |
|---|---|
| `o2-qualification` | Exit 0, `run-RKe5iM`, seven fresh children in 141 s wall time. All 54 O2 negative controls rejected and the O2 listening archive is byte-identical. |
| `qualification build/validation-migration/reference` | Exit 1, `run-mfWoMb`, 700 s wall time. All 43 regression children and 14 frozen comparisons passed. `unqualified.json` names only `listening.zip`. |
| Both workspaces, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`, `cargo test --workspace --locked --offline` | All six exit 0. Root 89 passed and one isolated timing probe ignored. Validation five passed and the packaged audition was run separately. Used `--manifest-path "$m"` for the standalone workspace. |
| `compare`, `i1-qr`, `i1-review82`, `signed-compare`, `package` | Each exited 0 under the same Python block in `build/review-v1/probes`. `i1-review82` is a successful reproduction of known failures. The packaged baseline matched its frozen payloads and reported zero state partition error. The aggregate also passed all four native CTests. |
| `cargo test --release --locked --offline --manifest-path "$m" --test packaged_recordings -- --ignored --nocapture` | Exit 0 with Python blocked. All eight PCM24 files match frozen hashes. Saved state at 127 frames recalls sample-identically at 17 and 1,024 frames. Receipt under `build/validation-migration/packaged-audition/run-1eLaju`. |
| `recordings <manifest>`, `repeatability <manifest>` on the existing synthetic cell | Preflight exit 0. Scoring exit 1 for the same withheld wet signed cue as the frozen result. DI/wet withheld margins match at 6.27248/2.05103 dB. |

The combined command runs the same producers and comparisons as `regression` and `differential`, then applies the canonical archive gate. The independent receipt audit matched all 57 child IDs and evidence hashes, all 14 comparison links, the 14 I1 evidence producer IDs and all 105 artifact links. Of the 105, 73 are byte-identical, 31 differ only in invocation-local renderer identity and one has the gain difference below. Frozen Python outputs supplied the historical side. [Earlier complete runs](STAGE_V1_RUST_VALIDATION_MIGRATION.md#complete-exit-records)

## Numerical and behavioral checks

- **Independent math.** Contact uses exact rational interpolation on the compiled coefficient graph, ring uses chronological history and O1 uses absolute-time rays. O2 compares compiled taps to warped analog transfer and pole convolution. A1/A2 uses causal conditioning and direct-window differences. All 40,350 pitch rows match the frozen classifications, including the known octave misses. I1 uses a separate complex-bank model and rectangular QR on raw sample windows to grade the compiled C++ bank and packed Gram. In fresh QR, the worst coefficient difference was 5.44e−15 and worst residual difference was 6.16e−15 FS². All candidate and realized decisions matched, including different final trial decisions on the two near-threshold cases and the finite rank veto. The joined comparator covered 62,088 fields with maximum scaled error 1.76e−15.
- **Spectra and rounding.** An independent direct DFT checked four frozen vectors, including odd length, an impulse and an alternating-sign 16-point vector whose Nyquist coefficient is 16. The fresh 36-vector Rust comparison had a largest FFT component difference of 2.85e−14. The implementation keeps the unnormalized one-sided FFT, the symmetric Hann and ties-to-even where declared. Partial projections exclude frequencies at or above Nyquist. At 48 kHz, each depth matched the analytical cutoff magnitude and Nyquist response. Phase/delay were null for the Nyquist zero-depth floor. Signed-zero checks preserve sample words at neutral transitions and normalize zero only for declared audio identities.
- **V17 and sample clock.** All 60 O2 predicate maps match the frozen report. Recomputing from the 48 kHz/196 Hz/0.5-depth raw samples gives −0.921422447 dB against original converted M5 (column 11) and −48.023990370 dB against the final off wet. All 108 fresh I1 stationary depth decisions and their neutral prerequisites match the frozen rows. All nine historical unsafe cases fail on fundamental or coloration and level. The registered motion evidence retains 700 active cells on each path. At 48 kHz acquisition still publishes at tick 5759 and rejects confirm/launch at the expiry tick 5760. The fractional-rate counterpart admits tick 3841 and rejects 3842. Exact withdrawal is a separate held condition. The frozen candidate still reaches zero on step 241 of a 240-step limit at 48 kHz and is one step late at four other rates. Maximum finite stored phase still reports MOMENT.
- **Signed wet and A4.** An independent pass over the frozen raw words found zero nominal product/cache mismatches and 17,099 in the half-product variant, starting on tick 480. Ten fresh signed A4 binary32 words match an independent calculation covering live negative gain, Character automation, cached fade from the intact peer, final signed-zero product and idle. The old joined producer has no emitted A4 telemetry. This checks the migrated offline harness without qualifying integrated delivery.

## Representative negative controls

| Existing behavior exercised | First rejection or measured disposition |
|---|---|
| O2 compiled denominator-sign edit | Tick 1, loaded tap `l`. Expected .06875532697275785, measured .011640511547882537. |
| Unsafe independent gain and always-neutral fourth path | `unsafe independent gain` and `constant delivery final wet`. The stronger Nyquist competitor fails `strongest fundamental`. |
| Half signed product and double cached energy | `bank joined sample column 10` and `conversion joined sample column 10`. |
| Acquisition expiry, slew and numerical faults | `elapsed acquisition eligibility` for the one-tick expiry bypass, `independent scheduled slew` for the bypassed slew and `epoch fault site=3 offset=0` for the aborting stereo variant. The exact-deadline failure in the frozen candidate remains visible separately. |
| Original M5 substitution and rolled wet with unchanged tags | `canonical Rust tap schema` and `synchronized wet producer samples`. The complete-entry reader mutants fail at `surviving substituted original M5` and `surviving rolled wet valid tags`. |
| Invalid evidence | Nonfinite JSON fails parsing. Strict JSON tests also reject duplicate keys and overflowing numbers. Coherent loss of provenance fields survives as described below. |

## Findings and minimum remediation

1. **V1-1 · Blocker · The canonical Stage 08 archive gate fails.** Compare `listening.zip` from `i1-study` with `build/validation-migration/reference/checkout/build/i1/listening.zip`. `unzip -Z1` plus per-member `cmp` isolates the only differing member to `listening-gains.json`. All fourteen WAVs match. `techs-P2-60-held` is `0.48111582544669923` versus frozen `0.4811158254466993`, one binary64 ULP. Restore exact historical metadata for this canonical artifact while continuing to report the freshly calculated gain, or obtain an explicit disposition for the representation difference before changing the gate. The Stage 08.2 listening ZIP is exact. The fresh member audit is in `build/review-v1/legacy-archive-audit.log`. [Submitted analysis of the gain difference](STAGE_V1_RUST_VALIDATION_MIGRATION.md#canonical-archive-comparison)
2. **V1-2 · Major · Controlled recording eligibility changed.** Copy the frozen synthetic cell, recode just `A1.wav` from IEEE float32 to IEEE float64 and update only its original-container hash in the manifest. Decoded, segment, capture, timing and role data are unchanged. The pinned `i1_recordings.inventory` accepted all eight takes. Rust `recordings build/review-v1/float64-recording/manifest.json` exited 1 with `A1: Ill-formed WAVE file: bits per sample is not 32`. The reproducer and receipts are `build/review-v1/recording-format-control.py`, `recording-format-legacy.log` and `probes/recordings/run-L7JTZC/recordings.json`. Preserve prior supported decoding and add this same-sample format comparison. [Rust decoder](../../../tools/validation-rs/src/recordings.rs)
3. **V1-3 · Major · Schema 3 inspection accepts missing provenance.** Copy a passed `fixtures` suite. Remove `inputs`, `rustc`, `target` and `profile` from its `provenance.json`, set the provenance digest in both `receipt.json` and `current.json` to the modified file's SHA-256 and run `inspect fixtures` against the copy. It returned 0 and `passed`. See `build/review-v1/provenance-control/inspect.log`. The reader checks selected identity fields and the artifact hash while the cross-reference consumers rely on its schema check. Require the declared provenance fields and validate the input inventory and run-local producer links at the consumption boundary. [Receipt reader](../../../tools/validation-rs/src/publication.rs)

## Authority and limits

Keep frozen Python and independent offline C++ sources in place at the reviewed entry revision. They serve historical comparison and replay, with no synchronization obligation for future Rust changes. Normal Rust qualification uses neither Python nor its dependencies. Preserve the existing native C++ boundaries and independent Rust math. Temporary Python research remains permissible. [Rust-first workflow](../AI_IMPLEMENTATION_GUIDE.md#permanent-validation-tooling)

S2a/S2b, M4/M6, O1, isolated O2 and injected baseline S4a retain their existing authorization. Isolated Rust I1 and full I1 tooling stay held on Review 8.2's signed delivery, exact withdrawal, layout ownership and strict evidence requirements. This review confirms the migrated half-product rejection and preserves the original finding against the legacy exit. Final A2 and live tracking/gesture, controlled source identity, owner listening, supported wet/S5, integrated headroom and release retain their separate holds. Real controlled takes, owner listening and new hosted Windows/Linux/native Intel results were unavailable. The next DSP increment remains the narrow offline transaction authorized by Review 8.2. [Current authorization table](REVIEW_08_2_I1_STREAMING_QUALIFICATION.md#final-authorization)
