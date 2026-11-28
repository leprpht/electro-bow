"""Replay the three Review 1 branch defects on disposable copies of the Rust solver."""

import subprocess
from pathlib import Path

from contact_contract import compare, counterexample, graph, verify_physics

OUT = Path("build/stage-07/contact-mutants")
SOURCE = Path("rust/dsp/src/instrument/friction.rs")
DRIVER = Path(__file__).with_name("contact_replay.rs")


def main():
    OUT.mkdir(exist_ok=True, parents=True)
    source = SOURCE.read_text()
    start = source.index("        for i in (0..last).rev() {")
    end = source.index("        None\n", start)
    ascending = """        for i in 0..last {
            *probes += 1;
            let left = Q[i] + k * self.mu[i];
            let right = Q[i + 1] + k * self.mu[i + 1];
            if left.min(right) <= h && h <= left.max(right) && left != right {
                let fraction = (h - left) / (right - left);
                return Some(Q[i] + (Q[i + 1] - Q[i]) * fraction);
            }
        }
"""
    for name, before, after, (d, k) in [
        (
            "cutoff",
            "if left <= h && h <= right {",
            "if left <= h && h <= right && right-left > 1e-12 {",
            counterexample(),
        ),
        (
            "static",
            "previous != Mode::Slide(direction) && h <= limit",
            "h <= limit",
            (0.14, 0.1),
        ),
        ("lower", source[start:end], ascending, (0.14, 0.1)),
    ]:
        if source.count(before) != 1:
            raise ValueError("ambiguous mutation site")
        (OUT / "friction.rs").write_text(source.replace(before, after))
        driver = OUT / "driver.rs"
        driver.write_text(
            DRIVER.read_text().replace(
                "../../rust/dsp/src/instrument/friction.rs", "friction.rs"
            )
        )
        binary = OUT / name
        subprocess.run(
            [
                "rustc",
                "--edition",
                "2024",
                "-O",
                "-A",
                "warnings",
                str(driver),
                "-o",
                str(binary),
            ],
            check=True,
        )
        j, r, mode, _ = subprocess.check_output(
            [binary], input=f"{d!r} {k!r} 1\n", text=True
        ).split()
        actual = float(j), float(r), int(mode)
        verify_physics(d, k, actual)
        try:
            compare(d, actual, graph(d, k, 1))
        except AssertionError:
            print(name, actual, "rejected by branch oracle")
        else:
            raise ValueError("escaped " + name)


if __name__ == "__main__":
    main()
