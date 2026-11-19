# Rust DSP integration

The filter bank is migrated on `feature/filter-bank` (2026-11-19). JUCE continues
to host the plugin in C++; filtering, double-precision channel histories, and the
four float band buffers now live in the dependency-free `electrobow-dsp` crate.
The crossover equations, 200/800/3000 Hz defaults, Nyquist guard, denormal floor,
chunking, channel independence, and zero added latency are unchanged.

This is an incremental migration. Existing pitch/analyzer, envelope, trigger,
and STK code remains C++ and is still covered by its component tests. The active
plugin path remains the transparent filter bank. No new bow excitation or string
resonance behavior has been introduced. Future DSP in those areas should be
implemented in Rust, migrating each existing component with a parity test.

## Ownership and safety

- `rust/dsp/src/filter_bank.rs` implements DSP in safe Rust and forbids unsafe
  code at compile time. Preparation is the only allocation/coefficient stage.
- `rust/dsp/src/lib.rs` contains the small C ABI boundary. Its unsafe operations
  are explicit, documented pointer conversions; invalid counts/indices/nulls
  return errors before accessing audio. Raw non-null pointers still require
  the caller to supply valid lengths and lifetimes.
- `rust/dsp/include/electrobow_dsp.h` documents the shared ABI. It uses fixed-width
  integers and opaque pointers, without exposing Rust data layouts.
- `Source/FilterBank.h` is an owning C++ adapter. A custom `unique_ptr` deleter
  frees each handle through Rust exactly once. Copy/move is disabled. Reprepare
  constructs a replacement before replacing the old handle.
- Rust owns all allocations. C++ may borrow band pointers between calls, must
  reacquire them after split/reset, and must never free or resize them. The C ABI
  is serialized per instance; there are no locks or global mutable DSP states.
- Separate split/recombine calls preserve in-place processing of host buffers.
  I/O buffers must not alias Rust-owned band storage. The unchanged processor
  chunks blocks larger than the prepared maximum before crossing the boundary.
- Development and release library profiles use `panic = "abort"`; panics cannot
  unwind into C++. Checked invalid FFI arguments return status/null instead of
  panicking. A programming error or fatal allocator failure can still terminate
  the process. Rust unit-test harnesses use the usual test panic behavior.

The test allocator counts allocations, reallocations, and frees over 1,000
stereo split/recombine calls plus reset. The expected count is zero. C++ retains
no alternate DSP implementation in the production plugin.

## Build integration

`rust-toolchain.toml` pins Rust 1.100.0 with rustfmt and Clippy. Root `Cargo.toml`
and `Cargo.lock` define the workspace. The DSP crate builds a `staticlib` for
JUCE and an `rlib` for Rust tests. There are no third-party Cargo dependencies.
The static library format is intended for embedding in a non-Rust application;
its native system libraries must be supplied by the final linker. See the
[Rust linkage reference](https://doc.rust-lang.org/reference/linkage.html).

`cmake/RustDsp.cmake` invokes Cargo using `--locked --offline` and emits archives
under the selected CMake build directory, per target and configuration. Debug
maps to Cargo's dev profile, other configurations to release. Both the JUCE
plugin and its headless filter-bank test executable link the same Rust library.
CTest registers the Rust tests alongside the existing suites.

On macOS, explicit `CMAKE_OSX_ARCHITECTURES` values select matching Rust targets;
a universal build combines arm64 and x86_64 static archives with `lipo`. Windows
selects an MSVC target matching the CMake architecture. Linux builds the native
GNU target. Other cross-compilation modes are rejected rather than linking an
archive with the wrong architecture. Native link libraries were checked with
`cargo rustc -p electrobow-dsp --lib --release --target <triple> -- --print=native-static-libs`;
recheck these when changing the pinned compiler. macOS deployment targets are
forwarded to Cargo when explicitly configured in CMake.

Both CI workflows install the pinned toolchain and both macOS targets, check
Rust formatting/Clippy, and run the combined CTest suite before packaging. Setup
scripts check Rust prerequisites. Build/test wrappers set `CMAKE_BUILD_TYPE` so
single-configuration generators honor the requested build profile. The macOS
VST3 receives its final ad-hoc signature after JUCE writes `moduleinfo.json`,
then strict signature verification runs as part of the build.

Install [rustup](https://rustup.rs/) once, then from the checkout:

```sh
rustup show
rustup target add aarch64-apple-darwin x86_64-apple-darwin # macOS universal only
python3 build.py
python3 run_tests.py
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
```

The plugin needs no separate Rust runtime installation. Toolchain installation
requires network access; the crate builds offline after that.

## Migration validation

The original implementation from commit `fc0273a` is frozen in
`tests/reference/CppFilterBankReference.h`, used only by the test target. This is
an independent pre-migration reference, not a second production implementation.
The parity test compares all four bands and their sums, mono/stereo, nine sample
rates from 4 to 384 kHz, changing block lengths, and reset/replay. Existing complex
frequency-response, impulse, transient, gain, stereo isolation, oversized-block,
reprepare and editor smoke tests remain in use.

Local native arm64 Release measurements, Apple M2 Pro / Apple Clang 17 / Rust
1.100.0:

| Measurement | Original C++ | Rust via C ABI |
| --- | ---: | ---: |
| Peak noise reconstruction error | 5.96046448e-8 FS | 5.96046448e-8 FS |
| Worst noise null, 4–384 kHz | -153.250729 dBr | -153.250729 dBr |
| Worst summed response deviation | 1.85718644e-7 dB | 1.85718644e-7 dB |
| Worst summed phase deviation | 1.39085960e-6 degrees | 1.39085960e-6 degrees |
| Added algorithmic latency | 0 samples | 0 samples |
| CPU, stereo 48 kHz / 64 frames | 0.0432% of one core | 0.0426% of one core |

The direct local parity test reported **0 maximum band difference and 0 maximum
output difference**. Tests allow `2e-7` across compilers/platforms; bit identity
is a local measurement, not a cross-platform guarantee. CPU uses the existing
median-of-seven microbenchmark and is subject to machine load, excluding host
scheduling, audio device, and editor costs. Tiny timing differences do not imply
a performance advantage for either language.

Reproduce the parity/measurement output with:

```sh
ctest --test-dir build --build-config Release -R ElectroBowFilterBankTests --verbose
```

Build and test results before committing:

- `cargo test --locked --offline`: four Rust tests passed in the dev/test profile.
- `cargo fmt --all -- --check` and Clippy with `-D warnings`: passed.
- `python3 build.py --config Release`: native macOS arm64 Release VST3 built.
- CMake Release with `CMAKE_OSX_ARCHITECTURES=arm64;x86_64`: full universal plugin
  built; `lipo` confirms both slices in the Rust archive and final VST3 binary.
- All four CTest suites passed for native and universal Release builds: Rust
  unit tests, filter-bank integration/parity, legacy DSP components, and editor
  open/reopen. Universal CTest ran on the native arm64 host.
- The universal filter-bank test also ran as x86_64 under Rosetta and passed,
  reporting zero C++/Rust band/output differences there as well.
- Xcode multi-configuration generator built `ElectroBowRustBuild` in both Debug
  and Release, verifying separate dev/release archive selection.
- Final macOS bundle signatures passed strict verification during each build.
- Rust static libraries cross-compiled for `x86_64-pc-windows-msvc` and
  `x86_64-unknown-linux-gnu`; their required native link libraries were inspected.
  Full Windows/Linux JUCE builds were not executed locally. The updated CI and
  release workflows run those platform builds and tests when pushed.

The frozen C++ baseline and parity test remain committed so later Rust DSP
changes can be checked against the original behavior. New modules should keep
unsafe code confined to the ABI boundary and add component/real-time tests
before replacing another C++ signal-path component.
