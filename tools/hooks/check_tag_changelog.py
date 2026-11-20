"""Validate every pushed tag using the changelog in its tagged commit."""

import argparse
import os
from pathlib import Path
import subprocess
import sys


def git(*args):
    return subprocess.check_output(["git", *args], text=True, encoding="utf-8", stderr=subprocess.PIPE)


def pushed_tags(filename):
    for line in Path(filename).read_text(encoding="utf-8").splitlines():
        _local_ref, local_sha, remote_ref, _remote_sha = line.split()
        if remote_ref.startswith("refs/tags/") and set(local_sha) != {"0"}:
            yield remote_ref, local_sha


def check_tag(ref, revision):
    tag = ref.removeprefix("refs/tags/")
    version = tag.removeprefix("v")
    try:
        commit = git("rev-parse", "--verify", f"{revision}^{{commit}}").strip()
        changelog = git("show", f"{commit}:CHANGELOG.md")
    except subprocess.CalledProcessError:
        print(f"tag-changelog: tag '{tag}' must point to a commit containing CHANGELOG.md.", file=sys.stderr)
        return False
    heading = f"## [{version}]"
    if not any(line == heading or line.startswith(heading + " - ") for line in changelog.splitlines()):
        print(f"tag-changelog: CHANGELOG.md in tag '{tag}' has no '{heading}' entry.", file=sys.stderr)
        return False
    print(f"tag-changelog: verified '{tag}' in commit {commit}.")
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", action="append", help="Check a specific local tag (repeatable).")
    args = parser.parse_args()
    push_refs = os.environ.get("ELECTROBOW_PUSH_REFS_FILE")
    if args.tag:
        tags = [(tag, f"refs/tags/{tag.removeprefix('refs/tags/')}") for tag in args.tag]
    elif push_refs:
        tags = list(pushed_tags(push_refs))
    else:
        parser.error("Use the installed pre-push hook, or specify --tag for a manual check.")
    # Evaluate every tag to report all failures in one run.
    return int(not all([check_tag(tag, revision) for tag, revision in tags]))


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError):
        print("tag-changelog: unable to read pushed refs or local Git objects.", file=sys.stderr)
        sys.exit(1)
