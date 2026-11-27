"""Build the offline renderer with read-only epoch probes and invalidation telemetry.

Use Rust's existing test counters. The disposable build has the same module layout
as the standalone adapter. Empty unit-test modules keep the crate harness separate.
"""

from pathlib import Path
from shutil import copy2
from subprocess import run
from tempfile import TemporaryDirectory

GUIDE_DIR = Path("rust/dsp/src/instrument")
ADAPTER = Path("tools/validation/guide_render.rs")
PROBES = ADAPTER.with_name("guide_epoch.rs")


def build(binary, late_relaunch=False, *, o2=False, mutation=None):
    with TemporaryDirectory(dir=binary.parent if o2 else None) as tmp:
        root = Path(tmp)
        src = root / GUIDE_DIR
        src.mkdir(parents=True)
        adapter = root / ADAPTER
        adapter.parent.mkdir(parents=True)
        copy2(ADAPTER, adapter)
        (src / "tests.rs").touch()
        for name in ("ring", "friction", "waveguide"):
            sub = src / name
            sub.mkdir()
            (sub / "tests.rs").touch()
            target = src / (name + ".rs")
            code = (GUIDE_DIR / target.name).read_text()
            if name == "ring":
                for before, after in (
                    (
                        "    write: usize,",
                        "    opens: usize,\n    clears: usize,\n    write: usize,",
                    ),
                    (
                        "            write: 0,",
                        "            opens: 0,\n            clears: 0,\n            write: 0,",
                    ),
                    (
                        "self.seed = Some(Seed {",
                        "self.opens += 1;\n        self.seed = Some(Seed {",
                    ),
                    (
                        "pub fn invalidate(&mut self) {",
                        "pub fn invalidate(&mut self) {\n        self.clears += 1;",
                    ),
                ):
                    if code.count(before) != 1:
                        raise ValueError(f"unknown ring layout at {before}")
                    code = code.replace(before, after)
                code += """
impl DelayRing {
    pub(super) fn check_epoch(&self, ticks: usize, seed: (f64, f64)) {
        assert_eq!((self.opens, self.clears), (1, 1), "ring epoch change at {ticks}");
        let capacity = self.samples.len();
        assert_eq!(self.write, ticks % capacity, "ring cursor at {ticks}");
        assert_eq!(self.count, ticks.min(capacity), "ring count at {ticks}");
        assert_eq!(self.seed.map(|s| (s.period, s.amplitude)), Some(seed));
    }
}
"""
            if name == "waveguide":
                # Count calls at the actual invalidation boundary, including launch.
                for before, after in (
                    ("    live: bool,", "    invalidations: usize,\n    live: bool,"),
                    (
                        "            live: false,",
                        "            invalidations: 0,\n            live: false,",
                    ),
                    (
                        "pub fn invalidate(&mut self) {",
                        "pub fn invalidate(&mut self) {\n        self.invalidations += 1;",
                    ),
                ):
                    if code.count(before) != 1:
                        raise ValueError(f"unknown guide layout at {before}")
                    code = code.replace(before, after)
                if late_relaunch:
                    for before, after in (
                        (
                            "    live: bool,",
                            "    ticks: usize,\n    bend_start: bool,\n    armed: bool,\n    live: bool,",
                        ),
                        (
                            "            live: false,",
                            "            ticks: 0,\n            bend_start: false,\n            armed: false,\n            live: false,",
                        ),
                        (
                            "        let result = self.transaction",
                            """        if self.ticks == 0 { self.bend_start = hz == 196.0; }
        if self.bend_start && self.ticks == 57600 && hz > 210.0 { self.armed = true; }
        if self.ticks == 96000 && self.armed { self.launch(hz, speed)?; }
        self.ticks += 1;
        let result = self.transaction""",
                        ),
                    ):
                        if code.count(before) != 1:
                            raise ValueError(f"unknown mutation site at {before}")
                        code = code.replace(before, after)
                code += PROBES.read_text()
            target.write_text(code)
        if o2:
            for name in ("o2_replay.rs", "o2_lifecycle.rs"):
                copy2(ADAPTER.with_name(name), adapter.with_name(name))
            lane = src / "o2.rs"
            lane.write_text(
                (GUIDE_DIR / "o2.rs").read_text()
                + ADAPTER.with_name("o2_probes.rs").read_text()
            )
            (src / "o2").mkdir()
            (src / "o2/tests.rs").touch()
            # Count physical commits at the real two-ring publication boundary.
            guide = src / "waveguide.rs"
            code = guide.read_text()
            for site, replacement in [
                (
                    "    invalidations: usize,",
                    "    pub commits: usize,\n    invalidations: usize,",
                ),
                (
                    "            invalidations: 0,",
                    "            commits: 0,\n            invalidations: 0,",
                ),
                (
                    "        nut.commit();",
                    "        nut.commit();\n        self.commits += 1;",
                ),
            ]:
                if code.count(site) != 1:
                    raise ValueError(site)
                code = code.replace(site, replacement)
            guide.write_text(code)
            if mutation:
                name, before, after = mutation
                path = lane if name == "o2" else adapter.with_name("o2_replay.rs")
                code = path.read_text()
                if code.count(before) != 1:
                    raise ValueError(f"ambiguous mutation {before}")
                path.write_text(code.replace(before, after))
        run(
            [
                "rustc",
                "--edition",
                "2024",
                "--cfg",
                "test",
                "--cfg",
                "epoch_audit",
                *(["--cfg", "o2_audit"] if o2 else []),
                "-O",
                str(adapter),
                "-o",
                str(binary.resolve()),
            ],
            check=True,
        )
