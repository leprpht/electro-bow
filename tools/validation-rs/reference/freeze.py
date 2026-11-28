"""V1 historical comparator only. Run with the entry checkout as cwd, before Rust replay."""
import hashlib
import json
import math
import runpy
import struct
import subprocess
import sys
from pathlib import Path

ENTRY = "fcc272f8ec7a0472917c1e787edbd2918dcc3ae0"
if subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip() != ENTRY:
    raise SystemExit("expected V1 entry revision")
if subprocess.check_output(["git", "status", "--porcelain"]):
    raise SystemExit("expected unmodified reference checkout")
out = Path(sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=True)
sys.path.insert(0, str(Path("tools/validation").resolve()))
import numpy as np
from i1_recordings import decoded, recorded_audio
from acoustic_observation import db, partials, rms
from scipy.io import wavfile

# Observe the old program's actual input order and independently calculated answers.
# Capture at its comparison boundary, including the mode updated in each sequential sweep.
def capture(frame, event, arg):
    if event == "call" and frame.f_code.co_name == "check_rust":
        suite = frame.f_globals
        with (out / "contact.f64le").open("wb") as stream:
            for d, k, mode in frame.f_locals["cases"]:
                j, slip, selected = suite["graph"](d, k, mode)
                stream.write(struct.pack("<ddb ddb", d, k, mode, j, slip, selected))
        sys.setprofile(None)

sys.argv = ["contact_contract.py", "--rust", "build/contact"]
sys.setprofile(capture)
runpy.run_path("tools/validation/contact_contract.py", run_name="__main__")
fixtures = []
for line in Path("tools/validation/fixtures/acoustic/SHA256SUMS").read_text().splitlines():
    digest, path = line.split("  ")
    rate, samples = recorded_audio(Path(path))
    original_rate, original = wavfile.read(path)
    assert rate == original_rate and hashlib.sha256(Path(path).read_bytes()).hexdigest() == digest
    fixtures.append(dict(path=path, sha256=digest, decoded=decoded(samples), rate=rate,
                         channels=1 if samples.ndim == 1 else samples.shape[1],
                         frames=len(samples), dtype=str(original.dtype)))
(out / "fixtures.json").write_text(json.dumps(fixtures, indent=2, allow_nan=False) + "\n")
# Store the actual generated input words so that platform libm is never the stimulus oracle.
numerics = []
for size in (1, 3, 16, 17, 240, 1024):
    for label, x in (
        ("silence", np.zeros(size)), ("negative-zero", np.full(size, -0.0)),
        ("subnormal", np.full(size, np.nextafter(0.0, 1.0))),
        ("impulse", np.eye(1, size).ravel()), ("nyquist", (-1.0) ** np.arange(size)),
        ("tone", np.sin(2 * np.pi * np.arange(size) * 196 / 32000)),
    ):
        ph = partials(x, 32000, 4000)
        fft = np.fft.rfft(x)
        numerics.append(dict(case=f"{label}/{size}", samples=x.tolist(), rms=rms(x),
            peak=float(max(abs(x))), mean=float(x.mean()), db=float(db(rms(x))),
            hann=np.hanning(size).tolist(), fft=np.column_stack((fft.real, fft.imag)).tolist(),
            projections=np.column_stack((ph.real, ph.imag)).tolist()))
(out / "numerics.json").write_text(json.dumps(numerics, allow_nan=False) + "\n")

# Preserve every original circuit-delay measurement and both sides of each eager/lazy read.
def ring_probe(frame, event, result):
    if event != "return":
        return
    a = frame.f_locals
    if frame.f_code.co_name == "absolute_delay":
        ring_stream.write(struct.pack("<d", result))
    elif frame.f_code.co_name == "check" and "reads" in a:
        ring_stream.write(struct.pack("<Qddddd", a["n"], a["delay"], *a["reads"][0], *a["reads"][1]))

with (out / "ring.bin").open("wb") as ring_stream:
    sys.setprofile(ring_probe)
    runpy.run_path("tools/validation/ring_oracle.py", run_name="__main__")
    sys.setprofile(None)
