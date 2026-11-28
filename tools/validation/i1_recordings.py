"""Stage 08.2 controlled recording manifest preflight, prior to wet rendering."""

from hashlib import sha256

import numpy as np
from guide_qualification import digest
from o2_response import require
from scipy.io import wavfile

PROTOCOL = "I1-repeatability-8.2"


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
        and config.get("manifest_revision") == 2,
        "recording protocol/manifest revision",
    )
    originals, segments, ids, ranges = {}, {}, set(), {}
    records, inputs, failures = {}, {}, []
    mode = config.get("recording_mode")
    require(mode in ("shared-session", "separate-originals"), "recording mode")
    ordered = []
    if mode == "shared-session":
        session = config["session"]
        session_path = (manifest.parent / session["original"]).resolve()
        rate, original_audio = recorded_audio(session_path)
        require(digest(session_path) == session["sha256"], "shared original recording")
        log_path = manifest.parent / session["attack_log"]
        import json

        require(digest(log_path) == session["attack_log_sha256"], "session attack log")
        log = json.loads(log_path.read_text())
        require(len(log) == 8, "attack log inventory")

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
                if mode == "shared-session":
                    begin, end = item["session_interval"]
                    attack = begin + start
                    require(
                        type(begin) is int
                        and type(end) is int
                        and 0 <= begin < end <= len(original_audio)
                        and rate == fs
                        and decoded(original_audio[begin:end]) == decoded(y),
                        "session sample coordinates",
                    )
                    order = item["order"]
                    row = log[order - 1]
                    label = config["classes"][c]
                    require(
                        row
                        == dict(
                            order=order,
                            take_id=uid,
                            attack=attack,
                            interval=[begin, end],
                            pickup=label["pickup"],
                            gain=label["gain"],
                            gesture=label["gesture"],
                        ),
                        "session attack log entry",
                    )
                    ordered.append((order, begin, end, attack))
                inputs[name] = (fs, y[start:], capture)
            except (ValueError, TypeError, KeyError, OSError) as err:
                errors.append(str(err))
            failures.extend(name + ": " + e for e in errors)
    if mode == "shared-session":
        for previous, current in zip(sorted(ordered), sorted(ordered)[1:]):
            if current[1] < previous[2] or current[3] - previous[3] < round(
                0.30 * rate
            ):
                failures.append(
                    "shared session chronological independent attack intervals"
                )
    elif len(originals) != 8:
        failures.append("separate originals require eight original recordings")
    return records, inputs, failures
