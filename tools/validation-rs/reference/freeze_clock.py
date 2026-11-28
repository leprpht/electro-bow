"""Observe the historical clock at the typed replay boundary, retaining exact discrete decisions."""
import hashlib
import json
from dataclasses import asdict
from pathlib import Path
import sys
sys.path.insert(0,str(Path("tools/validation").resolve()))
import i1_clock as c

reference = Path(sys.argv[1])
contract = Path(sys.argv[2])
old = c.replay
cases = {}
def replay(x, fs, frames, events, partitions=(127,), defect=None, *, identity_stamps):
    rows = old(x, fs, frames, events, partitions, defect, identity_stamps=identity_stamps)
    def event(e):
        return dict(kind=e.value) if isinstance(e,c.Gesture) else dict(kind=type(e).__name__.lower(),**asdict(e))
    discrete = [{k:v for k,v in r.items() if k not in ("held","target","published")} for r in rows]
    digest = hashlib.sha256(json.dumps(discrete,sort_keys=True,separators=(",",":")).encode()).hexdigest()
    case=dict(frames=frames and {str(k):asdict(v) for k,v in frames.items()},
              events={str(k):[event(e) for e in v] for k,v in events.items()},
              stamps=sorted(identity_stamps), count=len(x), rate=fs, defect=defect, digest=digest, constant_after=3840 if x[-1]==-.9 else None)
    if digest not in cases:
        cases[digest]=case
        (reference/(digest+".json")).write_text(json.dumps(rows,allow_nan=False,separators=(",",":")))
    return rows
reference.mkdir(exist_ok=True,parents=True)
c.replay=replay
c.controls()
contract.write_text(json.dumps(list(cases.values()),allow_nan=False,separators=(",",":"))+"\n")
