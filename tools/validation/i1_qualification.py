"""Stage 08.2 complete offline exit. Reconstruct both measurements and artifacts on fresh paths."""

import argparse
import io
import json
import os
import subprocess
import sys
from copy import deepcopy
from hashlib import sha256
from pathlib import Path
from tempfile import TemporaryDirectory
from uuid import uuid4

import numpy as np
from guide_qualification import digest
from i1_acceptance import stationary_inventory
from i1_motion import OUT, STRESS
from i1_safety import check_trajectory
from o2_bench import OUT as ELECTRICAL
from o2_coverage import RATE, grade_complete, keys
from o2_response import require

REPORTS = (
    "i1-results",
    "stationary",
    "motion",
    "joined",
    "integrity",
    "stream-checks",
    "layout",
    "listening-manifest",
    "clock",
    "source-clock",
    "listening-key",
)
SITES = (
    "candidate",
    "moment",
    "bank",
    "wet",
    "conversion",
    "delivery-state",
    "pre-guide",
    "post-o2",
)
CASES = [f"{k}-{fs}" for fs in RATE for k in ("steady", "vibrato", "bend")] + [
    k + "-48000" for k in STRESS
]


def relative_files():
    return (
        [
            "motion/" + case + ext
            for case in CASES
            for ext in (".f64le", ".json", ".npz")
        ]
        + [
            "joined/" + case + ext
            for case in ("lifted", "replacement", "fresh")
            for ext in ("-48000-18500.f64le", "-48000-18500.json")
        ]
        + ["joined/uneven-47001-141003" + ext for ext in (".json", ".f64le")]
        + ["joined/" + key + ".npz" for key in (*SITES, "reference", "uneven")]
        + [
            "listening.zip",
            "stage-08-2-listening.zip",
            "revised-held.npz",
            "bow-controls.f64le",
            "retune.f64le",
            "long-hold.f64le",
        ]
    )


def render(root):
    env = dict(
        os.environ,
        ELECTROBOW_I1_OUT=str(root),
        ELECTROBOW_O2_OUT=str(ELECTRICAL),
        OPENBLAS_NUM_THREADS="1",
        VECLIB_MAXIMUM_THREADS="1",
    )
    root.mkdir(parents=True, exist_ok=True)
    for name in (
        "i1_study",
        "i1_motion",
        "i1_stationary",
        "i1_joined",
        "i1_integrity_checks",
        "i1_stream_checks",
        "i1_layout",
        "i1_listening",
    ):
        args = ["--historical-measurements"] if name == "i1_study" else []
        with (root / (name + ".log")).open("w") as log:
            subprocess.run(
                [sys.executable, Path(__file__).with_name(name + ".py"), *args],
                env=env,
                stdout=log,
                stderr=subprocess.STDOUT,
                check=True,
            )


def snapshot(root):
    return (
        {key: json.loads((root / (key + ".json")).read_text()) for key in REPORTS},
        {key: digest(root / key) for key in relative_files()},
    )


def grade(candidate, reference):
    reports, files = candidate
    measured, artifacts = reference
    keys(reports, REPORTS, "required reports")
    keys(files, relative_files(), "sample artifacts")
    for k, v in artifacts.items():
        require(files[k] == v, "waveform/clock replay " + k)
    for k, v in measured.items():
        require(reports[k] == v, "remeasured " + k)
    old, steady, motion = (
        reports["i1-results"],
        reports["stationary"],
        reports["motion"],
    )
    keys(steady["rows"], stationary_inventory(), "revised stationary")
    keys(motion["rows"], CASES, "prescribed motion")
    require(old["passed"] and not old["failures"], "historical stationary grid")
    require(
        all(row["eligible"] and not row["failures"] for row in steady["rows"].values())
        and steady["passed"],
        "stationary or upstream V17",
    )
    for fs in RATE:
        require(
            motion["rows"][f"steady-{fs}"]["prerequisite"]["passed"],
            "upstream prerequisite",
        )
        vib = motion["rows"][f"vibrato-{fs}"]
        require(
            vib["witness"]["passed"]
            and vib["delivery_witness"]["passed"]
            and not vib["old_witness"]["passed"],
            "moving identity witness",
        )
    for name, row in motion["rows"].items():
        for block in row["monitor"]:
            if block["selection_tick"] is not None and not (
                block["available"] and block["realized_ok"]
            ):
                require(block["veto"] and block["scale"] == 0, "missed veto " + name)
        if name.startswith(("vibrato-", "bend-")):
            require(
                all(b["realized_ok"] for b in row["monitor"]),
                "realized motion safety " + name,
            )
    require(motion["motion_pass"], "motion usefulness")
    keys(reports["joined"]["faults"], SITES, "stereo faults")


def sample_checks(root):
    from i1_stream_checks import motion_samples

    report = json.loads((root / "motion.json").read_text())
    for case in CASES:
        fs = int(case.rsplit("-", 1)[1])
        motion_samples(
            root,
            case,
            report["rows"][f"steady-{fs}"]["reference"],
            report["rows"][case]["monitor"],
        )
        with np.load(root / "motion" / (case + ".npz")) as z:
            q, d, wet, a = z["q"], z["d"], z["wet"], z["scale"]
            n = (7 if case.startswith("lift-") else 3) * fs
            require(
                q.shape == wet.shape == d.shape == (n, 4) and a.shape == (n,),
                "sample coverage",
            )
            raw = np.memmap(
                root / "motion" / (case + ".f64le"), mode="r", dtype="<f8"
            ).reshape(n, 30)
            require(np.array_equal(q[:, :3], raw[:, 25:28]), "wrong carrier bytes")
            expected = q.copy()
            i = a != 0
            expected[i] += a[i, None] * d[i]
            require(
                np.array_equal(wet.view("u8"), expected.view("u8")),
                "realized wet trajectory",
            )
            check_trajectory(a, round(0.005 * fs))


def mutations(candidate, reference, root):
    reports, files = candidate
    controls = {}

    def reject(name, changed):
        try:
            grade(changed, reference)
        except ValueError as err:
            controls[name] = str(err)
        else:
            raise ValueError("escaped " + name)

    k = "48000/196/0.1/1.8/opposed"
    for name, edit in (
        (
            "coherent residual",
            lambda r: r["stationary"][k]["depths"][1].update(
                factory_difference_over_off=-240.0, factory_wet_difference_rms_FS=0.0
            ),
        ),
        (
            "empty harmonics",
            lambda r: r["stationary"][k]["depths"][0].update(
                harmonics={h: [] for h in ("peak_FS", "phase_deg", "dBr")}
            ),
        ),
        (
            "fabricated sweep",
            lambda r: r["measured_sweep"]["weak-H1/0.5"].update(
                peak={"hz": -1, "dB": -200, "width_hz": None}, complex_error=0
            ),
        ),
        ("truncated timeline", lambda r: r["dynamic"]["vibrato"]["timeline"].pop()),
        (
            "false controls",
            lambda r: r.update(negatives=dict.fromkeys(r["negatives"], False)),
        ),
        ("omitted source", lambda r: r["real"]["takes"].pop("techs-P1-60")),
        (
            "incorrect denominator",
            lambda r: r["stationary"][k]["depths"][0].update(final_over_M5=0),
        ),
    ):
        changed = deepcopy(reports["i1-results"])
        edit(changed)
        reject(name, ({**reports, "i1-results": changed}, files))
    m = deepcopy(reports["motion"])
    m["rows"].pop("vibrato-192000")
    reject("omitted motion", ({**reports, "motion": m}, files))

    def variant(label, key, edit):
        with np.load(root / key) as data:
            arrays = {k: data[k] for k in data.files}
        edit(arrays)
        b = io.BytesIO()
        np.savez(b, **arrays)
        reject(label, (reports, {**files, key: sha256(b.getbuffer()).hexdigest()}))

    def unsafe(z):
        z["wet"] = z["q"] + z["d"]

    def bypass(z):
        n = round(1.35 * 48000)
        z["scale"][n:] = 0
        z["wet"][n:] = z["q"][n:]

    def stale(z):
        z["coefficients"][480:8000] = z["coefficients"][9000]
        z["samples"][480:8000, 8] = z["samples"][9000, 8]

    for name, key, edit in (
        ("unsafe independent gain", "motion/steady-48000.npz", unsafe),
        ("bypassed slew", "motion/veto-48000.npz", bypass),
        ("always neutral", "motion/vibrato-48000.npz", lambda z: z.update(wet=z["q"])),
        (
            "incorrect paired wet",
            "motion/bend-48000.npz",
            lambda z: z["wet"].__setitem__((slice(None), 1), z["wet"][:, 0]),
        ),
        (
            "hidden normalization",
            "motion/held-48000.npz",
            lambda z: z.update(wet=0.9 * z["wet"]),
        ),
        ("stale feature", "joined/reference.npz", stale),
        (
            "wrong physical epoch",
            "joined/reference.npz",
            lambda z: z["samples"].__setitem__((slice(800, 6000), 0), 5),
        ),
        (
            "incorrect clock",
            "joined/reference.npz",
            lambda z: z.update(samples=np.roll(z["samples"], 1, axis=0)),
        ),
    ):
        variant(name, key, edit)
    return controls


def run(existing=False, challenge_first=None):
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / "qualification.json"
    result = {
        "status": "running",
        "run_id": uuid4().hex,
        "revision": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
    }

    def publish():
        temp = path.with_suffix(".tmp")
        temp.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
        temp.replace(path)

    publish()
    try:
        from i1_mutants import gate

        groups = ("paths", "faults", "events", "lifecycle", "qr", "producer")
        order = (
            groups
            if challenge_first is None
            else (challenge_first, *(g for g in groups if g != challenge_first))
        )
        for group in order:
            result["predicate_group"] = group
            gate(group)
        result.pop("predicate_group")
        if challenge_first is not None:
            raise RuntimeError("source mutation escaped all producer predicates")
        upstream = json.loads((ELECTRICAL / "results.json").read_text())
        grade_complete(upstream)
        from i1_mutants import execute
        from i1_receipt_checks import exercise

        result["o2_run"] = upstream["run_id"]
        result["receipt_controls"] = exercise(upstream)
        if not existing:
            render(OUT)
        result["behavioral_controls"] = execute()
        sample_checks(OUT)
        candidate = snapshot(OUT)
        with TemporaryDirectory(dir=OUT.parent) as scratch:
            render(Path(scratch))
            reference = snapshot(Path(scratch))
            grade(candidate, reference)
            result["negative_controls"] = mutations(candidate, reference, OUT)
        result.update(
            status="passed",
            evidence=candidate[1],
            tools={
                p.name: digest(p)
                for p in sorted(Path(__file__).parent.glob("i1_*"))
                if p.is_file()
            },
            reports={k: digest(OUT / (k + ".json")) for k in REPORTS},
            engineering="passed",
            motion_usefulness="passed",
            controlled_identity="pending",
            independent_review="pending",
        )
    except BaseException as err:
        result.update(
            status="interrupted" if isinstance(err, KeyboardInterrupt) else "failed",
            error_type=type(err).__name__,
            error=str(err),
        )
        raise
    finally:
        publish()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify-existing", action="store_true")
    parser.add_argument(
        "--challenge-first",
        choices=("paths", "faults", "events", "lifecycle", "qr", "producer"),
        help=argparse.SUPPRESS,
    )
    args = parser.parse_args()
    run(args.verify_existing, args.challenge_first)
