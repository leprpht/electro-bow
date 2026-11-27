"""Frozen inventories and applicable gates, independent of the emitted rows."""

import csv
import math
from copy import deepcopy
from itertools import product
from numbers import Real
from pathlib import Path

import guide_qualification as guide
from o2_response import require

DEPTH = (0, 0.5, 1)
RATE = [32000, 44100, 48000, 88200, 96000, 176400, 192000]
ELECTRICAL_RATES = [
    *RATE,
    math.nextafter(32000.0, math.inf),
    44150.5,
    48100.0,
    math.nextafter(192000.0, 0.0),
]
STATIONARY = [
    (48000, f, 0.1, r) for f, r in product((82.41, 196, 440), (1.2, 1.8, 2.4))
]
STATIONARY += [(fs, 1318.51, 0.1, 1.8) for fs in RATE] + [(48000, 440, 0.16, 1.8)]
VECTORS = ("neutral", "upper", "lower", "inversion")
NEGATIVES = {
    "no prewarp",
    "wrong denominator sign",
    "unstable pole",
    "single rate fixed cutoff",
    "no DC blocker",
    "old reset state",
    "gain after filter input",
    "duplicate conversion",
    "duplicate trim",
    "excess detail",
    "per-render normalization",
    "silent offset",
    "tail phase from numerical residue",
    "zero RMS denominator",
    "wrong denominator self-reference",
    "double wet level",
    "dominant H2",
    "dominant Nyquist",
    "15.04 cents flat",
    "invalid bridge anchor",
    "colored final wet",
    "omitted required row",
    "reported sweep peak",
    "unaligned wet pair",
    "unmatched wet settings",
    "duplicate corner",
    "missing predicate",
    "null measurement",
    "nonfinite measurement",
}


class UniqueRows(dict):
    def __setitem__(self, key, value):
        require(key not in self, f"duplicate row {key}")
        super().__setitem__(key, value)


def keys(rows, expected, label):
    actual = set(rows)
    expected = set(expected)
    require(
        actual == expected and len(rows) == len(expected),
        f"{label} inventory missing={expected - actual} extra={actual - expected} count={len(rows)}",
    )


def finite(value, label):
    require(
        isinstance(value, Real)
        and not isinstance(value, bool)
        and math.isfinite(value),
        f"{label} missing/nonfinite value",
    )
    return value


def stationary(rows):
    expected = {
        f"{fs}/{f}/{v}/{r}/{d}": (fs, f, d, None)
        for fs, f, v, r in STATIONARY
        for d in DEPTH
    }
    for f, vector in product((82.41, 196, 440), VECTORS[1:]):
        expected[f"stress/{f}/{vector}"] = (48000, f, 0, vector)
    keys(rows, expected, "V17")
    for key, (fs, f, d, vector) in expected.items():
        row = rows[key]
        gates = {"bridge_anchor", "coloration", "level", "fundamental"}
        if d == 0.5:
            gates.update(("factory_eta", "factory_residual"))
        if (fs, f, d) == (32000, 1318.51, 0):
            gates.add("high_note_eta")
        flags = row["passes"]
        keys([g for g, v in flags.items() if v is not None], gates, key)
        for field in (
            "peak_FS",
            "rms_FS",
            "spatial_over_N",
            "load_over_blocked",
            "detail_over_neutral",
            "common_over_M5",
            "corrected_over_neutral",
            "final_over_M5",
            "measured_f0",
            "strongest_hz",
            "same_source_difference_rms_FS",
            "local_q_difference_over_off",
        ):
            finite(row.get(field), key + "/" + field)
        eta = finite(row["spectral"]["eta"], key + "/eta")

        def near(hz, f=f):
            return hz > 0 and (abs(hz - f) <= 2 or abs(1200 * math.log2(hz / f)) <= 15)

        passes = {
            "bridge_anchor": near(row["measured_f0"]),
            "fundamental": near(row["strongest_hz"]),
            "level": abs(row["final_over_M5"]) <= 3,
            "coloration": abs(row["corrected_over_neutral"]) <= 3,
        }
        if "high_note_eta" in gates:
            passes["high_note_eta"] = eta >= 0.85
        if d == 0.5:
            passes["factory_eta"] = eta >= 0.85
            finite(row.get("factory_wet_difference_rms_FS"), key + "/factory absolute")
            passes["factory_residual"] = (
                finite(
                    row.get("factory_difference_over_off"), key + "/factory residual"
                )
                <= -28
            )
        expected_failed = (
            {"fundamental"}
            if vector == "inversion"
            else {"level", "coloration"}
            if vector
            else set()
        )
        for gate in gates:
            require(
                bool(passes[gate]) == (gate not in expected_failed)
                and flags[gate] is bool(passes[gate]),
                f"{key} {gate} measurement or recorded predicate",
            )


def electrical(rows):
    keys(rows, map(str, ELECTRICAL_RATES), "electrical")
    for fs in ELECTRICAL_RATES:
        row = rows[str(fs)]
        require(
            len(row["coefficients"]) == 6
            and all(math.isfinite(x) for x in row["coefficients"]),
            "coefficient inventory",
        )
        require(
            0 < row["coefficients"][0] < 1
            and len(row["poles"]) == 2
            and all(0 < p < 1 for p in row["poles"]),
            "poles",
        )
        require(abs(finite(row["cutoff_hz"], "cutoff") - 3200) < 1e-6, "cutoff")
        require(
            row["impulse_frames"] == 2 ** math.ceil(math.log2(0.5 * fs)),
            "impulse duration",
        )
        keys(row["response"], map(str, DEPTH), "response depths")
        keys(row["long_step"], map(str, DEPTH), "step depths")
        keys(
            row["differential"],
            [
                f"{s}/{d}"
                for s in ("constant", "impulse", "step", "sine", "multitone")
                for d in DEPTH
            ],
            "time domain",
        )
        for errors in row["differential"].values():
            require(
                len(errors) == 4
                and all(0 <= finite(e, "tap error") <= 1e-9 for e in errors),
                "tap tolerance",
            )
        for d in DEPTH:
            resp = row["response"][str(d)]
            require(
                0 <= finite(resp["max_complex_error"], "complex") <= 1e-9
                and 0 <= finite(resp["terminal"], "impulse tail") < 1e-10,
                "impulse",
            )
            points = resp["points"]
            keys(
                [p["hz"] for p in points],
                [0, 20, 82.41, 196, 440, 1000, 3200, 6000, 12000, fs / 2],
                "frequency probes",
            )
            for p in points:
                for field in ("real", "imag", "load_magnitude"):
                    finite(p[field], field)
                eligible = p["hz"] != 0 and (p["hz"] != fs / 2 or d != 0)
                for field in ("phase_deg", "delay_s", "delay_error_s"):
                    if eligible:
                        finite(p[field], field)
                    else:
                        require(p[field] is None, "ineligible spectral probe")
                if eligible:
                    require(0 <= p["delay_error_s"] < 1e-8, "delay")
            step = row["long_step"][str(d)]
            require(
                step["frames"] == math.ceil(60 * fs)
                and step["terminal"][0] == 1
                and len(step["terminal"]) == 4,
                "actual long step duration/history",
            )
            require(
                all(abs(finite(v, "step state")) < 1e-10 for v in step["terminal"][1:]),
                "step tail",
            )
            require(
                0 <= finite(step["max_after_one_second"], "step output") < 1e-10,
                "step output",
            )
            finite(step["first"], "step entry")
            finite(step["peak"], "step peak")


def sweeps(rows):
    keys(rows, map(str, DEPTH), "sweep")
    for row in rows.values():
        p = row["maximum"]
        for k in ("hz", "dB"):
            finite(p.get(k), "sweep maximum " + k)
        if p["width_hz"] is not None:
            finite(p["width_hz"], "peak width")
        require(
            65 <= p["hz"] <= 4000
            and p["dB"] == finite(row["maximum_dB"], "sweep magnitude"),
            "sweep maximum",
        )
        require(not row["peaks_above_6dB"] and p["dB"] <= 6, "sweep above +6 dB")
        require(
            0 <= finite(row["complex_error"], "sweep transfer") <= 1e-9,
            "sweep transfer",
        )


def corners(rows):
    fields = ("V", "R", "noise", "vector", "G", "character", "internal_Out")
    keys(
        [tuple(row[k] for k in fields) for row in rows],
        product(
            (0.06, 0.16),
            (1.2, 2.4),
            DEPTH,
            VECTORS,
            (0.08, 1.4),
            (0.0, 1.0),
            (0.0, 2.0),
        ),
        "corners",
    )
    with (
        Path(__file__)
        .with_name("fixtures")
        .joinpath("o2_corner_peaks.tsv")
        .open() as stream
    ):
        frozen = UniqueRows()
        for ref in csv.DictReader(stream, delimiter="\t"):
            frozen[
                float(ref["V"]), float(ref["R"]), float(ref["depth"]), ref["vector"]
            ] = float(ref["unity_gain_wet_peak_FS"])
    keys(frozen, product((0.06, 0.16), (1.2, 2.4), DEPTH, VECTORS), "frozen wet peak")
    for row in rows:
        wet = frozen[tuple(row[k] for k in fields[:4])]
        pre = row["internal_Out"] * (row["G"] * wet if row["character"] else 0.35)
        require(abs(pre - row["peak"]) <= 1e-9, "Stage 06 corner peak")
        peak = finite(row["peak"], "corner peak")
        require(
            peak >= 0
            and abs(
                finite(row["demand_dB"], "corner demand")
                - max(0.0, guide.db(peak / 0.98))
            )
            < 1e-9,
            "corner demand",
        )
        if row["internal_Out"] == 0:
            require(peak == 0, "muted corner")
        elif row["character"] == 0:
            require(abs(peak - 0.7) < 1e-9, "dry corner")


# tick, disposition, physical commits, O2 calls, launches, physical faults, invalidations
LIFE = {
    "cleared-idle": [0, 0, 0, 0, 0, 0, 0],
    "launch": [2, 1, 1, 1, 1, 0, 1],
    "sustain": [150, 1, 149, 149, 1, 0, 1],
    "retune": [193, 1, 192, 192, 1, 0, 1],
    "continuation": [320, 1, 319, 319, 1, 0, 1],
    "physical-lift": [447, 1, 446, 446, 1, 0, 1],
    "harness-release": [512, 1, 511, 511, 1, 0, 1],
    "disposal": [555, 0, 553, 553, 1, 0, 2],
    "physical-fault": [700, 2, 653, 653, 2, 1, 4],
    "post-commit-fault": [923, 3, 777, 777, 3, 1, 6],
    "fresh-epoch": [1100, 1, 778, 778, 4, 1, 7],
    "dirty-relaunch": [1300, 1, 978, 978, 5, 1, 8],
    "prepared-zero": [4],
    "invalid-input": [1],
    "invalid-detail": [1],
    "nonfinite-history": [4],
    "prospective-overflow": [1],
    "repeated-dirty-reset": [2],
    "invalid-preparation": [4],
    "partition-fixed": [1536],
    "partition-random": [1536],
    "partition-events": [1536],
}


def grade(result):
    require(not result.get("errors") and not result.get("error"), "recorded failure")
    expected_failures = {
        f"stress/{f}/{vector}": (
            ["fundamental"] if vector == "inversion" else ["level", "coloration"]
        )
        for f, vector in product((82.41, 196, 440), VECTORS[1:])
    }
    require(
        result["failed_predicates"] == expected_failures, "recorded failures differ"
    )
    require(
        result["nominal_mono_input"]["sha256_f32le"]
        == "cfc6d9342aa505dd7a53bd0ae531f717bec007042918ada9949a2117342a3d09",
        "nominal pluck hash",
    )
    stationary(result["v17"])
    electrical(result["electrical"])
    sweeps(result["sweep"])
    corners(result["corners"])
    wanted = {f"{fs}/{k}": v for fs in (32000, 48000, 192000) for k, v in LIFE.items()}
    require(result["lifecycle"] == wanted, "lifecycle coverage or counters")
    target = {
        "common": 1.005418823,
        "trimmed": 0.710938468,
        "implied": 1.990627709,
        "demand_db": 6.155279,
    }
    keys(result["witness"], target, "gain witness")
    for k, v in target.items():
        require(
            abs(finite(result["witness"][k], k) - v)
            < (0.0005 if k == "demand_db" else 1e-6),
            "gain " + k,
        )
    keys(result["negatives"], NEGATIVES, "negative controls")
    require(
        all(isinstance(msg, str) and msg for msg in result["negatives"].values()),
        "missing rejection",
    )


def coverage_negatives(result):
    controls = {}

    def reject(name, fn, data):
        try:
            fn(data)
        except ValueError as err:
            controls[name] = str(err)
        else:
            raise ValueError("escaped " + name)

    key = "48000/196/0.1/1.8/0.5"
    missing = dict(result["v17"])
    del missing[key]
    reject("omitted required row", stationary, missing)
    sweep = deepcopy(result["sweep"])
    defect = {"hz": 3000.0, "dB": 8.0, "width_hz": 17.0}
    sweep["0"].update(maximum=defect, maximum_dB=8.0, peaks_above_6dB=[defect])
    reject("reported sweep peak", sweeps, sweep)
    reject("duplicate corner", corners, result["corners"] + [result["corners"][0]])
    for name in ("missing predicate", "null measurement", "nonfinite measurement"):
        rows = dict(result["v17"])
        rows[key] = deepcopy(rows[key])
        if name == "missing predicate":
            del rows[key]["passes"]["factory_eta"]
        else:
            rows[key]["factory_difference_over_off"] = (
                None if name == "null measurement" else math.nan
            )
        reject(name, stationary, rows)
    return controls
