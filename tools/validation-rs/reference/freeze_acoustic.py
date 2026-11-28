"""Normalize historical report keys to sample-indexed intervals, without changing measurements."""
from pathlib import Path
import json
import re
import sys
old = json.loads(Path(sys.argv[1]).read_text())
assert old["status"] == "passed"
result = {k: old[k] for k in ("v17","high_band","listening")}
result["witness"] = [old["witness"][k] for k in ("common","trimmed","implied","demand_db")]
result["nominal_sha256"] = old["nominal_mono_input"]["sha256_f32le"]
result["corners"] = sorted(old["corners"], key=lambda r: (r["V"],r["R"],list(old["injected_vectors_dB"]).index(r["vector"]),r["noise"],r["G"],r["character"],r["internal_Out"]))
result["onset_release"] = {}
for key,row in old["onset_release"].items():
    f,d,window = key.split("/")
    start,end = [round(float(v)*48000) for v in re.split(r"(?<!e)-",window)]
    result["onset_release"][f"{f}/{d}/{start}-{end}"] = dict(row,interval=[start,end])
Path(sys.argv[2]).write_text(json.dumps(result, sort_keys=True,allow_nan=False)+"\n")
