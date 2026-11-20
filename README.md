# ElectroBow

ElectroBow is a JUCE-based VST3 audio plugin. Audio path is a transparent
four-band parallel IIR filter bank at 200, 800, and 3000 Hz, supporting mono and
stereo with zero added algorithmic latency. A continuous Rust attack shaper now softens upper-band transients before
recombination; Attack at minimum restores unity reconstruction. No bow processing runs. Existing bow controls/session data are
retained, Attack and Bow Character now affect audio; pitch/voice displays stay idle.

See [the Phase 1 report](docs/phase1-filter-bank.md) for the filter design,
reconstruction measurements, CPU results, Release build, and modified files.

## Requirements

- CMake 3.22 or newer
- Python 3.10 or newer (build helper and development hooks)
- A JUCE source checkout (JUCE is not bundled with this repository)
- A C++17 compiler
- Rust via [rustup](https://rustup.rs/); `rust-toolchain.toml` pins the compiler,
  rustfmt, and Clippy versions
- Node.js 20 or newer and npm (used to build the embedded React UI)

On Windows, install Visual Studio with the **Desktop development with C++** workload. On macOS, install Xcode or the Xcode Command Line Tools. On Linux, install GCC or Clang and the usual build tools for your distribution.

## Build

To prepare a new checkout, install the required developer dependencies and enable
the repository hooks with the setup script. Install rustup first and ensure
Cargo is on your PATH (`. "$HOME/.cargo/env"` on macOS/Linux after installation).
Run `rustup show` from this checkout to install the pinned toolchain:

```sh
rustup show
```

Then run:

```sh
./setup.sh                 # macOS/Linux
setup.bat                  # Windows Command Prompt
```

Install the [pre-commit framework](https://pre-commit.com/) as described under
[Development tools](#development-tools) before running setup.

The scripts verify Git, Node.js/npm, Python, pre-commit, CMake, a C++ compiler, Rust, and
`clang-format`; download JUCE 9.0.2 into `JUCE/` when needed; install the UI
dependencies with `npm ci`; and configure the local Git hooks. They do not pull
or modify repository history. Install platform-specific compiler packages and
the Windows WebView2 SDK separately when your build requires them.

Use the same Python command on Windows, macOS, and Linux. Pass the path to your JUCE checkout the first time:

```sh
python3 build.py "$HOME/path/to/JUCE"
```

The first build installs the frontend dependencies from `UI/package-lock.json` and
builds the TypeScript/React/Tailwind bundle before CMake embeds it in the plugin.
If you configure with CMake directly, run `npm ci && npm run build` from `UI/`
first.

On Windows, use `python` instead of `python3` if that is the command available on your system. The script configures the project automatically when needed and builds the `Release` configuration. Subsequent builds need no JUCE argument because the path is stored in the CMake build directory:

```sh
python3 build.py       # macOS/Linux
python build.py        # Windows
```

If a JUCE checkout is placed in a `JUCE/` folder beside the project, `build.py` detects it automatically. You can also set `JUCE_PATH` once instead of passing it as an argument:

```sh
export JUCE_PATH=/path/to/JUCE       # macOS/Linux
set JUCE_PATH=C:\path\to\JUCE       # Windows Command Prompt
```

For direct CMake use, the equivalent commands are:

```sh
cmake -S . -B build -DJUCE_PATH=/path/to/JUCE
cmake --build build --config Release
```

CMake builds the Rust DSP static library automatically with Cargo, then links it
into the plugin. There is no Rust runtime DLL to install. Debug configurations
use Cargo's dev profile; other configurations use its release profile. The DSP
crate has no third-party dependencies, and Cargo builds use `--locked --offline`
after the toolchain/target components are installed.

For universal macOS builds, install both Rust targets first:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
python3 build.py --cmake-arg "CMAKE_OSX_ARCHITECTURES=arm64;x86_64"
```

CMake builds each Rust architecture and combines the archives with `lipo`.
Windows uses MSVC; Linux uses the native GNU target. The final macOS bundle is
ad-hoc signed and verified after manifest generation.

The generated VST3 plugin is placed under `build/ElectroBow_artefacts/Release/VST3/` (the exact bundle/file layout varies slightly by platform).
The Windows release is a VST3 package, not a VST2 `.dll`: after extracting
`ElectroBow-vst3-windows.zip`, copy the included `ElectroBow.vst3` folder to a
VST3 scan location such as `C:\Program Files\Common Files\VST3`, then rescan
plugins in the DAW.

The Windows UI requires the [Microsoft Edge WebView2 Evergreen Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
Install or repair it and restart the DAW if ElectroBow reports that WebView2 could
not start. The WebView2 loader is linked into the plugin, so no separate
`WebView2Loader.dll` is needed. Browser data is stored in
`%LOCALAPPDATA%\ElectroBow\WebView2`, which must be writable by the current user.
macOS uses the system WebKit browser and does not require WebView2.

The release workflow builds the macOS plugin as a universal binary for both Apple Silicon (`arm64`) and Intel (`x86_64`) Macs. Local builds use all available CPU cores through CMake's parallel build mode.

## Tests

CTest runs the Rust unit tests as well as the C++ integration suites. Rust tests
check FFI validation, lifecycle behavior, and zero heap operations during DSP.
The integration suite compares all Rust band samples against a frozen C++ test
reference and covers filter-bank reconstruction, phase/response,
transients, stereo isolation, variable buffers, and processor integration. Retained
component tests also cover generated monophonic and polyphonic signals, including
isolated voice buffers, guitar-like harmonics, weak fundamentals, continuous pitch bends, and
noise rejection:

```sh
ctest --test-dir build --build-config Release --output-on-failure
```

For a convenient configure/build/run workflow, use the Python test wrapper:

```sh
python3 run_tests.py /path/to/JUCE
python3 run_tests.py                 # reuse the configured build directory
python3 run_tests.py --verbose       # show detailed CTest output
python3 run_tests.py --list          # list tests without running them
```

On Windows, use `python run_tests.py`. The wrapper builds the filter-bank and pitch detector tests
and, on Windows/macOS, `ElectroBowEditorSmokeTests` before invoking CTest. The
editor test requires a desktop session and opens the production editor twice,
checking that the embedded React UI, CSS and native state bridge load correctly.
Use `--reconfigure`
after changing CMake options, or `--no-build` to run an already-built test binary.

Standalone Rust checks are also available:

```sh
cargo test --locked --offline
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
```

GitHub Actions runs the build and tests on every push and pull request for Windows, macOS, and Linux. Release builds run the same tests before packaging. CMake build directories are cached when the platform, JUCE version, and source inputs match.

## VS Code

Install the **CMake Tools** and **C/C++** extensions, then configure the project once so CMake generates `build/compile_commands.json`:

```sh
python3 build.py /path/to/JUCE
```

The repository includes VS Code settings that use this compilation database, providing the JUCE and aubio include paths for IntelliSense. If diagnostics remain after configuring, run **CMake: Delete Cache and Reconfigure** and reload the editor window.

Visual Studio is only used as the compiler/generator on Windows; no solution file is required or checked into the repository. CMake will select an appropriate native generator unless one is specified explicitly.

## Project layout

- `Source/PluginProcessor.*` contains the audio/business logic and the small native UI bridge.
- `Source/PluginEditor.*` hosts JUCE's WebView and serves the embedded frontend assets.
- `UI/` contains the TypeScript/React/Tailwind frontend. Its production bundle is generated in
  `UI/dist/` and embedded into the VST3 during the CMake build.
- `rust/dsp/` owns the filter-bank algorithm, channel state, and band buffers.
- `Source/FilterBank.h` is the C++ ownership adapter for the Rust DSP C API.
- `cmake/RustDsp.cmake` integrates Cargo, native linking, and universal builds.
- `Source/PitchDetector.h` contains the retained aubio-based pitch detector.
- `ThirdParty/` contains the vendored aubio and STK sources used by the plugin.
- `CMakeLists.txt` defines the platform-independent build.

The vendored dependencies retain their upstream licenses in `ThirdParty/aubio-src/COPYING` and `ThirdParty/stk/LICENSE`.

New DSP work (filters, tracking, bow excitation, and string resonance) should
prefer Rust modules in `rust/dsp/`. JUCE hosting and the UI bridge stay in C++.
Existing aubio/STK and older C++ components remain available for incremental
migration. See [Rust DSP architecture and migration validation](docs/rust-dsp.md).

## Instrument interface

The 960 × 700 editor uses a vintage metal faceplate, reusable rotary knobs,
and an analog tuner. Attack Reduction and the smaller Bow Character control
operate the existing DSP. Drag vertically, hold Shift for finer changes, use
arrow/Home/End keys, or double-click to restore the default. Values appear on
hover or focus. Sustain, Resonance, Bow Pressure, Bow Speed, and Bow Friction
are disabled because they are not implemented in the current audio path.
The tuner shows **No signal** while pitch telemetry is inactive.

The React components live in `UI/src/components/`; native state synchronization
lives in `UI/src/hooks/usePluginState.ts`. Tailwind handles layout utilities.
Typed component styles live in `UI/src/styles/*.css.ts`: panel, knob, tuner,
screw, and signature modules keep materials, responsive rules, and interaction
states together. Vanilla Extract compiles these modules into the existing
`app.css` bundle; no styling runtime or extra native asset route is needed.
Edit `UI/src/styles/theme.ts` for the shared breakpoint, panel aspect ratio,
texture, and knob geometry. Screw positions and variants are inferred TypeScript
types. `UI/src/styles.css` contains only Tailwind setup, fonts, global defaults,
and reduced-motion behavior.
Fonts and the SVG texture are bundled into CSS for offline JUCE rendering.

Preview and browser interaction tests:

```sh
cd UI
npm ci
npm run dev
# In a separate terminal in UI/:
npx playwright install chromium webkit
npm test
npm run format:check
```

The browser suite builds and serves the production bundle, tests Chromium and
WebKit, and writes screenshots under the ignored `UI/test-results/` directory.
The native editor smoke test checks embedded fonts, layout, state loading,
and a React control edit reaching the processor on both open and reopen.

## Development tools

The [pre-commit framework](https://pre-commit.com/) manages both Git hook stages
through `.pre-commit-config.yaml`. Before commits it formats staged C/C++ files
with clang-format, Rust files with the pinned rustfmt, and UI files with the
Prettier installed from `UI/package-lock.json`. Vendored files under
`ThirdParty/` are excluded. Install Rust's format component with
`rustup component add rustfmt` and UI dependencies with `npm ci --prefix UI`.

Install LLVM, which provides `clang-format` and `clang-tidy`:

```sh
# macOS
brew install llvm
export PATH="$(brew --prefix llvm)/bin:$PATH"

# Ubuntu/Debian
sudo apt-get install clang-format clang-tidy
```

On Windows, install LLVM with `winget install LLVM.LLVM`, then make sure its `bin` directory is on `PATH`.

Install the framework in a local virtual environment and activate it:

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements-dev.txt
```

On Windows Command Prompt, use `python -m venv .venv`, then
`.venv\Scripts\activate.bat` and the same pip command.

Run the setup script or install both hooks from the repository root:

```sh
python tools/install_hooks.py
```

The installer removes the old local `core.hooksPath=.githooks` setting when
present, then installs the framework hooks. Other custom hook paths are left
for pre-commit to report. Keep `.venv` available: generated hooks use its Python.

The installer adapts the framework's generated pre-push launcher to forward
Git's complete ref list through `tools/hooks/pre_push.py`, which runs the
configured pre-push stage using `pre-commit run`. This preserves multi-tag pushes
and tag-only pushes to commits already on the remote; the framework's native
push handler can skip those cases. Use this installer again after changing the
framework version, rather than running `pre-commit install` directly.

When a formatter changes files, the commit stops for review. Stage the resulting
changes and commit again. Partially staged files are handled by the framework;
formatting changes are never automatically staged. To format all tracked files:

```sh
pre-commit run --all-files
```

`clang-tidy` configuration is provided in `.clang-tidy`; run it from a configured
CMake build directory with `compile_commands.json` when doing lint checks.
Release builds do not run linting.

To test hook installation, staged formatting, and tag pushes against disposable
local repositories (no project remote is contacted):

```sh
python -m unittest discover -s tools/hooks/tests -v
```

## Releases

Releases are built automatically by GitHub Actions for Windows, macOS, and Linux. The workflow runs when either a `v1.5.0` or `1.5.0` semantic-version tag is pushed.

Before tagging, update the version in `CMakeLists.txt`, `UI/package.json`, and
the root package entries in `UI/package-lock.json`. Add a dated changelog entry
for the same version and commit these changes. Then create and push an annotated
tag, for example:

```sh
git tag -a v1.5.0 -m "Release v1.5.0"
git push --atomic origin main v1.5.0
```

The pre-push hook checks every tag being created or updated against the
changelog in its tagged commit. It supports annotated and lightweight tags,
multiple tags in a push, and tags pointing to commits already on the remote.
Branch pushes and tag deletions do not require changelog entries, and unrelated
local tags are ignored. Check a specific tag locally with:

```sh
python tools/hooks/check_tag_changelog.py --tag v1.5.0
```

The check reads the changelog from the tagged commit, not your working copy.
The workflow file must also be present in that commit. The workflow can
be started manually from the Actions tab; manual runs build the artifacts but
do not publish a release.

The workflow builds and tests the plugin before packaging one VST3 archive per operating system. Each archive has `ElectroBow.vst3` at its root; on Windows the plugin binary is inside that package at `Contents/x86_64-win/ElectroBow.vst3`. Release notes are created from the matching version section in `CHANGELOG.md`.

Keep the changelog in this strict structure:

```text
## [X.Y.Z] - YYYY-MM-DD

### Added

- Change description.
```

Allowed category headings are `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, and `Security`. The release workflow requires the exact version heading and date before publishing.

See [attack processing](docs/attack-processing.md) for the algorithm and validation.

See [Bow Character architecture and audio comparisons](docs/bow-character.md).

The [final integration report](docs/final-effect.md) documents output protection,
final controls, dependency audit, and the pending FL Studio validation.
