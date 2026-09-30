# Changelog

All notable changes to ElectroBow are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
