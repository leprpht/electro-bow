"""Install pre-commit hooks and preserve all refs for pre-push checks."""

from pathlib import Path
import subprocess
import sys


def main():
    root = Path(__file__).resolve().parents[1]
    # Verify the framework is available before disabling the old hook path.
    subprocess.run([sys.executable, "-m", "pre_commit", "validate-config"], cwd=root, check=True)
    old_path = subprocess.run(
        ["git", "config", "--local", "--get", "core.hooksPath"],
        cwd=root, capture_output=True, text=True, check=False,
    ).stdout.strip()
    if old_path == ".githooks":
        subprocess.run(["git", "config", "--local", "--unset", "core.hooksPath"], cwd=root, check=True)
    subprocess.run([sys.executable, "-m", "pre_commit", "install"], cwd=root, check=True)

    hook_path = subprocess.check_output(
        ["git", "rev-parse", "--git-path", "hooks/pre-push"], cwd=root, text=True,
    ).strip()
    hook = root / hook_path
    generated = hook.read_text()
    original = 'exec "$INSTALL_PYTHON" -mpre_commit "${ARGS[@]}"'
    if original not in generated:
        raise RuntimeError("Unexpected pre-commit hook template; check requirements-dev.txt.")
    # Keep the framework's generated launcher and uninstall marker. Only adapt
    # dispatch so the full push ref list reaches the framework-managed check.
    generated = generated.replace(original, 'exec "$INSTALL_PYTHON" tools/hooks/pre_push.py "$@"')
    generated = generated.replace(
        'exec pre-commit "${ARGS[@]}"',
        'echo "Hook Python environment is missing. Reinstall with python tools/install_hooks.py." >&2\n    exit 1',
    )
    hook.write_text(generated, newline="\n")
    print("Installed pre-push ref forwarding for tag validation.")


if __name__ == "__main__":
    main()
