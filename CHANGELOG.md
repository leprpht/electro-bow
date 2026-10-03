# Changelog

All notable changes to ElectroBow are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.4] - 2026-10-03

### Added

- Added continuous input-dynamics tracking for more musical intensity throughout each note.

### Changed

- Applied the continuous input envelope to bow excitation and voice rendering while keeping attack detection separate.
- Reformatted pitch-detector tests for consistency.

## [1.3.3] - 2026-10-03

### Changed

- Added per-voice pitch candidate, confidence, stability, and confirmed-pitch tracking.
- Preserved active voices through short unreliable analysis gaps with voice-local grace state.
- Improved harmonic continuity handling for temporary octave and partial detections.

### Fixed

- Prevented transient fundamental/harmonic/fundamental observations from retriggering one physical pluck.
- Prevented weak one-frame spectral residues from creating unwanted voices.

## [1.3.2] - 2026-10-03

### Changed

- Improved voice continuity during unreliable spectral analysis frames.
- Added per-voice pitch smoothing and safer releasing-voice reuse.
- Applied input dynamics continuously to bow excitation and voice envelopes.

### Fixed

- Prevented pitch-analysis dropouts and physical release transitions from causing unwanted retriggers.

## [1.3.1] - 2026-10-02

### Fixed

- Fixed release archives so the Windows `ElectroBow.vst3` package is placed at the archive root and contains its platform binary.
- Added release validation to prevent publishing incomplete VST3 artifacts.

### Changed

- Documented VST3 installation in FL Studio and clarified that releases do not include a VST2 `.dll`.

## [1.3.0] - 2026-10-02

### Added

- Added an embedded React/Tailwind user interface for monitoring pitch, tracking confidence, active voices, and plugin parameters.
- Added a JUCE WebView bridge for synchronizing plugin state and parameter changes with the frontend.

### Changed

- Improved polyphonic voice tracking stability and preserved input dynamics during analysis dropouts.
- Updated cross-platform CI and release builds to generate the frontend bundle and package the embedded UI.

## [1.2.3] - 2026-10-02

### Fixed

- Improved voice stability during polyphonic pitch analysis.
- Prevented weak or noisy FFT frames from clearing all currently detected voices.
- Improved voice continuity during temporary pitch-analysis dropouts.
- Prevented YIN pitch-tracking confidence from affecting voice volume.
- Kept voice strength based on the spectral strength estimate instead of pitch-tracker confidence.
- Improved pitch correction stability by limiting YIN corrections to a reasonable pitch range, reducing unwanted octave and harmonic jumps.

## [1.2.2] - 2026-10-01

### Changed

- Improved realtime bow tracking and physical attack detection.
- Improved voice management and analysis-generation handling for more stable realtime processing.

### Fixed

- Fixed realtime stability issues in bow-trigger and voice-processing behavior.

## [1.2.1] - 2026-10-01

### Changed

- Improved pitch detection and voice processing with more reliable analysis and reset behavior.
- Replaced note release decay with configurable natural resonance for more natural bowed-string behavior.

### Fixed

- Fixed natural resonance and note release behavior.

## [1.2.0] - 2026-10-01

### Added

- Added isolated polyphonic voice tracking with stable analyzer voice IDs.
- Added spectral analysis for improved tracking of polyphonic input.
- Added a Python test wrapper for configuring, building, and running the test suite.
- Added tests for isolated voice buffers, guitar-like harmonics, weak fundamentals, continuous pitch bends, and noise rejection.

### Changed

- Improved polyphonic pitch detection and voice matching when tracked frequencies cross.
- Improved resource management for isolated audio input analysis.
- Improved Windows build and configuration workflows, including operation without Python.

## [1.1.0] - 2026-09-30

### Added

- Added polyphonic pitch detection for chord input.
- Added bowed-string voice synthesis with configurable attack and release envelopes.
- Added bow-trigger processing for note release detection.
- Added cross-platform automated builds and pitch-detector tests for Windows, macOS, and Linux.
- Added CMake, Python, and batch build helpers for local development.
- Added clang-format and clang-tidy configuration, a formatting pre-commit hook, and VS Code settings.

### Changed

- Improved pitch detection and plugin processing for polyphonic input.
- macOS CI and release builds now produce universal Apple Silicon and Intel binaries.
- CI and release builds now use parallel compilation and compiler caching.

### Fixed

- Fixed Linux test linking and added the JUCE dependencies required by Linux builds.

## [1.0.3] - 2026-09-28

### Fixed

- Fixed cross-platform release packaging in the GitHub Actions workflow.
- Added the Linux fontconfig build dependency required by JUCE.
- Added the Linux XInput dependency required by JUCE.

### Changed

- Enabled parallel builds in the cross-platform build script.
- macOS release artifacts now include both Intel and Apple Silicon architectures.

## [1.0.0] - 2026-09-28

### Added

- Initial public release of the ElectroBow VST3 plugin.
- Cross-platform CMake build for Windows, macOS, and Linux.
