"""Compile disposable defects and require the nominated runtime failure in actual O1."""

import shutil
import subprocess
from pathlib import Path
from tempfile import TemporaryDirectory

GUIDE = Path("rust/dsp/src/instrument/waveguide.rs")
RING = GUIDE.with_name("ring.rs")
BODY = """Ok(self
            .advance(hz, speed, k)?
            .map(|(sample, period)| (sample, Observation::read(&self.nut, period))))"""
GRID = "chronological_grid"
STATION = "first O1 divergence station"
DIRECT = "first O1 divergence direct"
REFLECTED = "first O1 divergence reflected history"

CASES = [
    (
        "single ray",
        GRID,
        STATION,
        [(GUIDE, "self.direct - self.reflected_history", "self.direct")],
    ),
    (
        "positive reflection",
        GRID,
        STATION,
        [
            (
                GUIDE,
                "self.direct - self.reflected_history",
                "self.direct + self.reflected_history",
            )
        ],
    ),
    (
        "reversed velocity",
        GRID,
        STATION,
        [
            (
                GUIDE,
                "self.direct - self.reflected_history",
                "self.reflected_history - self.direct",
            )
        ],
    ),
    (
        "pre-commit history",
        "first_high_note_uses_committed_emission",
        "committed emission rate=32000",
        [
            (
                GUIDE,
                BODY,
                """
        let old = self.live.then(|| Observation::read(&self.nut, self.rate / hz));
        Ok(self.advance(hz, speed, k)?.map(|(s, _)| (s, old.unwrap())))""",
            )
        ],
    ),
    (
        "extra lag",
        GRID,
        DIRECT,
        [(RING, "self.interpolate(delay, 1)", "self.interpolate(delay, 2)")],
    ),
    (
        "reversed O1 interpolation",
        GRID,
        DIRECT,
        [
            (
                RING,
                "self.interpolate(delay, 1)",
                "self.interpolate(delay.ceil() - delay.fract(), 1)",
            )
        ],
    ),
    (
        "extra bridge reflection",
        GRID,
        REFLECTED,
        [
            (
                GUIDE,
                "nut.read_committed((2.0 - position - BETA) * period / 2.0),",
                "nut.read_committed((2.0 - position - BETA) * period / 2.0) - 0.98 * nut.read_committed((position + BETA) * period / 2.0),",
            )
        ],
    ),
    (
        "wrong center",
        GRID,
        DIRECT,
        [(GUIDE, "[0.162, 0.174, 0.186]", "[0.162, 0.170, 0.186]")],
    ),
    (
        "uniform aperture",
        GRID,
        "first O1 divergence aperture",
        [
            (
                GUIDE,
                "(left + 4.0 * center + right) / 6.0",
                "(left + center + right) / 3.0",
            )
        ],
    ),
    (
        "retuned historical seed",
        GRID,
        REFLECTED,
        [
            (
                RING,
                "impl DelayRing {",
                """impl DelayRing {
            pub fn swap_seed_period(&mut self, p: f64) -> f64 {
                std::mem::replace(&mut self.seed.as_mut().unwrap().period,p)
            }""",
            ),
            (
                GUIDE,
                "(sample, Observation::read(&self.nut, period))",
                """{
            let saved = self.nut.swap_seed_period(period);
            let observation = Observation::read(&self.nut, period);
            self.nut.swap_seed_period(saved);
            (sample, observation)
        }""",
            ),
        ],
    ),
    (
        "publish last epoch while cleared",
        "lifecycle_parity",
        "observed lifecycle tick=1250",
        [
            (
                GUIDE,
                "    live: bool,",
                "    cached: Option<(Sample,Observation)>,\n    live: bool,",
            ),
            (
                GUIDE,
                "            live: false,",
                "            cached: None,\n            live: false,",
            ),
            (
                GUIDE,
                BODY,
                """let result = self.advance(hz,speed,k)?
                    .map(|(s,p)| (s,Observation::read(&self.nut,p)));
            self.cached = result.or(self.cached);
            Ok(self.cached)""",
            ),
        ],
    ),
    (
        "decompose after float shift",
        "preserves_fraction_at_post_commit",
        "lost original fraction",
        [(RING, "self.interpolate(delay, 1)", "self.interpolate(delay + 1.0, 0)")],
    ),
]


def main():
    out = Path("build/stage-05/tmp")
    out.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(dir=out) as tmp:
        root = Path(tmp)
        for filename in ["Cargo.lock", "Cargo.toml", "rust-toolchain.toml"]:
            shutil.copy2(filename, root)
        shutil.copytree("rust/dsp", root / "rust/dsp")
        for label, test, predicate, edits in CASES:
            for path, old, new in edits:
                dest = root / path
                src = dest.read_text()
                if src.count(old) != 1:
                    raise ValueError((label, old))
                dest.write_text(src.replace(old, new))
            full = f"instrument::waveguide::tests::o1::{test}"
            r = subprocess.run(
                [
                    "cargo",
                    "test",
                    "--release",
                    "--lib",
                    "--locked",
                    "--offline",
                    full,
                    "--",
                    "--exact",
                ],
                check=False,
                cwd=root,
                capture_output=True,
                text=True,
            )
            if not (
                r.returncode
                and f"test {full} ... FAILED" in r.stdout
                and predicate in r.stdout
                and "panicked at" in r.stdout
            ):
                raise RuntimeError(f"{label}\n{r.stdout}\n{r.stderr}")
            print(
                label,
                next(line for line in r.stdout.splitlines() if predicate in line),
                flush=True,
            )
            for path, _, _ in edits:
                shutil.copy2(path, root / path)


if __name__ == "__main__":
    main()
