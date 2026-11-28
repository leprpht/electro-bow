"""Pin original 8.2 inputs, delivered wet and scheduled decisions for all motion rows."""
import hashlib
import json
import sys
from pathlib import Path
import numpy as np

sys.path.insert(0, str(Path("tools/validation").resolve()))
from i1_motion import inputs
def digest(a):
    return hashlib.sha256(np.asarray(a, dtype="<f8").tobytes()).hexdigest()
root = Path("build/i1")
old = json.loads((root / "motion.json").read_text())["rows"]
cases = {}
for key, row in sorted(old.items()):
    kind, rate = key.rsplit("-",1)
    controls, depth = inputs(int(rate),kind)
    trace=root / "motion" / (key+".npz")
    assert hashlib.sha256(trace.read_bytes()).hexdigest()==row["trace"]
    with np.load(trace) as z:
        hashes={k:digest(z[k]) for k in ("pitch","depth","request","q","d","scale","wet")}
    # Recorded decision sequence has exact categorical values.
    monitor=[[r[k] for k in ("start","end","conditioned","bridge_ok","available","realized_ok","selection_tick","veto","scale")] for r in row["monitor"]]
    cases[key]=dict(controls=digest(controls),physical=row["physical"],hashes=hashes,
        monitor_sha256=digest([[(-1 if x is None else x) for x in r] for r in monitor]),
        windows=len(monitor))
    if kind=="vibrato":
        for w in ("witness","delivery_witness"):
            cases[key][w]=row[w]
Path(sys.argv[1]).write_text(json.dumps(cases,indent=2,allow_nan=False)+"\n")
