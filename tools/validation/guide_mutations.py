"""Compile each defective guide in an isolated workspace. Require its nominated numerical failure."""

from pathlib import Path
from shutil import copy2, copytree
from subprocess import run, PIPE, STDOUT
from tempfile import TemporaryDirectory

GUIDE = Path("rust/dsp/src/instrument/waveguide.rs")
RING = GUIDE.with_name("ring.rs")
PASSIVE = "passive_absolute_arrival"
EAGER = "evolving_eager_trajectory"
ATOMIC = "atomic_fault_boundary"

# Each mutation is a list of unique source replacements, including any
# instrumentation needed to change only the intended state.
cut = "let nut = self.nut.stage(sample.contact.to_nut)?;\n        bridge.commit();"
mutations = [
    (
        "reflection sign",
        PASSIVE,
        "positive compact return",
        [(GUIDE, "let from_nut = -self.nut.read", "let from_nut = self.nut.read")],
    ),
    (
        "missing FIR delay",
        PASSIVE,
        "absolute arrival",
        [(GUIDE, "(0.125 * x + 0.75 * self.loss[0] + 0.125 * self.loss[1])", "x")],
    ),
    (
        "extra sample",
        PASSIVE,
        "absolute arrival",
        [
            (
                GUIDE,
                "self.bridge.read(bridge_distance - 1.0)",
                "self.bridge.read(bridge_distance)",
            )
        ],
    ),
    (
        "reversed bridge weights",
        PASSIVE,
        "absolute arrival",
        [
            (
                RING,
                "let u = delay - m as f64;",
                "let mut u = delay - m as f64;\n        if matches!(self.arm, Arm::Bridge) && u!=0.0 {u=1.0-u;}",
            )
        ],
    ),
    (
        "wrong FIR preload",
        EAGER,
        "pre x2",
        [(GUIDE, "self.bridge.read_at(delay, 2)", "self.bridge.read_at(delay, 1)")],
    ),
    (
        "incorrect launch history",
        EAGER,
        "first divergence",
        [(RING, "t + 1.0 - BETA", "t + BETA")],
    ),
    (
        "launch P replaced on retune",
        EAGER,
        "first divergence",
        [
            (
                RING,
                "impl DelayRing {",
                "impl DelayRing {\n    pub fn replace_period(&mut self,p:f64){self.seed.as_mut().unwrap().period=p;}",
            ),
            (
                GUIDE,
                "let period = self.rate / hz;",
                "let period = self.rate / hz;\n        self.nut.replace_period(period);",
            ),
        ],
    ),
    (
        "wrong arm routing",
        EAGER,
        "stored B",
        [
            (
                GUIDE,
                "self.bridge.stage(sample.contact.to_bridge)",
                "self.bridge.stage(sample.contact.to_nut)",
            ),
            (
                GUIDE,
                "self.nut.stage(sample.contact.to_nut)",
                "self.nut.stage(sample.contact.to_bridge)",
            ),
        ],
    ),
    (
        "forget prior branch",
        EAGER,
        "first divergence",
        [(GUIDE, "                self.mode,", "                Mode::Stick,")],
    ),
    (
        "commit before second stage",
        ATOMIC,
        "partial scalar/cursor commit",
        [
            (
                GUIDE,
                cut,
                "bridge.commit();\n        let nut = self.nut.stage(sample.contact.to_nut)?;",
            )
        ],
    ),
    (
        "stale bridge on invalidation",
        EAGER,
        "assertion",
        [(GUIDE, "self.bridge.invalidate();", "// stale bridge epoch")],
    ),
]

with TemporaryDirectory() as tmp:
    root = Path(tmp)
    for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
        copy2(name, root / name)
    copytree("rust/dsp", root / "rust/dsp")
    for label, test, reason, edits in mutations:
        for path, before, after in edits:
            target = root / path
            source = target.read_text()
            assert source.count(before) == 1, (label, before)
            target.write_text(source.replace(before, after))
        name = "instrument::waveguide::tests::" + test
        proc = run(
            ["cargo", "test", "--locked", "--offline", "--lib", name, "--", "--exact"],
            cwd=root,
            stdout=PIPE,
            stderr=STDOUT,
            text=True,
        )
        assert proc.returncode and f"test {name} ... FAILED" in proc.stdout, proc.stdout
        assert (
            "panicked at rust/dsp/src/instrument/waveguide/tests.rs:" in proc.stdout
            and reason in proc.stdout
        ), proc.stdout
        print(
            label,
            test,
            next(line for line in proc.stdout.splitlines() if reason in line),
            flush=True,
        )
        for path, _, _ in edits:
            copy2(path, root / path)
