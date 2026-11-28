"""Fault injection at the actual report runner, including an uncatchable process exit."""

import json
import os
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import o2_qualification as runner
from o2_coverage import grade_complete
from o2_response import require


def main():
    root = runner.OUT.parent
    evidence = json.loads((runner.OUT / "results.json").read_text())
    grade_complete(evidence)
    checks = {}
    with TemporaryDirectory(dir=root) as tmp, patch.object(runner, "OUT", Path(tmp)):
        path = Path(tmp) / "results.json"
        (Path(tmp) / "traces").symlink_to(
            (root / "o2" / "traces").resolve(), target_is_directory=True
        )
        for test in (
            "ordinary",
            "anchor",
            "serialization",
            "grading interrupt",
            "publication interrupt",
            "inventory",
        ):
            runner.publish(evidence)

            def produce(result, test=test):
                identity = {k: result[k] for k in ("head", "run_id")}
                result.update(
                    evidence, status="running", neutral_candidate_passed=False
                )
                result.update(identity)
                if test == "ordinary":
                    raise RuntimeError("injected")
                if test == "serialization":
                    result["extra"] = float("nan")
                if test == "anchor":
                    result["v17"] = dict(result["v17"])
                    key = "48000/196/0.1/1.8/0.5"
                    result["v17"][key] = dict(
                        result["v17"][key],
                        failure_reason="ineligible M5 bridge anchor",
                        measured_f0=24000.0,
                        spectral=None,
                    )
                if test == "inventory":
                    result.pop("sweep")

            actual_grade = runner.grade_measurements

            def grade(result, test=test, actual_grade=actual_grade):
                if test == "grading interrupt":
                    raise KeyboardInterrupt()
                # Timing receipts belong to the original measured invocation. Use its
                # identity solely inside this injected grading call.
                actual_grade(dict(result, run_id=evidence["run_id"]))

            publish = runner.publish

            def writer(result, name="results.json", test=test, publish=publish):
                if test == "publication interrupt" and result.get("status") == "passed":
                    with patch.object(os, "replace", side_effect=KeyboardInterrupt()):
                        publish(result, name)
                publish(result, name)

            with (
                patch.object(runner, "qualify", produce),
                patch.object(runner, "grade_measurements", grade),
                patch.object(runner, "publish", writer),
            ):
                try:
                    runner.main()
                except (RuntimeError, ValueError, KeyboardInterrupt) as err:
                    row = json.loads(path.read_text())
                    require(
                        row["status"]
                        == ("interrupted" if "interrupt" in test else "failed")
                        and not row["neutral_candidate_passed"]
                        and row["run_id"] != evidence["run_id"],
                        "unsuccessful report",
                    )
                    checks[test] = [row["status"], str(err)]
                else:
                    raise ValueError("published defective report")
        env = dict(os.environ, ELECTROBOW_O2_OUT=tmp)
        code = (
            "import os,o2_qualification as q; q.qualify=lambda r:os._exit(43); q.main()"
        )
        require(
            subprocess.run(
                [sys.executable, "-c", code], env=env, check=False
            ).returncode
            == 43,
            "abrupt exit",
        )
        row = json.loads(path.read_text())
        require(
            row["status"] == "running" and not row["neutral_candidate_passed"],
            "interrupted inventory",
        )
        for r, reason in (
            (row, "unsuccessful publication"),
            (evidence, "stale run identity"),
        ):
            try:
                grade_complete(r)
            except ValueError as err:
                require(reason in str(err), str(err))
            else:
                raise ValueError(reason)
        checks["uncatchable termination"] = (
            "running; prior success rejected by current invocation"
        )
    (root / "publication-controls.json").write_text(json.dumps(checks, indent=2) + "\n")


if __name__ == "__main__":
    main()
