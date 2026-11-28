"""Inventory and independent sample/measurement predicates for the Stage 08 exit."""

import math
from copy import deepcopy
from itertools import combinations, product

import numpy as np
from acoustic_identity import audio
from acoustic_observation import db, rms
from i1_clock import OUT
from i1_safety import GRID, check_state, check_trajectory, inspect, moments
from o2_coverage import RATE, keys
from o2_qualification import strongest
from o2_response import require
from scipy.io import wavfile


def stationary_inventory():
    expected = {
        (48000, f, 0.1, r, "opposed")
        for f, r in product((82.41, 196, 440), (1.2, 1.8, 2.4))
    }
    expected.update((fs, 1318.51, 0.1, 1.8, "opposed") for fs in RATE)
    expected.update(
        (48000, 440, v, r, "opposed") for v, r in product((0.06, 0.16), (1.2, 1.8, 2.4))
    )
    expected.update(
        (48000, f, 0.1, 1.8, v)
        for f, v in product((82.41, 196, 440), ("upper", "lower", "inversion"))
    )
    expected.update(
        (48000, 196, 0.1, 1.8, v)
        for v in ("weak-H1", "missing-six", "dip-six", "bright", "dark")
    )
    return {"/".join(map(str, k)): k for k in expected}


def grade_depths(depths, fs, f, m5_rms, neutral_rms):
    keys(range(len(depths)), range(3), "measurement depths")
    for i, (row, nr) in enumerate(zip(depths, neutral_rms)):

        def ratio(num, den):
            return float(db(num / den)) if den > 1e-12 else None

        level = ratio(row["rms_FS"], m5_rms)
        require(
            level is not None and abs(level - row["final_over_M5"]) < 1e-9,
            "original M5 denominator",
        )
        require(
            abs(row["corrected_over_neutral"] - ratio(row["rms_FS"], nr)) < 1e-9,
            "same-depth reference",
        )
        require(
            strongest(row["measured_f0"], f) and row["failure_reason"] is None,
            "ineligible M5",
        )
        require(abs(level) <= 3 and abs(row["corrected_over_neutral"]) <= 3, "level")
        require(strongest(row["strongest_hz"], f), "fundamental")
        h = row["harmonics"]
        count = sum(k * row["measured_f0"] < fs / 2 for k in range(1, 13))
        require(
            all(len(h[k]) == count for k in ("peak_FS", "phase_deg", "dBr")),
            "harmonic inventory",
        )
        for j, (amp, phase, rel) in enumerate(
            zip(h["peak_FS"], h["phase_deg"], h["dBr"])
        ):
            eligible = amp > 1e-12 and h["peak_FS"][0] > 1e-12
            require(
                (phase is not None) == eligible and (rel is not None) == eligible,
                "harmonic eligibility",
            )
            if eligible:
                require(
                    math.isfinite(phase)
                    and abs(phase) <= 180
                    and abs(rel - db(amp / h["peak_FS"][0])) < 1e-9,
                    "relative harmonic",
                )
        require(
            0 <= row["spectral"]["eta"] <= 1
            and abs(row["spectral"]["eta"] + row["spectral"]["residual"] - 1) < 1e-12,
            "spectral energy",
        )
        if i == 1 or (i == 0 and fs == 32000 and f == 1318.51):
            require(row["spectral"]["eta"] >= 0.85, "harmonic fraction")
        if i == 1:
            residual = ratio(row["factory_wet_difference_rms_FS"], depths[0]["rms_FS"])
            require(
                residual is not None
                and abs(residual - row["factory_difference_over_off"]) < 1e-9
                and residual <= -28,
                "final-wet residual",
            )
        else:
            require(
                row["factory_difference_over_off"] is None
                and row["factory_wet_difference_rms_FS"] is None,
                "ineligible factory residual",
            )
        flags = row["passes"]
        required = {"level", "coloration", "bridge_anchor", "fundamental"}
        if i == 1:
            required.update(("factory_eta", "factory_residual"))
        if (fs, f, i) == (32000, 1318.51, 0):
            required.add("high_note_eta")
        keys([k for k, v in flags.items() if v is not None], required, "V17 predicates")
        require(all(flags[k] is True for k in required), "recorded V17 predicate")


def pack(result):
    from i1_study import FIXTURES, carrier

    clips = [
        "techs-P1-60-DI",
        "techs-P2-60-DI",
        "techs-P1-60-held",
        "techs-P2-60-held",
        "neutral-held",
        "bend",
        "gain-veto",
    ]
    keys(result["listening_gains"], clips, "listening inventory")
    for name in clips:
        fs, engineering = wavfile.read(
            OUT / "listening" / "engineering" / (name + ".wav")
        )
        _, matched = wavfile.read(OUT / "listening" / "matched" / (name + ".wav"))
        gain = result["listening_gains"][name]
        require(
            np.max(abs(engineering.astype(float) * gain - matched)) < 1e-7,
            "hidden listening normalization",
        )
        if name.endswith("DI"):
            rate, x = audio(FIXTURES / (name[:-3] + ".wav"))
            start = np.flatnonzero(abs(x) > 0.01)[0]
            require(
                fs == rate
                and np.array_equal(
                    engineering, x[start : start + len(engineering)].astype("f4")
                ),
                "DI gain",
            )
        if name == "neutral-held":
            require(
                np.array_equal(engineering, carrier(fs, 196)[-fs:, 25].astype("f4")),
                "engineering normalization",
            )


def audit_structure(result):
    from guide_qualification import digest
    from i1_study import carrier

    keys(
        result["clock"]["negatives"],
        (
            "wrong launch token",
            "wrong capture window",
            "wrong event order",
            "future input",
            "invalid refresh",
            "stale owner",
            "late",
            "wrong epoch",
        ),
        "clock controls",
    )
    require(result["clock"]["sha256"] == digest(OUT / "clock.json"), "clock provenance")
    keys(
        result["negatives"],
        (
            "independent clipping inversion",
            "independent clipping upper",
            "independent clipping lower",
            "wrong M5 denominator",
            "wrong residual",
            "ineligible phase",
            "lost pressure variation",
            "neutral-only identity",
            "missing required case",
            "hidden normalization",
            "unbounded bank state",
            "unsafe smoothing",
            "Nyquist veto",
        ),
        "negative controls",
    )
    require(
        all(isinstance(v, str) and v for v in result["negatives"].values()),
        "missing rejection",
    )
    inventory = stationary_inventory()
    keys(result["stationary"], inventory, "I1 cases")
    refs = {}
    for k, (fs, f, v, r, name) in inventory.items():
        row = result["stationary"][k]
        if (fs, f, v, r) not in refs:
            raw = carrier(fs, f, v, r)
            refs[fs, f, v, r] = rms(raw[-fs:, 11]), [rms(x) for x in raw[-fs:, 25:28].T]
        for field in ("neutral_depths", "depths"):
            grade_depths(row[field], fs, f, *refs[fs, f, v, r])
        require(
            len(row["sweep"]) == 3
            and all(65 <= hz <= 4000 and d <= 6 for hz, d in row["sweep"]),
            "sweep peak",
        )
        scale = row["policy"]["settled_scale"]
        coeff = np.expm1(np.log(10) * np.array(row["requested_dB"]) / 20)
        accepted = np.asarray(row["accepted_dB"])
        require(
            len(coeff) == 12
            and np.max(abs(accepted)) <= 5 + 1e-12
            and np.allclose(
                accepted, 20 * np.log10(1 + scale * coeff), rtol=0, atol=1e-10
            ),
            "realized coefficient ray",
        )
    takes = {f"techs-P{p}-{s}" for p, s in product((1, 2), (60, 132, 204))}
    keys(result["real"]["takes"], takes, "actual source takes")
    pairkeys = {frozenset(k.split("/")) for k in result["real"]["pairs"]}
    keys(pairkeys, map(frozenset, combinations(takes, 2)), "source pairs")
    for take in result["real"]["takes"].values():
        grade_depths(take["depths"], 48000, 196, *refs[48000, 196, 0.1, 1.8])
    for pair in result["real"]["pairs"].values():
        require(
            pair["neutral"] == 0
            and all(
                math.isfinite(v) for v in (pair["di"], pair["safe"], pair["unsafe"])
            ),
            "source distances",
        )
    keys(
        result["complex"],
        (
            f"{fs}/{f}"
            for fs, f in product((32000, 48000, 192000), (82.41, 196, 1318.51))
        ),
        "complex grid",
    )
    require(all(0 <= e <= 1e-9 for e in result["complex"].values()), "complex transfer")
    for row in result["dynamic"].values():
        require(
            row["timeline"]
            and all(
                w["scale"] in GRID and (w["scale"] == 0 or not w["veto"])
                for w in row["timeline"]
            ),
            "monitor decisions",
        )
    require(
        result["dynamic"]["lift"]["released_peak_after_zero"] == 0, "release disposal"
    )
    require(
        result["dynamic"]["long-hold"]["delta_rms"] > 1e-5
        and result["dynamic"]["bend"]["delta_rms"] > 1e-5,
        "missing held correction",
    )
    keys(
        result["measured_sweep"],
        (
            f"{name}/{d}"
            for name, d in product(
                ("weak-H1", "upper", "lower", "inversion", "opposed"), (0, 0.5, 1)
            )
        ),
        "measured sweep",
    )
    for measurement in result["measured_sweep"].values():
        require(
            measurement["peak"]["dB"] <= 6
            and 0 <= measurement["complex_error"] <= 1e-9,
            "measured sweep",
        )
    corner_grid = list(
        product(
            (0.06, 0.16),
            (1.2, 2.4),
            ("neutral", "upper", "lower", "inversion"),
            (0, 0.5, 1),
            (0.08, 1.4),
            (0, 1),
            (0, 2),
        )
    )
    keys(
        result["corners"],
        ("/".join(map(str, k)) for k in corner_grid),
        "corrected gain corners",
    )
    for case in corner_grid:
        row = result["corners"]["/".join(map(str, case))]
        g, c, trim = case[-3:]
        expected = trim * (g * row["unity_wet_peak"] if c else 0.35)
        require(
            abs(row["peak"] - expected) <= 1e-9
            and abs(row["demand_dB"] - max(0.0, db(expected / 0.98))) <= 1e-9,
            "corrected corner levels",
        )
    pack(result)


def audit(result):
    """Standalone Review 8 report audit. Reconstruct measurements from actual taps."""
    import json
    import os
    import subprocess
    import sys
    from pathlib import Path
    from tempfile import TemporaryDirectory

    from i1_study import O2

    audit_structure(result)
    with TemporaryDirectory(dir=OUT.parent) as tmp:
        env = dict(os.environ, ELECTROBOW_I1_OUT=tmp, ELECTROBOW_O2_OUT=str(O2))
        subprocess.run(
            [
                sys.executable,
                Path(__file__).with_name("i1_study.py"),
                "--historical-measurements",
            ],
            env=env,
            check=True,
        )
        expected = json.loads((Path(tmp) / "i1-results.json").read_text())
    require(result == expected, "historical report differs from waveform replay")


def negative_controls(result):
    """Defects use actual measured levels and spectra, including historical unsafe rows."""
    import json

    from i1_study import O2

    rejected = {}

    def reject(name, fn, reason):
        try:
            fn()
        except ValueError as err:
            require(reason in str(err), f"{name} wrong failure {err}")
            rejected[name] = str(err)
        else:
            raise ValueError("escaped " + name)

    data = result["stationary"]["48000/196/0.1/1.8/opposed"]
    neutral = data["neutral_depths"]
    original_rms = neutral[0]["rms_FS"] / 10 ** (neutral[0]["final_over_M5"] / 20)
    nr = [r["rms_FS"] for r in neutral]

    def check(rows):
        grade_depths(rows, 48000, 196, original_rms, nr)

    actual = json.loads((O2 / "results.json").read_text())["v17"]
    for name, reason in (
        ("inversion", "fundamental"),
        ("upper", "level"),
        ("lower", "level"),
    ):
        reject(
            "independent clipping " + name,
            lambda name=name: check([actual["stress/196/" + name], *neutral[1:]]),
            reason,
        )
    for name, depth, mutate, reason in (
        ("wrong M5 denominator", 0, lambda r: r.update(final_over_M5=0), "original M5"),
        (
            "wrong residual",
            1,
            lambda r: r.update(factory_difference_over_off=-240),
            "final-wet",
        ),
        (
            "ineligible phase",
            0,
            lambda r: r["harmonics"]["phase_deg"].__setitem__(0, None),
            "eligibility",
        ),
    ):
        rows = deepcopy(data["depths"])
        mutate(rows[depth])
        reject(name, lambda rows=rows: check(rows), reason)
    from i1_study import grade

    def check_expression(changed):
        failures, _ = grade({**result, "stationary": changed})
        require(not failures, ", ".join(failures))

    flat_pressure = deepcopy(result["stationary"])
    flat_pressure["48000/440/0.1/2.4/opposed"]["depths"] = flat_pressure[
        "48000/440/0.1/1.2/opposed"
    ]["depths"]
    reject(
        "lost pressure variation",
        lambda: check_expression(flat_pressure),
        "pressure contrast",
    )
    neutral_sources = deepcopy(result["stationary"])
    for name in ("bright", "dark"):
        entry = neutral_sources[f"48000/196/0.1/1.8/{name}"]
        entry["depths"] = entry["neutral_depths"]
    reject(
        "neutral-only identity",
        lambda: check_expression(neutral_sources),
        "source transfer",
    )
    reject(
        "missing required case",
        lambda: keys(
            list(result["stationary"])[1:], stationary_inventory(), "I1 cases"
        ),
        "inventory",
    )
    require(
        "neutral-held" in result["listening_gains"], "missing engineering reference"
    )
    changed = deepcopy(result["listening_gains"])
    changed["neutral-held"] *= 2
    reject(
        "hidden normalization",
        lambda: pack({"listening_gains": changed}),
        "normalization",
    )
    reject("unbounded bank state", lambda: check_state(1.01, 1.0), "contractive")
    reject(
        "unsafe smoothing",
        lambda: check_trajectory(np.array([0.0, 1.0]), 240),
        "trajectory",
    )
    # A coherent Nyquist competitor must veto even though H2 is absent.
    n = 1960
    t = np.arange(n) / n
    base = np.cos(2 * np.pi * 8 * t)
    defect = -0.5 * base + 0.6 * (-1.0) ** np.arange(n)
    require(
        not inspect(moments(base, defect), np.ones(1), 0.5, False)[0],
        "Nyquist veto escaped",
    )
    rejected["Nyquist veto"] = "realized projection/energy gate"
    return rejected
