"""Compile each defective component/harness and require the nominated runtime divergence."""

import json
import subprocess

import numpy as np
from guide_audit_build import build
from o2_bench import OUT, compare, replay

# Short impulses discriminate signs and gains before unstable mutants can overflow.
FILTERS = {
    "missing blocker": ("coeff.c * ((self.h + v) - self.v)", "v", "h"),
    "blocker feedback": (
        "coeff.c * ((self.h + v) - self.v)",
        "coeff.c * self.h + (v-self.v)",
        "h",
    ),
    "unwarped": ("(PI * 3200.0 / fs).tan()", "PI * 3200.0 / fs", "l"),
    "wrong cutoff": ("PI * 3200.0 / fs", "PI * 3000.0 / fs", "l"),
    "denominator sign": (
        "coeff.b1 * a - coeff.a1 * l",
        "coeff.b1 * a + coeff.a1 * l",
        "l",
    ),
    "state update": ("+ self.z2;", "+ self.z1;", "l"),
    "missing trim": ("0.85 * h / SQRT_2", "0.85 * h", "a"),
    "duplicate conversion": ("0.85 * h / SQRT_2", "0.85 * 0.85 * h / SQRT_2", "a"),
    "duplicate trim": ("0.85 * h / SQRT_2", "(0.85 * h / SQRT_2) / SQRT_2", "a"),
    "reversed blend": ("* (a - l)", "* (l - a)", "q"),
    "excess detail": ("0.025 * noise", "0.25 * noise", "q"),
}
LIFECYCLE = {
    "missing reset": (
        "o2",
        "*self = Self::default();",
        "/* stale memory */",
        "dirty reset",
    ),
    "partial publication": (
        "o2",
        "        if ![h, a, l, z1, z2, q]",
        "        self.v = v;\n        if ![h, a, l, z1, z2, q]",
        "partial publication",
    ),
    "stale observation": (
        "harness",
        "self.lane.tick(&self.coeff, o.velocity(), detail)",
        "self.lane.tick(&self.coeff, self.lane.state()[0], detail)",
        "same-tick velocity",
    ),
    "post-commit replay": (
        "harness",
        "self.dispose();",
        "let _ = self.guide.tick_observed(hz, speed, k); self.dispose();",
        "physical commit count",
    ),
}


def main():
    out = OUT / "mutations"
    out.mkdir(parents=True, exist_ok=True)
    report = {}
    x = np.zeros(32)
    x[0] = 1
    for name, (site, edit, tap) in FILTERS.items():
        binary = out / name.replace(" ", "-")
        build(binary, o2=True, mutation=("o2", site, edit))
        result = replay(48000, x, 0.5, binary)
        try:
            compare(48000, name, x, 0.5, result[:, :4], result[:, 4:])
        except ValueError as err:
            if "tap=" + tap + " " not in str(err):
                raise
            report[name] = str(err)
        else:
            raise ValueError("escaped " + name)
    for name, (module, site, edit, marker) in LIFECYCLE.items():
        binary = out / name.replace(" ", "-")
        build(binary, o2=True, mutation=(module, site, edit))
        p = subprocess.run(
            [binary, "--o2-lifecycle", "48000"],
            capture_output=True,
            text=True,
            check=False,
        )
        if p.returncode == 0 or marker not in p.stderr:
            raise ValueError(f"{name}: {p.stderr}")
        report[name] = marker
    (OUT / "mutations.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"{len(report)} compiled and rejected")


if __name__ == "__main__":
    main()
