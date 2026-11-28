"""Export original DI publications and producer digests. The Rust path independently reanalyzes the files."""
import hashlib
import json
from pathlib import Path
import sys
import numpy as np
sys.path.insert(0, str(Path("tools/validation").resolve()))
from acoustic_identity import FIXTURES, audio
from i1_study import updates_for

p = Path("build/i1")
old = json.loads((p / "stationary.json").read_text())
out = {}
for name, row in sorted(old["sources"].items()):
    rate, x = audio(FIXTURES / (name + ".wav"))
    attack = int(np.flatnonzero(abs(x) > .01)[0])
    schedule = [[tick, np.expm1(np.log(10)*db/20).tolist()] for tick,db in updates_for(x[attack:],rate,196)]
    out[name] = dict(attack=attack, schedule=schedule, wet=row["row"]["wet"], held=row["held"])
contract = dict(sources=out, pairs=old["pairs"])
for filename in ("stage-08-2-listening.zip","listening-key.json","listening-manifest.json"):
    contract[filename] = hashlib.sha256((p / filename).read_bytes()).hexdigest()
Path(sys.argv[1]).write_text(json.dumps(contract, separators=(",",":"), allow_nan=False)+"\n")
