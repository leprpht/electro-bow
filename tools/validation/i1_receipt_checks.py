"""Run the complete O2 reader against coherent cross-rate and negative-evidence replacements."""

from copy import deepcopy
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import o2_bench as bench
from guide_qualification import digest
from o2_coverage import grade_complete
from o2_response import require


def exercise(current):
    results = {}

    def reject(label, variant, expected):
        try:
            grade_complete(variant)
        except ValueError as err:
            require(str(err) == expected, f"{label} unrelated rejection: {err}")
            results[label] = dict(
                run_id=current["run_id"],
                invocation="grade_complete",
                exit=1,
                actual=str(err),
                expected=expected,
            )
        else:
            raise ValueError("accepted " + label)

    alt = deepcopy(current)
    alt["negatives"] = dict.fromkeys(alt["negatives"], "unexecuted control")
    reject("arbitrary negative claims", alt, "executed negative-control predicates")
    with TemporaryDirectory(dir=bench.OUT.parent) as tmp:
        directory = Path(tmp)
        for rate in (32000, 48000):
            filename = f"timing-{rate}.tsv"
            (directory / filename).symlink_to((bench.OUT / filename).resolve())
        lines = []
        for line in (bench.OUT / "timing-32000.tsv").read_text().splitlines():
            run, _, work, batch, count, elapsed, h = line.split()
            if work == "near-denormal":
                h = "-5e-324"
            lines.append(
                f"{run} 192000 {work} {batch} {6 * int(count)} {6 * int(elapsed)} {h}"
            )
        filename = directory / "timing-192000.tsv"
        filename.write_text("\n".join(lines) + "\n")
        alt = deepcopy(current)
        timing = alt["local_timing"]["rates"]
        row = deepcopy(timing["32000"])
        row["argv"][1] = "192000"
        row["sha256"] = digest(filename)
        for record in row["records"]:
            if record[0] == "near-denormal":
                record[-1] = -5e-324
        timing["192000"] = row
        with patch.object(bench, "OUT", directory):
            reject(
                "coherent rate frames elapsed terminal and report",
                alt,
                "executed timing stdout",
            )
    return results
