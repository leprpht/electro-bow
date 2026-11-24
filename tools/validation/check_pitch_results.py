"""Check frozen observations, including expected failures of the offline comparison."""

import argparse
import csv
import hashlib
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument(
    "--reference-hashes",
    action="store_true",
    help="Require byte-identical inputs on the pinned reference stack",
)
args = parser.parse_args()

with Path("build/spec-signoff/pitch.csv").open() as file:
    data = list(csv.DictReader(file))


def select(**fields):
    return [r for r in data if all(r[k] == str(v) for k, v in fields.items())]


def total(rows, field):
    return sum(int(r[field] or 0) for r in rows)


def clean(row):
    return (
        int(row["accepted"]) > 0
        and float(row["median"]) <= 12
        and float(row["p95"]) <= 25
        and int(row["octave"]) < 0.01 * int(row["eligible"])
    )


for fs, expected in ((48000, 200), (34300, 199)):
    for method in ("integer", "fitted"):
        (row,) = select(group="counter", fs=fs, method=method, mode="start")
        assert int(row["eligible"]) == int(row["accepted"]) == expected
        assert int(row["octave"]) == (expected if method == "integer" else 0)
        if method == "fitted":
            assert clean(row)
        print("counter", fs, method, row["median"], row["p95"], row["sha256"])
for group, shape in (
    ("named", None),
    ("sweep", "six"),
    ("sweep", "weak"),
    ("sweep", "second"),
    ("sweep", "bright"),
):
    for method in ("integer", "fitted", "deepest"):
        rows = select(group=group, method=method, mode="start")
        if shape:
            rows = [r for r in rows if r["shape"] == shape]
        print(
            group,
            shape,
            method,
            len(rows),
            "accepted/eligible",
            total(rows, "accepted"),
            total(rows, "eligible"),
            "gross",
            total(rows, "octave"),
            "cases outside cents/octave gates",
            sum(not clean(r) for r in rows),
        )
        if method == "fitted" and (group == "named" or shape == "six"):
            assert len(rows) == (336 if group == "named" else 4950)
            assert all(clean(r) and r["accepted"] == r["eligible"] for r in rows)
(row,) = select(
    group="harmonics",
    fs=48000,
    f=110,
    peak=0.35,
    shape="second",
    method="fitted",
    mode="start",
)
assert int(row["accepted"]) == int(row["octave"]) == int(row["eligible"]) == 200
assert len(select(group="DI", method="fitted", mode="start")) == 7
assert total(select(group="unpitched", method="fitted", mode="start"), "accepted") == 0
for r in select(method="fitted", mode="start"):
    if r["group"] in ("trajectory", "DI", "onset", "harmonics"):
        print(
            r["group"],
            r["shape"],
            r["peak"],
            r["accepted"],
            r["eligible"],
            r["median_hz"],
            r["median"],
            r["p95"],
            r["lag_ms"],
            r["aligned_rms"],
            r["modulation_hz"],
            r["depth_cents"],
        )
# One fingerprint per case independent of ranking; sorted to remove run-order dependence.
rows = select(method="integer", mode="start")
manifest = "".join(
    sorted(
        "\t".join(r[k] for k in ("group", "fs", "f", "shape", "peak", "sha256")) + "\n"
        for r in rows
    )
)
Path("build/spec-signoff/inputs.tsv").write_text(manifest)
fingerprint = hashlib.sha256(manifest.encode()).hexdigest()
print("fixture manifest", len(rows), fingerprint)
assert len(rows) == 6725
if args.reference_hashes:
    assert (
        fingerprint
        == "08c856c64164aaf2e7b38f7ff01f8d476a990adc9320a6704cddca2f27acf798"
    ), fingerprint
