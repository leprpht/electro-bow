"""Compact 8.2 audition. Engineering exports are unity-gain float32."""

import io
import json
from zipfile import ZIP_DEFLATED, ZipFile

import numpy as np
from acoustic_identity import FIXTURES
from acoustic_observation import rms
from i1_motion import OUT
from o2_qualification import archive_entry
from o2_records import sample_hash
from scipy.io import wavfile


def main():
    fs = 48000
    clips = {}
    provenance = {}
    for kind, label in (
        ("steady", "previous-stationary"),
        ("vibrato", "revised-vibrato"),
        ("bend", "revised-bend"),
        ("veto", "depth-veto-recovery"),
    ):
        with np.load(OUT / f"motion/{kind}-{fs}.npz") as z:
            q = z["q"]
            value = (
                q[:, 0] + z["old_scale"] * z["d"][:, 0]
                if kind == "steady"
                else z["wet"][:, 3]
            )
            clips[label] = value[: 2 * fs]
            provenance[label] = f"{kind}-{fs}, " + (
                "previous off" if kind == "steady" else "automated final wet"
            )
            if kind == "vibrato":
                clips["neutral-vibrato"] = q[: 2 * fs, 3]
                provenance["neutral-vibrato"] = "vibrato-48000, automated returned q"
    with np.load(OUT / "revised-held.npz") as z:
        for name in sorted(z.files):
            clips[name] = z[name]
            provenance[name] = "steady-48000-196, held DI " + name
    for player in (1, 2):
        name = f"techs-P{player}-60-DI"
        _, x = wavfile.read(OUT / "listening/engineering" / (name + ".wav"))
        clips[name] = x.astype(float)
        provenance[name] = "attributed original native-gain DI excerpt, attack aligned"
    gains = {}
    with ZipFile(OUT / "stage-08-2-listening.zip", "w", ZIP_DEFLATED) as archive:
        for name, x in clips.items():
            anchor = (
                (round(0.06 * fs), round(0.22 * fs))
                if name.endswith("-DI")
                else ((0, len(x)) if name.endswith("-held") else (fs // 2, fs))
            )
            gain = 0.1 / rms(x[slice(*anchor)])
            gains[name] = {
                "matched": gain,
                "engineering": 1.0,
                "anchor": anchor,
                "fs": fs,
                "source": provenance[name],
                "samples": sample_hash(x),
            }
            for group, g in (("engineering", 1.0), ("matched", gain)):
                file = io.BytesIO()
                wavfile.write(file, fs, (g * x).astype("<f4"))
                alias = name
                if group == "matched" and name.endswith("-held"):
                    alias = "held-B" if "P1" in name else "held-A"
                archive_entry(archive, group + "/" + alias + ".wav", file.getvalue())
        archive_entry(archive, "manifest.json", json.dumps(gains, indent=2))
        archive_entry(
            archive, "ATTRIBUTION.md", (FIXTURES / "ATTRIBUTION.md").read_bytes()
        )
        archive_entry(
            archive,
            "LISTEN.txt",
            "Use only the matched folder for blind source pairing. Compare neutral and revised vibrato at fixed playback gain, follow the bend and listen through the veto and recovery. Record timestamp, route, A/B cue, transition and indistinguishable outcomes before revealing the key. No owner verdict recorded.\n",
        )
    (OUT / "listening-manifest.json").write_text(json.dumps(gains, indent=2) + "\n")


if __name__ == "__main__":
    main()
