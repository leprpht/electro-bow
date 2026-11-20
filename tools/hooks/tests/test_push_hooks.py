"""Exercise installed framework hooks against disposable local Git remotes.

Run with: python -m unittest discover -s tools/hooks/tests -v
Requires requirements-dev.txt; never contacts the project's real remote.
"""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[3]


class PushHooksTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="electrobow-hooks-test-")
        self.addCleanup(self.directory.cleanup)
        base = Path(self.directory.name)
        self.repo = base / "checkout with spaces"
        self.repo.mkdir()
        self.remote = base / "remote.git"
        self.env = dict(
            os.environ,
            GIT_AUTHOR_NAME="Sammaele", GIT_AUTHOR_EMAIL="holysammael@gmail.com",
            GIT_COMMITTER_NAME="Sammaele", GIT_COMMITTER_EMAIL="holysammael@gmail.com",
            GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_COUNT="0",
        )
        self.run_command("git", "init", "--bare", str(self.remote))
        self.git("init", "--initial-branch=main")
        shutil.copytree(ROOT / "tools", self.repo / "tools", ignore=shutil.ignore_patterns("__pycache__"))
        shutil.copy(ROOT / ".pre-commit-config.yaml", self.repo)
        (self.repo / "CHANGELOG.md").write_text("## [1.0.0] - 2026-11-20\n\nInitial release.\n")
        self.git("add", ".")
        self.git("commit", "-m", "Initial fixture")
        self.git("remote", "add", "origin", str(self.remote))
        self.git("push", "-u", "origin", "main")
        # Exercise migration from the old repository-local hook setting.
        self.git("config", "core.hooksPath", ".githooks")
        self.run_command(sys.executable, "tools/install_hooks.py")

    def run_command(self, *command, succeeds=True):
        result = subprocess.run(
            command, cwd=self.repo, env=self.env,
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
        )
        if succeeds:
            self.assertEqual(result.returncode, 0, result.stdout)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        return result

    def git(self, *args, **kwargs):
        return self.run_command("git", *args, **kwargs)

    def test_installer_migrates_both_hooks_and_can_be_run_again(self):
        self.assertTrue((self.repo / ".git/hooks/pre-commit").is_file())
        self.assertIn("tools/hooks/pre_push.py", (self.repo / ".git/hooks/pre-push").read_text())
        self.git("config", "--get", "core.hooksPath", succeeds=False)
        self.run_command(sys.executable, "tools/install_hooks.py")
        self.assertFalse((self.repo / ".git/hooks/pre-push.legacy").exists())

    def test_annotated_and_lightweight_tags_on_published_commit(self):
        self.git("tag", "1.0.0")
        self.git("tag", "-a", "v1.0.0", "-m", "Release 1.0.0")
        result = self.git("push", "origin", "1.0.0", "v1.0.0")
        self.assertIn("Check pushed tags", result.stdout)
        self.assertIn("Passed", result.stdout)

    def test_missing_entry_blocks_tag_only_push_of_published_commit(self):
        self.git("tag", "v9.0.0")
        result = self.git("push", "origin", "v9.0.0", succeeds=False)
        self.assertIn("has no '## [9.0.0]' entry", result.stdout)
        self.assertEqual(self.git("ls-remote", "--tags", "origin").stdout, "")

    def test_branch_and_multiple_tags_are_checked_together(self):
        (self.repo / "notes.txt").write_text("Another commit\n")
        self.git("add", "notes.txt")
        self.git("commit", "-m", "Advance branch")
        self.git("tag", "v1.0.0")
        self.git("tag", "-a", "v9.0.0", "-m", "Missing release entry")
        result = self.git("push", "--atomic", "origin", "main", "v1.0.0", "v9.0.0", succeeds=False)
        self.assertIn("verified 'v1.0.0'", result.stdout)
        self.assertIn("has no '## [9.0.0]' entry", result.stdout)
        self.assertEqual(self.git("ls-remote", "--tags", "origin").stdout, "")

    def test_working_copy_changelog_cannot_make_a_bad_tag_valid(self):
        self.git("tag", "v9.0.0")
        with (self.repo / "CHANGELOG.md").open("a") as changelog:
            changelog.write("\n## [9.0.0]\n")
        result = self.git("push", "origin", "v9.0.0", succeeds=False)
        self.assertIn("has no '## [9.0.0]' entry", result.stdout)

    def test_missing_changelog_and_blob_tags_fail(self):
        self.git("rm", "CHANGELOG.md")
        self.git("commit", "-m", "Remove changelog")
        self.git("tag", "v1.0.0")
        self.git("tag", "blob-tag", "HEAD:.pre-commit-config.yaml")
        result = self.git("push", "origin", "v1.0.0", "blob-tag", succeeds=False)
        self.assertIn("tag 'v1.0.0' must point to a commit containing CHANGELOG.md", result.stdout)
        self.assertIn("tag 'blob-tag' must point to a commit containing CHANGELOG.md", result.stdout)

    def test_branch_push_ignores_unrelated_invalid_local_tag(self):
        self.git("tag", "v9.0.0")
        (self.repo / "notes.txt").write_text("Branch-only update\n")
        self.git("add", "notes.txt")
        self.git("commit", "-m", "Advance branch")
        self.git("push", "origin", "main")

    def test_tag_deletion_does_not_validate_changelog(self):
        self.git("tag", "v1.0.0")
        self.git("push", "origin", "v1.0.0")
        self.git("push", "origin", ":refs/tags/v1.0.0")
        self.assertEqual(self.git("ls-remote", "--tags", "origin").stdout, "")

    def test_destination_tag_name_determines_required_version(self):
        self.git("tag", "v1.0.0")
        result = self.git("push", "origin", "refs/tags/v1.0.0:refs/tags/v9.0.0", succeeds=False)
        self.assertIn("has no '## [9.0.0]' entry", result.stdout)

    @unittest.skipUnless(
        shutil.which("clang-format") or Path("/opt/homebrew/opt/llvm/bin/clang-format").is_file(),
        "clang-format is required for the formatting integration test",
    )
    def test_formatter_preserves_unstaged_changes_and_does_not_stage_fixes(self):
        staged = "int main(){return 0;}\n" + "// unchanged context\n" * 20
        source = self.repo / "example.cpp"
        source.write_text(staged)
        self.git("add", "example.cpp")
        source.write_text(staged + "// unstaged sentinel\n")
        result = self.git("commit", "-m", "Needs formatting", succeeds=False)
        self.assertIn("files were modified by this hook", result.stdout)
        self.assertIn("// unstaged sentinel", source.read_text())
        self.assertNotEqual(source.read_text(), staged + "// unstaged sentinel\n")
        self.assertEqual(self.git("show", ":example.cpp").stdout, staged)


if __name__ == "__main__":
    unittest.main()
