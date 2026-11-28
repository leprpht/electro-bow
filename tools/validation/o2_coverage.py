"""Frozen inventories and applicable gates, independent of the emitted rows."""

import csv
import json
import math
from copy import deepcopy
from functools import lru_cache
from itertools import product
from numbers import Real
from pathlib import Path
from statistics import median

import guide_qualification as guide
import numpy as np
from o2_response import coefficients, reference, require

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
        require(
            row.get("failure_reason") is None and isinstance(row.get("spectral"), dict),
            key + " ineligible M5 bridge anchor or wet spectrum",
        )
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
                hz = p["hz"]
                measured = complex(finite(p["real"], "real"), finite(p["imag"], "imag"))
                expected = reference(fs, hz, d)
                require(abs(measured - expected) <= 1e-9, "complex probe")
                # Both taps are actual DFT sums of the Rust unit impulse.
                a, l = (complex(*p[key]) for key in ("a", "l"))
                blocked = reference(fs, hz, 40) * 0.85 / math.sqrt(2)
                u = math.tan(math.pi * hz / fs) / math.tan(math.pi * 3200 / fs)
                load = 1 / complex(1 - u * u, math.sqrt(2) * u)
                require(
                    abs(a - blocked) <= 1e-9
                    and abs(l - blocked * load) <= 1e-9
                    and abs(measured - (l + 0.025 * d * (a - l)) * math.sqrt(2) / 0.85)
                    <= 1e-9,
                    "impulse tap consistency",
                )
                if hz:
                    require(
                        abs(p["load_magnitude"] - abs(l / a)) <= 1e-12
                        and abs(p["load_magnitude"] - abs(load)) <= 1e-9,
                        "load magnitude",
                    )
                else:
                    require(p["load_magnitude"] is None, "ineligible load ratio")
                eligible = hz != 0 and (hz != fs / 2 or d != 0)
                if eligible:
                    delay = (complex(*p["time_moment"]) / measured).real
                    ref_delay = -np.angle(
                        reference(fs, hz + 0.001, d) / reference(fs, hz - 0.001, d)
                    ) / (0.002 * 2 * np.pi)
                    require(
                        abs(p["phase_deg"] - np.angle(measured, deg=True)) <= 1e-9,
                        "phase",
                    )
                    require(
                        abs(p["delay_s"] - delay) < 1e-12
                        and abs(p["delay_error_s"] - abs(delay - ref_delay)) < 1e-12
                        and 0 <= p["delay_error_s"] < 1e-8,
                        "delay",
                    )
                else:
                    require(
                        all(
                            p[k] is None
                            for k in ("phase_deg", "delay_s", "delay_error_s")
                        ),
                        "ineligible spectral probe",
                    )
            peak_hz, re, im = resp["worst_bin"]
            require(
                abs(
                    abs(complex(re, im) - reference(fs, peak_hz, d))
                    - resp["max_complex_error"]
                )
                < 1e-14,
                "worst FFT bin",
            )
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
            first, peak = step_reference(fs, d)
            require(
                abs(step["first"] - first) <= 1e-9 and abs(step["peak"] - peak) <= 1e-9,
                "long step entry/peak",
            )
        require(
            np.allclose(row["coefficients"], coefficients(fs), rtol=0, atol=1e-14)
            and np.allclose(
                row["poles"],
                abs(np.roots([1, *coefficients(fs)[4:]])),
                rtol=0,
                atol=1e-14,
            ),
            "coefficient or pole mismatch",
        )


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


@lru_cache(None)
def step_reference(fs, d):
    from o2_bench import oracle

    _h, a, l = oracle(fs, np.ones(math.ceil(fs * 0.1)))
    q = l + 0.025 * d * (a - l)
    return q[0], max(abs(q))


REQUIRED = {
    "electrical",
    "lifecycle",
    "local_timing",
    "high_band",
    "onset_release",
    "inputs",
    "replay",
    "tool_hashes",
    "head",
    "renderer_sha256",
    "v17",
    "negatives",
    "nominal_mono_input",
    "corners",
    "injected_vectors_dB",
    "witness",
    "sweep",
    "failed_predicates",
}
WORKLOADS = ("silence", "near-denormal", "o2", "guide-o1", "guide-o1-o2")
EXTRA_CONTROLS = {
    "removed " + s
    for s in ("local_timing", "high_band", "onset_release", "inputs", "replay")
} | {
    "complex cutoff",
    "wrong phase",
    "wrong load magnitude",
    "fabricated Nyquist phase/delay",
    "flushed subnormal",
    "voiced off-target anchor",
    "off only",
    "factory only",
    "joint shift",
    "wrong epoch",
    "wrong reference",
    "wrong measurement window",
    "incorrect underlying factory bytes",
    "wrong source",
    "incorrect count",
    "self-attested shift",
    "rolled wet valid tags",
    "substituted original M5",
    "fabricated factory sweep",
    "cross-rate timing",
}
NEGATIVES.update(EXTRA_CONTROLS)


def timing_ratio(records):
    keys(
        [(k, b) for k, b, ns, h in records],
        product(WORKLOADS, range(1, 6)),
        "timing batches",
    )
    groups = {k: [] for k in WORKLOADS}
    for k, b, ns, h in records:
        require(finite(ns, "time") > 0, "timing duration")
        finite(h, "terminal h")
        if k == "silence":
            require(h == 0, "silence control")
        if k == "near-denormal":
            require(0 < abs(h) < np.finfo(float).tiny, "active subnormal workload")
        groups[k].append(ns)
    return median(groups["near-denormal"]) / median(groups["silence"])


@lru_cache(None)
def diagnostic_reference():
    from o2_qualification import acoustic

    return acoustic(make_audio=False)


@lru_cache(None)
def provenance():
    import o1_qualification as o1
    from o2_audio import FIX, OUT, physical_key

    manifest = json.loads((guide.OUT / "manifest.json").read_text())["cases"]
    observed = {}
    for path in (o1.HASHES, FIX / "o2_sha256.tsv"):
        for line in path.read_text().splitlines():
            digest, name = line.split()
            observed[Path(name).stem] = digest
    kept = set(observed) | {physical_key(fs, 1318.51) for fs in RATE}
    traces = {name: guide.digest(OUT / "traces" / (name + ".f64le")) for name in kept}
    with (FIX / "o2_replay_sha256.tsv").open() as stream:
        reviewed = {r["case"]: r for r in csv.DictReader(stream, delimiter="\t")}
    keys(reviewed, set(manifest) | set(traces), "reviewed replay")
    return manifest, observed, traces, reviewed


def replay_grade(result):
    from o2_audio import physical_key

    manifest, observed, traces, reviewed = provenance()
    rows = result["replay"]
    keys(rows, set(manifest) | set(traces), "physical replay")
    for name, row in rows.items():
        n = manifest[name]["successful_ticks"] if name in manifest else 3 * 48000
        require(row["ticks"] == row["commits"] == row["calls"] == n, "replay clock")
        require(
            (row["original_sha256"], row["observed_sha256"], row.get("sha256", "-"))
            == tuple(
                reviewed[name][k] for k in ("guide_M5", "guide_M5_O1", "retained_O2")
            ),
            "reviewed replay hashes",
        )
        if name in manifest:
            require(
                row["original_sha256"] == manifest[name]["raw_sha256"], "original M5"
            )
        if name in observed:
            require(row["observed_sha256"] == observed[name], "O1 provenance")
        keys(row["max_error"], map(str, DEPTH), "replay depths")
        for errors in row["max_error"].values():
            require(
                len(errors) == 4 and all(0 <= e <= 1e-9 for e in errors),
                "replay differential",
            )
        if name in traces:
            require(
                row["sha256"] == traces[name]
                and len(row["stage06_max_FS"]) == 3
                and all(0 <= v <= 1e-9 for v in row["stage06_max_FS"]),
                "O2 provenance",
            )
    inputs = {
        f"{fs}/{f}/{v}/{r}": traces[physical_key(fs, f, v, r)]
        for fs, f, v, r in STATIONARY
    }
    inputs.update(
        {f"lift/{f}": traces[physical_key(48000, f, kind="lift")] for f in (82.41, 440)}
    )
    require(result["inputs"] == inputs, "input provenance")
    for row in result["corners"]:
        require(
            row["trace"] == traces[physical_key(48000, 440, row["V"], row["R"])],
            "corner provenance",
        )


def diagnostics(result):
    times = (
        (0, 1 / 48000),
        (0, 0.01),
        (0, 0.05),
        (0.5, 1),
        (0.975, 1),
        (1, 1.025),
        (1.025, 1.05),
        (1.1, 1.15),
        (2, 2.05),
        (6, 7),
    )
    keys(
        result["onset_release"],
        (f"{f}/{d}/{a}-{b}" for f, d, (a, b) in product((82.41, 440), DEPTH, times)),
        "transient",
    )
    keys(
        result["high_band"],
        (
            f"{fs}/{k}"
            for fs, k in product(RATE, ("raw", "blocked", "loaded", "factory", "full"))
        ),
        "high band",
    )
    # Remeasure the declared physical taps, including every eligible phase,
    # residual denominator, absolute level, null and high-band reference delta.
    for section in ("onset_release", "high_band"):
        for k, r in result[section].items():
            require("failure_reason" not in r, f"{k} {r.get('failure_reason')}")
    ref = diagnostic_reference()
    for key in ("high_band", "onset_release", "v17"):
        require(result[key] == ref[key], "tap-derived " + key)
    timing = result["local_timing"]
    require(
        (timing["coefficient_bytes"], timing["lane_bytes"]) == (48, 32), "timing sizes"
    )
    keys(timing["rates"], ("32000", "48000", "192000"), "timing rates")
    from o2_bench import measured_timing

    for rate, data in timing["rates"].items():
        ratio = timing_ratio(data["records"])
        require(
            data == measured_timing(int(rate), result["run_id"]),
            "measured timing record",
        )
        require(ratio == data["near_denormal_ratio"] and ratio <= 2, "timing bound")


@lru_cache(None)
def electrical_reference(fs):
    from o2_bench import check_rate

    return check_rate(fs)


@lru_cache(None)
def provenance_files():
    from o2_qualification import BINARY, tool_hashes

    return tool_hashes(), guide.digest(BINARY)


@lru_cache(None)
def sweep_reference(executable_hash):
    from o2_qualification import sweep

    return sweep(48000)


def grade(result):
    require(
        not REQUIRED - result.keys(),
        f"missing evidence {sorted(REQUIRED - result.keys())}",
    )
    require(not result.get("errors") and not result.get("error"), "recorded failure")
    expected_failures = {
        f"stress/{f}/{vector}": (
            ["fundamental"] if vector == "inversion" else ["level", "coloration"]
        )
        for f, vector in product((82.41, 196, 440), VECTORS[1:])
    }
    require(
        result["nominal_mono_input"]["sha256_f32le"]
        == "cfc6d9342aa505dd7a53bd0ae531f717bec007042918ada9949a2117342a3d09",
        "nominal pluck hash",
    )
    source_hashes, binary_hash = provenance_files()
    require(
        result["tool_hashes"] == source_hashes
        and result["renderer_sha256"] == binary_hash
        and len(result["head"]) == 40
        and all(c in "0123456789abcdef" for c in result["head"]),
        "replay provenance",
    )
    pluck = result["nominal_mono_input"]
    require(
        (pluck["fs"], pluck["frames"], pluck["eligible_frames"])
        == (48000, 211200, 144000)
        and abs(pluck["peak_FS"] - 0.35) < 1e-7,
        "nominal input clock/level",
    )
    require(
        result["injected_vectors_dB"]
        == {
            "neutral": [0] * 12,
            "upper": [5] * 12,
            "lower": [-5] * 12,
            "inversion": [-5, 5] + [0] * 10,
        },
        "stress vector inventory",
    )
    stationary(result["v17"])
    require(
        result["failed_predicates"] == expected_failures, "recorded failures differ"
    )

    electrical(result["electrical"])
    for fs in ELECTRICAL_RATES:
        require(
            result["electrical"][str(fs)] == electrical_reference(fs),
            f"Rust tap replay at {fs}",
        )
    diagnostics(result)
    replay_grade(result)
    sweeps(result["sweep"])
    require(result["sweep"] == sweep_reference(binary_hash), "tap-derived sweep")
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


def grade_measurements(result):
    grade(result)
    grade_controls(result["negatives"])
    from o2_qualification import negative_controls, executed_controls

    executed = {**negative_controls(), **coverage_negatives(result)}
    require(
        result["negatives"] == executed
        and result.get("executions") == executed_controls(result, executed),
        "executed negative-control predicates",
    )
    require(
        result["all_tested_vectors_passed"] is (not bool(result["failed_predicates"])),
        "inconsistent qualification summary",
    )


def reset_references():
    for fn in (
        provenance_files,
        provenance,
        diagnostic_reference,
        electrical_reference,
        sweep_reference,
    ):
        fn.cache_clear()


def grade_complete(result):
    from o2_qualification import OUT

    current = json.loads((OUT / "current-run.json").read_text())
    require(
        all(current[k] == result.get(k) for k in ("run_id", "head")),
        "stale run identity",
    )
    reset_references()
    require(
        result.get("status") == "passed"
        and result.get("neutral_candidate_passed") is True,
        "unsuccessful publication",
    )
    grade_measurements(result)


def grade_controls(controls):
    keys(controls, NEGATIVES, "negative controls")
    require(
        all(isinstance(msg, str) and msg for msg in controls.values()),
        "missing rejection",
    )


def execute_control(name, invocation, expected):
    # In-process predicate ABI. Build errors and unrelated runtime exceptions
    # propagate. Both pass and rejection return an actual status to this caller.
    def invoke():
        try:
            invocation()
        except ValueError as error:
            return 1, str(error)
        return 0, None

    code, detail = invoke()
    require(code == 1, "escaped negative: " + name)
    require(expected in detail, f"{name} unrelated predicate: {detail}")
    return detail


def coverage_negatives(result):
    controls = {}

    # Start with an accepted, complete report. Send each defective copy
    # through the same aggregate that determines the maintained runner's exit.
    grade(result)

    def reject(name, section, data, expected):
        controls[name] = execute_control(
            name, lambda: grade({**result, section: data}), expected
        )

    key = "48000/196/0.1/1.8/0.5"
    missing = dict(result["v17"])
    del missing[key]
    reject("omitted required row", "v17", missing, "V17 inventory")
    sweep = deepcopy(result["sweep"])
    defect = {"hz": 3000.0, "dB": 8.0, "width_hz": 17.0}
    sweep["0"].update(maximum=defect, maximum_dB=8.0, peaks_above_6dB=[defect])
    reject("reported sweep peak", "sweep", sweep, "sweep above +6 dB")
    reject(
        "duplicate corner",
        "corners",
        result["corners"] + [result["corners"][0]],
        "corners inventory",
    )
    for name in ("missing predicate", "null measurement", "nonfinite measurement"):
        rows = dict(result["v17"])
        rows[key] = deepcopy(rows[key])
        if name == "missing predicate":
            del rows[key]["passes"]["factory_eta"]
        else:
            rows[key]["factory_difference_over_off"] = (
                None if name == "null measurement" else math.nan
            )
        reject(
            name,
            "v17",
            rows,
            "inventory" if name == "missing predicate" else "missing/nonfinite",
        )

    def defect(name, alter, reason):
        copy = deepcopy(result)
        alter(copy)
        controls[name] = execute_control(name, lambda: grade(copy), reason)

    for section in ("local_timing", "high_band", "onset_release", "inputs", "replay"):
        defect(
            "removed " + section,
            lambda x, section=section: x.pop(section),
            "missing evidence",
        )
    for name, idx, changes, reason in (
        ("complex cutoff", 6, {"real": 123.0, "imag": -456.0}, "complex probe"),
        ("wrong phase", 6, {"phase_deg": 999.0}, "phase"),
        ("wrong load magnitude", 6, {"load_magnitude": -100.0}, "load magnitude"),
        (
            "fabricated Nyquist phase/delay",
            -1,
            {"phase_deg": 900.0, "delay_s": 100.0},
            "phase",
        ),
    ):
        defect(
            name,
            lambda x, idx=idx, changes=changes: x["electrical"]["48000"]["response"][
                "0.5"
            ]["points"][idx].update(changes),
            reason,
        )

    defect(
        "fabricated factory sweep",
        lambda x: x["sweep"]["0.5"].update(
            maximum={"hz": 65.0, "dB": -100.0, "width_hz": None},
            maximum_dB=-100.0,
            complex_error=0.0,
            peaks_above_6dB=[],
        ),
        "tap-derived sweep",
    )
    defect(
        "cross-rate timing",
        lambda x: x["local_timing"]["rates"].update(
            {"192000": x["local_timing"]["rates"]["32000"]}
        ),
        "measured timing record",
    )

    def flush(x):
        for row in x["local_timing"]["rates"]["48000"]["records"]:
            if row[0] == "near-denormal":
                row[3] = 0.0

    defect("flushed subnormal", flush, "active subnormal")
    from o2_qualification import v17_row

    t = np.arange(48000) / 48000
    x = np.sin(2 * np.pi * 196 * t)
    bridge = 0.5 * x + 0.6 * (-1.0) ** np.arange(len(t))
    bad = v17_row(x, x, x, x, bridge, x, x, 48000, 196, 0.5, operation="neutral")
    json.dumps(bad, allow_nan=False)
    defect(
        "voiced off-target anchor",
        lambda r: r["v17"].update({key: bad}),
        "ineligible M5",
    )
    return controls
