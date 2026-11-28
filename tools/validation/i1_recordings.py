"""Stage 08.1 controlled recording manifest preflight, prior to wet rendering."""

from hashlib import sha256

import numpy as np
from guide_qualification import digest
from o2_response import require
from scipy.io import wavfile

PROTOCOL = "I1-repeatability-8.1"


def decoded(x):
    a = np.array(x, dtype="<f8", copy=True)
    a[a == 0] = 0.0  # collapse both IEEE zero encodings
    return sha256(a.tobytes()).hexdigest()


def recorded_audio(path):
    fs, pcm = wavfile.read(path)
    if pcm.dtype == np.uint8:
        return fs, (pcm.astype(float) - 128) / 128
    if np.issubdtype(pcm.dtype, np.signedinteger):
        return fs, pcm.astype(float) / float(-np.iinfo(pcm.dtype).min)
    return fs, pcm.astype(float)


def inventory(config, manifest):
    require(
        config.get("protocol_revision") == PROTOCOL
        and config.get("manifest_revision") == 1,
        "recording protocol/manifest revision",
    )
    originals, segments, ids, ranges = {}, {}, set(), {}
    records, inputs, failures = {}, {}, []
    for c in ("A", "B"):
        takes = config["classes"][c]["takes"]
        require(len(takes) == 4, "four independent attacks per setting")
        for i, item in enumerate(takes, 1):
            name = c + str(i)
            errors = []
            records[name] = {"metadata": item, "failures": errors}
            try:
                path = (manifest.parent / item["original"]).resolve()
                fs, x = recorded_audio(path)
                require(x.ndim == 1 and np.isfinite(x).all(), "invalid decoded samples")
                require(fs >= 16000, "unsupported DI decimator rate")
                original = digest(path)
                whole = decoded(x)
                lo, hi = item["interval"]
                require(
                    type(lo) is int and type(hi) is int and 0 <= lo < hi <= len(x),
                    "invalid sample interval",
                )
                y = x[lo:hi]
                starts = np.flatnonzero(abs(y) > 0.01)
                require(len(starts) > 0, "absent attack")
                start = int(starts[0])
                count = round(0.30 * fs)
                require(
                    len(y) - start >= count and max(abs(y)) < 1,
                    "clipped or incomplete take",
                )
                capture = decoded(y[start : start + count])
                records[name].update(
                    original=original,
                    decoded=whole,
                    segment=decoded(y),
                    capture=capture,
                    fs=fs,
                    attack=lo + start,
                )
                if whole in originals and originals[whole] != path:
                    errors.append("duplicate decoded recording")
                if capture in segments:
                    errors.append("duplicate decoded capture " + segments[capture])
                if any(a < hi and lo < b for a, b in ranges.get(whole, [])):
                    errors.append("overlapping recording segments")
                originals[whole] = path
                segments[capture] = name
                ranges.setdefault(whole, []).append((lo, hi))
                for k, v in (
                    ("original_sha256", original),
                    ("decoded_sha256", whole),
                    ("segment_sha256", decoded(y)),
                    ("rate", fs),
                    ("order", 2 * i - (c == "A")),
                ):
                    if item.get(k) != v:
                        errors.append("incorrect " + k)
                uid = item.get("take_id")
                if not isinstance(uid, str) or not uid.strip() or uid in ids:
                    errors.append("missing or duplicate take identity")
                ids.add(uid)
                if (
                    not isinstance(item.get("attack_record"), str)
                    or not item["attack_record"].strip()
                ):
                    errors.append("missing physical attack documentation")
                inputs[name] = (fs, y[start:], capture)
            except (ValueError, TypeError, KeyError, OSError) as err:
                errors.append(str(err))
            failures.extend(name + ": " + e for e in errors)
    return records, inputs, failures
