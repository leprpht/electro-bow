"""Extract Review 8.2 bank inputs and execute the original C++ comparator on all motion ticks."""
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import numpy as np

entry = "fcc272f8ec7a0472917c1e787edbd2918dcc3ae0"
if subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip() != entry:
    raise SystemExit("expected entry checkout")
out = Path(sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=True)
source = Path("build/i1/motion")
motion = json.loads(source.with_name("motion.json").read_text())["rows"]
rows = []
binary = out / "cpp-bank"
subprocess.run(["clang++", "-std=c++17", "-O3", "-Wall", "-Wextra", "-Werror",
                "tools/validation/i1_motion_bank.cpp", "-o", binary], check=True)
for name, row in sorted(motion.items()):
    rate = int(name.rsplit("-", 1)[1])
    original = source / (name + ".npz")
    assert hashlib.sha256(original.read_bytes()).hexdigest() == row["trace"]
    with np.load(original) as archive:
        f, q, g = (archive[key] for key in ("pitch", "q", "request"))
        frames = np.column_stack((f, q, np.broadcast_to(g, (len(q), 12)))).astype("<f8")
        payload = frames.tobytes()
        actual = subprocess.check_output([binary, str(rate)], input=payload)
        taps = np.frombuffer(actual, "<f8").reshape(len(f), 6)
        assert np.allclose(taps[:, :4], archive["d"], rtol=0, atol=2e-11)
        (out / (name + ".in")).write_bytes(payload)
        (out / (name + ".out")).write_bytes(actual)
        record = dict(case=name, frames=len(f), rate=rate, source=row["trace"],
                      input=hashlib.sha256(payload).hexdigest(), output=hashlib.sha256(actual).hexdigest())
        if name.startswith("vibrato-"):
            diff = (archive["wet"] - q).astype("<f8")[:, [0, 3]].copy().tobytes()
            (out / (name + ".difference")).write_bytes(diff)
            record.update(difference=hashlib.sha256(diff).hexdigest(), witness=row["witness"],
                          delivery_witness=row["delivery_witness"])
        rows.append(record)
(out / "manifest.json").write_text(json.dumps(rows, indent=2, allow_nan=False) + "\n")
