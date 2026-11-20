"""Run project formatters on the filenames supplied by pre-commit."""

from pathlib import Path
import shutil
import subprocess
import sys


def executable(name):
    found = shutil.which(name)
    if found:
        return found
    candidates = {
        "clang-format": [
            Path("/opt/homebrew/opt/llvm/bin/clang-format"),
            Path("/usr/local/opt/llvm/bin/clang-format"),
        ],
        "rustfmt": [Path.home() / ".cargo/bin" / ("rustfmt.exe" if sys.platform == "win32" else "rustfmt")],
    }
    for candidate in candidates.get(name, []):
        if candidate.is_file():
            return str(candidate)
    raise RuntimeError(f"{name} was not found. See README.md for development tool setup.")


def main():
    formatter, *filenames = sys.argv[1:]
    if not filenames:
        return 0
    if formatter == "prettier":
        ui = Path(__file__).resolve().parents[2] / "UI"
        prettier = ui / "node_modules/prettier/bin/prettier.cjs"
        if not prettier.is_file():
            raise RuntimeError("Prettier was not found. Run 'npm ci --prefix UI'.")
        command = [executable("node"), str(prettier), "--write"]
        return subprocess.call(command + [str(Path(f).resolve()) for f in filenames], cwd=ui)
    if formatter == "clang-format":
        command = [executable(formatter), "-i", "--"]
    elif formatter == "rustfmt":
        # Format only supplied files; pre-commit owns staged-file selection.
        command = [executable(formatter), "--edition", "2024", "--config", "skip_children=true"]
    else:
        raise RuntimeError(f"Unknown formatter: {formatter}")
    return subprocess.call(command + filenames)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, RuntimeError) as error:
        print(f"format: {error}", file=sys.stderr)
        sys.exit(1)
