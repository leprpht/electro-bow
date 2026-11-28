"""Extract the reviewed injected feature schedules and expected producer digests for all 36 cases."""
import json
import sys
import subprocess
from pathlib import Path

import numpy as np

if subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip() != "fcc272f8ec7a0472917c1e787edbd2918dcc3ae0":
    raise SystemExit("expected V1 entry")
sys.path.insert(0,str(Path("tools/validation").resolve()))
from i1_acceptance import stationary_inventory
from i1_study import VECTORS, source, updates_for
from o2_audio import physical_key

old = json.loads(Path("build/i1/stationary.json").read_text())["rows"]
result = []
for name, (rate, f, v, r, vector) in sorted(stationary_inventory().items()):
    updates = [] if vector in VECTORS else updates_for(source(rate,f,vector),rate,f)
    row = old[name]
    result.append(dict(case=name, rate=rate, hz=f, physical=physical_key(rate,f,v,r),
        constant=np.expm1(np.log(10) * VECTORS[vector] / 20).tolist() if vector in VECTORS else None,
        smoother=float(-np.expm1(-1/(.015*rate))),
        updates=[[tick,np.expm1(np.log(10)*db/20).tolist()] for tick,db in updates],
        wet=row["wet"], correction=row["delta"], scale=row["scale"],
        predicates=[d["passes"] for d in row["depths"]]))
Path(sys.argv[1]).write_text(json.dumps(result,indent=2,allow_nan=False)+"\n")
