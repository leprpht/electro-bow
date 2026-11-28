"""Compile or import one coherent defective offline producer and run the qualification predicate."""

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

from guide_qualification import digest
from i1_motion import OUT
from o2_response import require

# Source substitutions are localized to copies of executable code. The independent
# waveform, event and recovery predicates come from the unmodified complete exit.
CASES = [
    (
        "always-neutral fourth",
        "i1_stream.cpp",
        "if(probe==1) phase=NAN;",
        "d[3]=0.; if(probe==1) phase=NAN;",
        "paths",
        "constant delivery final wet",
    ),
    (
        "Review 8.1 fourth source",
        "i1_motion_bank.cpp",
        "if (std::fwrite",
        "result[3] = 0.0;\n        if (std::fwrite",
        "paths",
        "fourth-bank correction",
    ),
    (
        "independent gain",
        "i1_stream.cpp",
        "gain==0?direct:direct+gain*delta",
        "gain==0?direct+delta:direct+delta",
        "paths",
        "unsafe independent gain",
    ),
    (
        "incorrect bank coefficient",
        "i1_stream.cpp",
        "d[j]+coeff*taper[k]",
        "d[j]+.97*coeff*taper[k]",
        "paths",
        "fourth-bank correction",
    ),
    (
        "bypassed slew",
        "i1_stream.cpp",
        "scale+=std::clamp(target-scale,-step,step);",
        "scale=step>0?target:0.;",
        "paths",
        "independent scheduled slew",
    ),
    (
        "disabled realized veto",
        "i1_stream.cpp",
        "ok && available && live?",
        "ok && available?",
        "paths",
        "delivery veto recovery",
    ),
    (
        "wrong phase progression",
        "i1_stream.cpp",
        "INFINITY:tau*f/fs",
        "INFINITY:tau*f/fs*1.00001",
        "qr",
        "packed cross moments",
    ),
    (
        "wrong candidate schedule",
        "i1_stream.cpp",
        "trial=0;",
        "trial=1;",
        "qr",
        "candidate schedule",
    ),
    (
        "expiry bypass",
        "i1_clock.py",
        "tick - anchor >= acquire_ticks",
        "tick - anchor > acquire_ticks",
        "events",
        "elapsed acquisition eligibility",
    ),
    (
        "duplicate gesture acceptance",
        "i1_clock.py",
        "if kind in seen or",
        "if (kind in seen and event != Gesture.HOLD) or",
        "events",
        "duplicate event rejection",
    ),
    (
        "stale publication",
        "i1_clock.py",
        'elif action == "hold" and owner and owner == live:\n                job = None',
        'elif action == "hold" and owner and owner == live:\n                pass',
        "lifecycle",
        "stale feature publication",
    ),
    (
        "wrong physical epoch",
        "i1_clock.py",
        "epoch = last_epoch = launch.epoch",
        "epoch = last_epoch = launch.epoch + 1",
        "lifecycle",
        "physical epoch ownership",
    ),
    (
        "phase poison treated as veto",
        "i1_stream.cpp",
        "if(probe==1) phase=NAN;",
        "if(probe==1) phase=NAN; if(!std::isfinite(phase)) { phase=0; ready=0; veto=true; }",
        "faults",
        "lane-local numerical ownership phase",
    ),
    (
        "stereo abort",
        "i1_joined.py",
        "except NumericalFault as err:\n",
        "except NumericalFault as err:\n                raise\n",
        "faults",
        "lane numerical fault escaped stereo",
    ),
    (
        "double cache envelope",
        "i1_joined.py",
        "cache[1] * scale",
        "cache[0] * cache[1] * scale",
        "faults",
        "cached wet product fade",
    ),
    (
        "wrong M5 denominator",
        "o2_records.py",
        'require(taps == record["taps"], "canonical Rust tap schema")',
        'taps = record["taps"]',
        "producer",
        "escaped negative: substituted original M5",
    ),
    (
        "shifted wet pair",
        "o2_records.py",
        "sample_hash(corrected.samples) == record.hashes[index]",
        "True",
        "producer",
        "escaped negative: rolled wet valid tags",
    ),
]


def gate(kind):
    from i1_motion import build_bank
    from i1_stream import NumericalFault
    from i1_stream_checks import (
        collisions,
        differential,
        fault_cases,
        lifecycle_behavior,
        path_behavior,
    )
    from o2_qualification import negative_controls

    (OUT / "joined").mkdir(exist_ok=True, parents=True)
    if kind == "paths":
        build_bank()
        path_behavior()
    elif kind == "faults":
        try:
            fault_cases()
        except NumericalFault as err:
            raise ValueError("lane numerical fault escaped stereo") from err
    elif kind == "qr":
        differential(47001, "bend")
    elif kind == "events":
        collisions()
    elif kind == "lifecycle":
        lifecycle_behavior()
    elif kind == "producer":
        negative_controls()


def execute():
    records = {}
    root = Path(__file__).resolve().parent
    for name, filename, original, mutated, group, predicate in CASES:
        with TemporaryDirectory(dir=OUT.parent) as tmp:
            directory = Path(tmp) / "validation"
            shutil.copytree(
                root, directory, ignore=shutil.ignore_patterns("__pycache__")
            )
            path = directory / filename
            source = path.read_text()
            if filename.endswith(".cpp"):
                tokens = re.findall(r"[a-zA-Z0-9_]+|[^\s]", original)
                pattern = r"\s*".join(map(re.escape, tokens))
                source, count = re.subn(pattern, lambda _: mutated, source)
            else:
                count = source.count(original)
                source = source.replace(original, mutated)
            require(count == 1, "mutation site " + name)
            if name == "shifted wet pair":
                source = source.replace(
                    "sample_hash(corrected_off.samples) == record.hashes[0]", "True"
                )
            path.write_text(source)
            cmd = [
                sys.executable,
                str(directory / "i1_qualification.py"),
                "--challenge-first",
                group,
            ]
            env = dict(
                os.environ,
                ELECTROBOW_I1_OUT=str(Path(tmp) / "output"),
                PYTHONPATH=str(directory),
            )
            run = subprocess.run(cmd, env=env, capture_output=True, text=True)
            status_path = Path(tmp) / "output/qualification.json"
            status = json.loads(status_path.read_text()) if status_path.exists() else {}
            require(
                run.returncode == 1
                and status.get("status") == "failed"
                and status.get("predicate_group") == group
                and status.get("error_type") == "ValueError"
                and status.get("error") == predicate,
                f"{name} unexpected exit {run.returncode}, status {status}, stderr {run.stderr[-500:]}",
            )
            actual = dict(
                predicate=status["error"],
                run_id=status["run_id"],
                status=status["status"],
            )
            records[name] = dict(
                source=filename,
                mutation=[original, mutated],
                mutant_sha256=digest(path),
                invocation=["i1_qualification.py", "--challenge-first", group],
                exit=run.returncode,
                expected=predicate,
                observed=actual,
            )
    return records


if __name__ == "__main__":
    (OUT / "source-mutants.json").write_text(json.dumps(execute(), indent=2) + "\n")
