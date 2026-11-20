"""Pass all Git push refs to checks run by the pre-commit framework.

The native framework handler selects only one ref and can skip tags pointing to
already-published commits. Run the configured pre-push stage unconditionally;
the changelog check uses Git's original stdin to select exactly the pushed tags.
"""

import os
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    with tempfile.TemporaryDirectory(prefix="electrobow-push-") as directory:
        refs = Path(directory) / "refs"
        refs.write_bytes(sys.stdin.buffer.read())
        env = dict(os.environ, ELECTROBOW_PUSH_REFS_FILE=str(refs))
        return subprocess.call(
            [sys.executable, "-m", "pre_commit", "run", "--hook-stage", "pre-push", "--all-files"],
            env=env,
        )


if __name__ == "__main__":
    sys.exit(main())
