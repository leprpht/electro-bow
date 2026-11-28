"""Extract only the historical publication boundary and original chronological identities."""
from pathlib import Path
import hashlib
import json
import sys
import numpy as np

root,destination=map(Path,sys.argv[1:])
old=json.loads((root/'joined.json').read_text())['uneven_delivery']
result={}
with np.load(root/'joined/uneven.npz') as arrays:
    for name,row in old.items():
        schedule=[]
        for sample in row['capture']:
            entry=[sample['tick'],sample['epoch'],sample['target']]
            if not schedule or entry[1:] != schedule[-1][1:]:
                schedule.append(entry)
        digest=lambda key:hashlib.sha256(arrays[key].astype('<f8').tobytes()).hexdigest()
        assert digest(name)==row['trace']
        result[name]=dict(schedule=schedule,samples=digest(name),gains=digest(name+'_gains'))
destination.write_text(json.dumps(result,allow_nan=False,separators=(',',':'))+'\n')
