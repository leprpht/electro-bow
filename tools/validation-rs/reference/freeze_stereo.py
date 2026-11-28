"""Freeze the sparse feature/epoch boundary and the outputs on the other side of the transaction."""
from pathlib import Path
import hashlib
import json
import sys
import numpy as np

root, destination = map(Path, sys.argv[1:])
result = json.loads((root / "joined.json").read_text())
contract = {}
for name in ["reference", *sorted(result["faults"])]:
    fault = result["faults"].get(name)
    capture = result["capture"] if fault is None else fault["capture"]
    schedule = []
    previous = None
    for row in capture:
        value = [row["epoch"], row["target"]]
        if value != previous:
            schedule.append([row["tick"], *value])
        previous = value
    with np.load(root / "joined" / (name + ".npz")) as data:
        rows = data["samples" if fault is None else "left"]
        gains = data["coefficients" if fault is None else "left_coefficients"]
        identity = lambda a: hashlib.sha256(a.astype("<f8").tobytes()).hexdigest()
        contract[name] = dict(schedule=schedule, samples=identity(rows), gains=identity(gains),
                              columns=[identity(column) for column in rows.T])
        if fault:
            contract[name]["right"] = identity(data["right"])
destination.write_text(json.dumps(contract, indent=2, allow_nan=False) + "\n")
