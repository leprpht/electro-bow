# ElectroBow

ElectroBow is a JUCE-based VST3 audio plugin that detects the pitch of incoming audio and emits MIDI notes.

## Requirements

- CMake 3.22 or newer
- A JUCE source checkout (JUCE is not bundled with this repository)
- A C++17 compiler

On Windows, install Visual Studio with the **Desktop development with C++** workload. On macOS, install Xcode or the Xcode Command Line Tools. On Linux, install GCC or Clang and the usual build tools for your distribution.

## Build

Use the same Python command on Windows, macOS, and Linux. Pass the path to your JUCE checkout the first time:

```sh
python3 build.py "$HOME/path/to/JUCE"
```

On Windows, use `python` instead of `python3` if that is the command available on your system. The script configures the project automatically when needed and builds the `Release` configuration. Subsequent builds need no JUCE argument because the path is stored in the CMake build directory:

```sh
python3 build.py       # macOS/Linux
python build.py        # Windows
```

### Windows without Python

Python is not required on Windows. Double-click `build.bat`, or run it from Command Prompt. The script uses the folder containing the batch file as the project directory, so the project can be placed anywhere.

Before the first build, install CMake and Visual Studio with the **Desktop development with C++** workload, and make sure `cmake` is available on `PATH`. Then either:

- place the JUCE source checkout in a folder named `JUCE` beside this project; or
- set the JUCE path before building:

```bat
set JUCE_PATH=C:\path\to\JUCE
build.bat
```

`build.bat` runs `configure.bat` automatically when the `build/` directory has not been configured yet. To reconfigure manually, run `configure.bat` first. Both scripts build the `Release` configuration.

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

The generated VST3 plugin is placed under `build/ElectroBow_artefacts/Release/VST3/` (the exact bundle/file layout varies slightly by platform).

The release workflow builds the macOS plugin as a universal binary for both Apple Silicon (`arm64`) and Intel (`x86_64`) Macs. Local builds use all available CPU cores through CMake's parallel build mode.

## Tests

The test suite uses CTest and covers generated monophonic and polyphonic signals, including
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

On Windows, use `python run_tests.py`. The wrapper builds only the
`ElectroBowPitchDetectorTests` target before invoking CTest. Use `--reconfigure`
after changing CMake options, or `--no-build` to run an already-built test binary.

GitHub Actions runs the build and tests on every push and pull request for Windows, macOS, and Linux. Release builds run the same tests before packaging. CMake build directories are cached when the platform, JUCE version, and source inputs match.

## VS Code

Install the **CMake Tools** and **C/C++** extensions, then configure the project once so CMake generates `build/compile_commands.json`:

```sh
python3 build.py /path/to/JUCE
```

The repository includes VS Code settings that use this compilation database, providing the JUCE and aubio include paths for IntelliSense. If diagnostics remain after configuring, run **CMake: Delete Cache and Reconfigure** and reload the editor window.

Visual Studio is only used as the compiler/generator on Windows; no solution file is required or checked into the repository. CMake will select an appropriate native generator unless one is specified explicitly.

## Project layout

- `Source/PluginProcessor.*` and `Source/PluginEditor.*` contain the plugin implementation.
- `Source/PitchDetector.h` contains the aubio-based pitch detector.
- `ThirdParty/` contains the vendored aubio and STK sources used by the plugin.
- `CMakeLists.txt` defines the platform-independent build.

The vendored dependencies retain their upstream licenses in `ThirdParty/aubio-src/COPYING` and `ThirdParty/stk/LICENSE`.

## Development tools

Project C/C++ files are formatted automatically by the pre-commit hook. Vendored files under `ThirdParty/` are intentionally excluded.

Install LLVM, which provides `clang-format` and `clang-tidy`:

```sh
# macOS
brew install llvm
export PATH="$(brew --prefix llvm)/bin:$PATH"

# Ubuntu/Debian
sudo apt-get install clang-format clang-tidy
```

On Windows, install LLVM with `winget install LLVM.LLVM`, then make sure its `bin` directory is on `PATH`.

Enable the committed hook once per checkout:

```sh
git config core.hooksPath .githooks
```

Every commit will format staged project C/C++ files and stage the formatting changes. `clang-tidy` configuration is provided in `.clang-tidy`; run it from a configured CMake build directory with `compile_commands.json` when doing lint checks. Release builds do not run linting.

## Releases

Releases are built automatically by GitHub Actions for Windows, macOS, and Linux. The workflow runs when either a `v1.2.1` or `1.2.1` semantic-version tag is pushed, for example:

```sh
git tag v1.2.1
git push origin v1.2.1
```

The workflow file must be committed and pushed before creating the tag. It can also be started manually from the Actions tab; manual runs build the artifacts but do not publish a release.

The workflow builds and tests the plugin before packaging one VST3 archive per operating system. Release notes are created from the matching version section in `CHANGELOG.md`.

Keep the changelog in this strict structure:

```text
## [X.Y.Z] - YYYY-MM-DD

### Added

- Change description.
```

Allowed category headings are `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, and `Security`. The release workflow requires the exact version heading and date before publishing.
