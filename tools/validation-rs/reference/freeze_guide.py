"""Export only reports from the frozen Python guide run, including the phase time series."""
import hashlib
import json
from pathlib import Path
import sys
import numpy as np
root = Path(sys.argv[1])
out = Path(sys.argv[2])
source = json.loads((root / "manifest.json").read_text())
assert source["passed"] and len(source["cases"]) == 152
keys = ("bridge pickup slip_onsets mean_slip_interval_ms duty normal_force static_force_limit "
        "force_rms measured_period_ms band_1500_6000 start_rms end_rms delta_db unfloored_delta_db "
        "six_second_rms last_second first_second_after_lift_time windows mean_audio rms_drift_db "
        "bridge_rms_drift_db lag_ms unaligned_cents aligned_cents min_hz max_hz min_slew_cents_s "
        "max_slew_cents_s max_error_cents max_sample_step modulation_hz depth_cents "
        "raw_sha256 controls_sha256").split()
rows = {}
for name, row in source["cases"].items():
    r = {k: row[k] for k in keys if k in row}
    r["peaks"] = [row["peaks"][k] for k in source["columns"]]
    if "phase" in row:
        z = np.load(root / row["phase"])
        data = np.column_stack([z[k] for k in ("time_s","observed_cents","input_cents","aligned_cents")])
        payload = data.astype("<f8").tobytes()
        (out / (name + "-phase.f64le")).write_bytes(payload)
        r["phase_reference_sha256"] = hashlib.sha256(payload).hexdigest()
    rows[name] = r
(out / "guide.json").write_text(json.dumps(rows, sort_keys=True, allow_nan=False) + "\n")
