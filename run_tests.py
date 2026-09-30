#!/usr/bin/env python3
"""Configure, build, and run ElectroBow's C++ tests.

Examples:
    python3 run_tests.py /path/to/JUCE
    python3 run_tests.py --build-dir build-debug --config Debug
    python3 run_tests.py --no-build --verbose
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import sys
from typing import Optional


PROJECT_DIR = Path(__file__).resolve().parent
TEST_TARGET = "ElectroBowPitchDetectorTests"


def run(command: list[str], *, quiet: bool = False) -> None:
    print("+", " ".join(command))
    subprocess.run(command, cwd=PROJECT_DIR, check=True, stdout=None if not quiet else subprocess.DEVNULL)


def cached_value(cache_file: Path, name: str) -> Optional[str]:
    if not cache_file.is_file():
        return None

    prefix = f"{name}:"
    for line in cache_file.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith(prefix) and "=" in line:
            return line.split("=", 1)[1]
    return None


def has_cmake_cache(build_dir: Path) -> bool:
    return (build_dir / "CMakeCache.txt").is_file()


def resolve_build_dir(value: str) -> Path:
    build_dir = Path(value).expanduser()
    return build_dir if build_dir.is_absolute() else PROJECT_DIR / build_dir


def resolve_juce_path(explicit: Optional[str], build_dir: Path) -> Optional[Path]:
    value = explicit or os.environ.get("JUCE_PATH")
    if not value:
        cached = cached_value(build_dir / "CMakeCache.txt", "JUCE_PATH")
        value = cached
    if not value:
        local = PROJECT_DIR / "JUCE"
        if (local / "CMakeLists.txt").is_file():
            value = str(local)
    if not value:
        return None

    path = Path(value).expanduser()
    if not path.is_absolute():
        path = Path.cwd() / path
    return path.resolve()


def cmake_args(values: list[str]) -> list[str]:
    return [value if value.startswith("-D") else f"-D{value}" for value in values]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("juce_path", nargs="?", help="Path to the JUCE source tree")
    parser.add_argument("--build-dir", default="build", help="CMake build directory")
    parser.add_argument("--config", default="Release", help="Build configuration")
    parser.add_argument("--generator", help="CMake generator, for example Ninja")
    parser.add_argument(
        "--cmake-arg",
        action="append",
        default=[],
        help="Additional CMake cache entry, with or without the -D prefix",
    )
    parser.add_argument("--reconfigure", action="store_true", help="Run CMake configure first")
    parser.add_argument("--no-build", action="store_true", help="Run existing built tests only")
    parser.add_argument("--verbose", action="store_true", help="Show CTest progress and output")
    parser.add_argument("--list", action="store_true", help="List tests without running them")
    args = parser.parse_args()

    build_dir = resolve_build_dir(args.build_dir)
    cache_file = build_dir / "CMakeCache.txt"

    if not args.no_build and (args.reconfigure or not has_cmake_cache(build_dir)):
        juce_path = resolve_juce_path(args.juce_path, build_dir)
        if juce_path is None:
            parser.error(
                "JUCE was not found; pass its path as the first argument, set JUCE_PATH, "
                "or configure this build directory first"
            )
        if not (juce_path / "CMakeLists.txt").is_file():
            parser.error(f"JUCE CMakeLists.txt was not found at {juce_path}")

        configure = [
            "cmake",
            "-S",
            str(PROJECT_DIR),
            "-B",
            str(build_dir),
            f"-DJUCE_PATH={juce_path}",
            "-DBUILD_TESTING=ON",
            *cmake_args(args.cmake_arg),
        ]
        if args.generator:
            configure[1:1] = ["-G", args.generator]
        run(configure)

    if not has_cmake_cache(build_dir):
        parser.error(f"{build_dir} is not configured; remove --no-build or configure it first")

    if not args.no_build and not args.list:
        run(
            [
                "cmake",
                "--build",
                str(build_dir),
                "--config",
                args.config,
                "--target",
                TEST_TARGET,
                "--parallel",
            ]
        )

    ctest = [
        "ctest",
        "--test-dir",
        str(build_dir),
        "--build-config",
        args.config,
        "--output-on-failure",
    ]
    if args.list:
        ctest.append("-N")
    elif args.verbose:
        ctest.append("--verbose")
    run(ctest)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        print(f"Test command failed with exit code {error.returncode}.", file=sys.stderr)
        raise SystemExit(error.returncode)
