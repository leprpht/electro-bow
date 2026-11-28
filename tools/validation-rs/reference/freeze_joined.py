"""Extract the two actually executed joined producers from the nominal and half-product runs."""
import hashlib
import json
from pathlib import Path
import sys
import numpy as np

out = Path(sys.argv[1])
proof = {}
for case, directory in (("entry", "build/i1"), ("half-product", "build/half-product")):
    root = Path(directory)
    result = json.loads((root / "qualification.json").read_text())
    assert result["status"] == "passed"
    columns = json.loads((root / "joined.json").read_text())["columns"]
    with np.load(root / "joined/reference.npz") as z:
        samples = z["samples"][:, [columns.index(k) for k in
            ("epoch", "physical_index", "E", "wet", "product", "cached_E", "cached_product")]]
        data = np.column_stack((samples,z["di"])).astype("<f8").tobytes()
    (out / (case+"-joined.bin")).write_bytes(data)
    proof[case] = {"run":result["run_id"], "sha256": hashlib.sha256(data).hexdigest()}
print(json.dumps(proof, indent=2))
