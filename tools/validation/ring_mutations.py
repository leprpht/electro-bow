"""Verify the named Rust test fails on each isolated mutation (repository root)."""

from pathlib import Path
from shutil import copy2, copytree
from subprocess import run, PIPE, STDOUT
from tempfile import TemporaryDirectory


def replace_once(source, before, after):
    assert source.count(before) == 1, before
    return source.replace(before, after)


ring = Path("rust/dsp/src/instrument/ring.rs")
tests = ring.with_suffix("") / "tests.rs"
impulse = "impulse_before_commit"
edge = "full_capacity_neighbor_across_wraps"
write = "        push(&mut ring, f64::from(n == 0));"
# The caller owns read-before-write. Mutate its tick order while retaining
# the original, analytic expected impulse arrivals.
suite = tests.read_text()
start, rest = suite.split("fn full_capacity_neighbor_across_wraps()", 1)
early = replace_once(start, write, "")
early = replace_once(early, "for n in 0..4 * c {", "for n in 0..4 * c {\n" + write)
early += "fn full_capacity_neighbor_across_wraps()" + rest
original = ring.read_text()
mutations = [
    ("commit before reads", tests, early, impulse),
    ("historical index one too recent", ring, replace_once(original,
        "return self.samples[slot];", "return self.samples[(slot + 1) % self.samples.len()];"), impulse),
    ("swapped weights", ring, replace_once(original, "(1.0 - u) * newer + u *",
        "u * newer + (1.0 - u) *"), impulse),
    ("wrap one slot early", ring, replace_once(original, "ring.write == ring.samples.len()",
        "ring.write == ring.samples.len() - 1"), edge),
    ("missing oldest interpolation neighbor", ring, replace_once(original,
        "self.history(lag + 1, seed)", "self.history((lag + 1).min(self.samples.len() - 1), seed)"), edge),
    ("stale epoch after invalidation", ring, replace_once(original,
        "self.seed = None;", "// old seed still valid"), "invalidation_and_relaunch_hide_stale_slots"),
]

with TemporaryDirectory() as tmp:
    root = Path(tmp)
    for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
        copy2(name, root / name)
    copytree("rust/dsp", root / "rust/dsp")
    for name, path, source, test in mutations:
        target = root / path
        target.write_text(source)
        full_name = "instrument::ring::tests::" + test
        result = run(["cargo", "test", "--locked", "--offline", "--lib", full_name,
                      "--", "--exact"], cwd=root, stdout=PIPE, stderr=STDOUT, text=True)
        assert result.returncode and f"test {full_name} ... FAILED" in result.stdout, result.stdout
        assert "panicked at rust/dsp/src/instrument/ring/tests.rs:" in result.stdout, result.stdout
        print(f"{name} rejected by {test}")
        copy2(path, target)
