"""Compile the entry revision and the observed implementation with identical private-state probes."""

import os
import shutil
import subprocess
from hashlib import sha256
from itertools import zip_longest
from pathlib import Path
from tempfile import TemporaryDirectory

ENTRY = "0c98f608a0d46e29693320f36e85f173cd3faefe"
INSTRUMENT = Path("rust/dsp/src/instrument")
PROBE = Path(__file__).with_name("o1_state.rs")


def main():
    temp = Path("build/stage-05/tmp")
    temp.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(dir=temp) as directory:
        root = Path(directory).resolve()
        for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
            shutil.copy2(name, root)
        shutil.copytree("rust/dsp", root / "rust/dsp")
        outputs = []
        for label in ("entry", "observed"):
            for name in ("mod.rs", "friction.rs", "ring.rs", "waveguide.rs"):
                path = INSTRUMENT / name
                code = (
                    subprocess.check_output(["git", "show", f"{ENTRY}:{path}"])
                    if label == "entry"
                    else path.read_bytes()
                )
                (root / path).write_bytes(code)
            tick = (
                "g.tick(hz, 0.1, k)"
                if label == "entry"
                else "g.tick_observed(hz, 0.1, k).map(|v| v.map(|(s,o)| { std::hint::black_box(o.velocity()); s }))"
            )
            wrapper = f"fn advance(g: &mut Guide, hz:f64, k:f64)->Result<Option<Sample>,Fault>{{ {tick} }}"
            (root / INSTRUMENT / "waveguide/tests.rs").write_text(
                PROBE.read_text() + wrapper
            )
            trace = root / label
            subprocess.run(
                [
                    "cargo",
                    "test",
                    "--release",
                    "--lib",
                    "--locked",
                    "--offline",
                    "instrument::waveguide::tests::epoch_state_trace",
                    "--",
                    "--exact",
                ],
                cwd=root,
                env={**os.environ, "O1_STATE_TRACE": str(trace)},
                check=True,
            )
            outputs.append(trace.read_bytes())
        digest = sha256(outputs[0]).hexdigest()
        for i, (a, b) in enumerate(zip_longest(*(v.splitlines() for v in outputs))):
            if a != b:
                raise ValueError(f"first state mismatch at row {i}: {a!r} vs {b!r}")
        print(
            f"{len(outputs[0].splitlines())} entry/observed state rows identical SHA-256 {digest}"
        )


if __name__ == "__main__":
    main()
