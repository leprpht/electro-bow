"""Extract the historical 6,725 fixture specifications. Expected results stay in the reference CSV."""
import csv
import sys
from pathlib import Path
root = Path("build/spec-signoff")
with (root/"pitch.csv").open() as src,Path(sys.argv[1]).open("w") as dest:
    rows=csv.DictReader(src)
    fields="group fs f shape peak stimulus_start score_from score_until sha256".split()
    writer=csv.DictWriter(dest,fieldnames=fields,extrasaction="ignore",delimiter="\t",lineterminator="\n")
    writer.writeheader()
    count=0
    for row in rows:
        if row["method"]=="integer" and row["mode"]=="start":
            writer.writerow(row)
            count+=1
    assert count==6725
