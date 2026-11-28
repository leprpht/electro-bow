"""Capture the immutable artifact contract and every historical archive member."""
from pathlib import Path
from zipfile import ZipFile
import hashlib
import json
import sys

root, dest=map(Path,sys.argv[1:])
old=json.loads((root/'qualification.json').read_text())
artifacts={}
for name,sha in old['evidence'].items():
    path=root/name
    assert hashlib.sha256(path.read_bytes()).hexdigest()==sha
    entry=dict(sha256=sha)
    if name.endswith('.json'):
        entry['record']=json.loads(path.read_text())
    if name=='listening.zip':
        with ZipFile(path) as z:
            entry['members']={n:hashlib.sha256(z.read(n)).hexdigest() for n in z.namelist()}
            entry['gains']=json.loads(z.read('listening-gains.json'))
    artifacts[name]=entry
assert len(artifacts)==105 and len(old['reports'])==11
dest.write_text(json.dumps(dict(artifacts=artifacts,reports=sorted(old['reports'])),sort_keys=True,separators=(',',':'))+'\n')
