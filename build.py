#!/usr/bin/env python3
"""Configure and build ElectroBow on any platform with Python and CMake."""

from pathlib import Path
import argparse
import os
import subprocess
from typing import Optional


PROJECT_DIR = Path(__file__).resolve().parent
UI_DIR = PROJECT_DIR / "UI"


def has_build_files(build_dir: Path) -> bool:
    generated_files = (
        [build_dir / "Makefile", build_dir / "build.ninja"]
        + list(build_dir.glob("*.sln"))
        + list(build_dir.glob("*.xcodeproj"))
        + list(build_dir.glob("*.vcxproj"))
    )
    return any(path.exists() for path in generated_files)


def cached_juce_path(cache_file: Path) -> Optional[Path]:
    if not cache_file.is_file():
        return None

    prefix = "JUCE_PATH:PATH="
    for line in cache_file.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith(prefix):
            return Path(line[len(prefix) :]).expanduser().resolve()

    return None


def cached_cmake_arg(cache_file: Path, arg: str) -> bool:
    """Return whether a requested -DNAME=value is already in the cache."""
    name, _, value = arg.removeprefix("-D").partition("=")
    if not name or not value:
        return True
    prefix = f"{name}:"
    return any(
        line.startswith(prefix) and line.split("=", 1)[-1] == value
        for line in cache_file.read_text(encoding="utf-8", errors="replace").splitlines()
    )


def prepare_ui() -> None:
    """Install and build the embedded React frontend before CMake configures."""
    if not (UI_DIR / "package.json").is_file():
        raise RuntimeError(f"UI package.json was not found at {UI_DIR}")

    npm = "npm.cmd" if os.name == "nt" else "npm"
    if not (UI_DIR / "node_modules").is_dir():
        subprocess.run([npm, "ci"], cwd=UI_DIR, check=True)

    subprocess.run([npm, "run", "build"], cwd=UI_DIR, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("juce_path", nargs="?", help="Path to the JUCE source tree")
    parser.add_argument("--build-dir", default="build", help="CMake build directory")
    parser.add_argument("--config", default="Release", help="Build configuration")
    parser.add_argument(
        "--cmake-arg",
        action="append",
        default=[],
        help="Additional CMake cache entry, without the -D prefix",
    )
    args = parser.parse_args()

    build_dir = Path(args.build_dir)
    if not build_dir.is_absolute():
        build_dir = PROJECT_DIR / build_dir

    juce_path = args.juce_path or os.environ.get("JUCE_PATH")
    local_juce = PROJECT_DIR / "JUCE"
    if not juce_path and (local_juce / "CMakeLists.txt").is_file():
        juce_path = str(local_juce)

    if not juce_path:
        parser.error("provide JUCE_PATH or pass the JUCE path as the first argument")

    juce_path_obj = Path(juce_path).expanduser()
    if not juce_path_obj.is_absolute():
        juce_path_obj = Path.cwd() / juce_path_obj
    juce_path_obj = juce_path_obj.resolve()

    if not (juce_path_obj / "CMakeLists.txt").is_file():
        parser.error(f"JUCE CMakeLists.txt was not found at {juce_path_obj}")

    try:
        prepare_ui()
    except (OSError, RuntimeError) as error:
        parser.error(f"frontend preparation failed: {error}")

    cache_file = build_dir / "CMakeCache.txt"
    needs_configure = (
        not cache_file.is_file()
        or not has_build_files(build_dir)
        or cached_juce_path(cache_file) != juce_path_obj
        or any(not cached_cmake_arg(cache_file, arg) for arg in args.cmake_arg)
    )

    if needs_configure:
        subprocess.run(
            [
                "cmake",
                "-S",
                str(PROJECT_DIR),
                "-B",
                str(build_dir),
                f"-DJUCE_PATH={juce_path_obj}",
                *[arg if arg.startswith("-D") else f"-D{arg}" for arg in args.cmake_arg],
            ],
            check=True,
        )

    subprocess.run(
        ["cmake", "--build", str(build_dir), "--config", args.config, "--parallel"],
        check=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
