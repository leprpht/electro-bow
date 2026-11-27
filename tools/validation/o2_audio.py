"""Qualify the same-commit renderer against the frozen physical and offline references."""

import json
import subprocess
from hashlib import sha256
from pathlib import Path

import guide_qualification as guide
import numpy as np
import o1_qualification as o1
from o2_bench import BINARY, DEPTHS, OUT, compare, oracle
from o2_response import apply_load, blend, coefficients, require
from scipy.io import wavfile

FIX = Path(__file__).with_name("fixtures")


def physical_key(fs, f, speed=0.1, ratio=1.8, kind="steady"):
    seconds = 7 if kind == "lift" else 3
    return f"{kind}_{fs}_{f:g}_R{ratio:g}_V{speed:g}_{seconds}s"


def rows(name):
    return np.memmap(OUT / "traces" / (name + ".f64le"), mode="r", dtype="<f8").reshape(
        -1, 30
    )


def qualify():
    (OUT / "traces").mkdir(exist_ok=True)
    (OUT / "audio").mkdir(exist_ok=True)
    cases = json.loads((guide.OUT / "manifest.json").read_text())["cases"]
    canonical = "".join(
        f"{n} {e['raw_sha256']} {e.get('wav_sha256', '-')}\n"
        for n, e in sorted(cases.items())
    )
    require(
        len(cases) == 152 and sha256(canonical.encode()).hexdigest() == o1.BASELINE,
        "physical inventory",
    )
    hashes = {}
    for manifest, checksum in [
        (o1.HASHES, "85ecdac330745653572bd43dd1f7eeebddf80f75511e10daa9a5dc14d0084464"),
        (
            FIX / "o2_sha256.tsv",
            "d956ed978cbf8a9e5f943e897ccf137dbac7034518e8e38bd9fe55adc1b6efc2",
        ),
    ]:
        require(guide.digest(manifest) == checksum, "frozen trace inventory")
        for line in manifest.read_text().splitlines():
            digest, name = line.split()
            hashes[Path(name).stem] = digest
    keep = set(hashes) | {physical_key(fs, 1318.51) for fs in guide.RATES}
    missing = keep - cases.keys()
    extra = {}
    for name in sorted(missing):
        kind, fs, f, r, s, sec = name.split("_")
        fs, speed, ratio = int(fs), float(s[1:]), float(r[1:])
        extra[name] = {
            "rate": fs,
            "hz": float(f),
            "speed": speed,
            "ratio": ratio,
            "k": ratio * speed / 1.65,
            "trajectory": kind,
            "successful_ticks": fs * int(sec[:-1]),
        }
    result = {}
    for name, e in {**cases, **extra}.items():
        fs, n = e["rate"], e["successful_ticks"]
        hz = e["hz"] * np.exp2(
            guide.trajectory(np.arange(n) / fs, e["trajectory"]) / 1200
        )
        controls = np.column_stack(
            (hz, np.full(n, e["speed"]), np.full(n, e["k"]))
        ).astype("<f8")
        if e["trajectory"] == "lift":
            controls[fs:, 2] = 0
        payload = controls.tobytes()
        if name in cases:
            require(sha256(payload).hexdigest() == e["controls_sha256"], name)
        path = OUT / "traces" / (name + ".f64le" if name in keep else "scratch.f64le")
        with path.open("wb") as dest:
            proc = subprocess.run(
                [BINARY, "--o2-guide", str(fs)],
                input=payload,
                stdout=dest,
                stderr=subprocess.PIPE,
                check=True,
            )
        require(
            list(map(int, proc.stderr.split())) == [n, 1, 0, 1, n, n],
            name + " commit count",
        )
        data = np.memmap(path, mode="r", dtype="<f8").reshape(n, 30)
        original = sha256(data[:, :12].tobytes()).hexdigest()
        observed = sha256(data[:, :22].tobytes()).hexdigest()
        if name in cases:
            require(original == e["raw_sha256"], name + " guide/M5 bit parity")
            if "wav" in e:
                require(
                    guide.digest(guide.OUT / e["wav"]) == e["wav_sha256"], name + " WAV"
                )
        if name in hashes:
            require(observed == hashes[name], name + " O1 bits")
        v = data[:, 21]
        expected = oracle(fs, v)
        states = np.column_stack((v, data[:, 22], data[:, 28:30]))
        errors = {
            str(d): compare(
                fs, name, v, d, data[:, [22, 23, 24, 25 + i]], states, expected
            )
            for i, d in enumerate(DEPTHS)
        }
        info = {
            "ticks": n,
            "commits": n,
            "calls": n,
            "original_sha256": original,
            "observed_sha256": observed,
            "max_error": errors,
        }
        if name in keep:
            h, l, _ = apply_load(v, coefficients(fs))
            offline_errors = []
            for i, d in enumerate(DEPTHS):
                difference = abs(data[:, 25 + i] - blend(h, l, d))
                require(
                    np.all(difference <= 1e-9),
                    f"{name} offline audio at {difference.argmax()}",
                )
                offline_errors.append(float(max(difference)))
            info.update(stage06_max_FS=offline_errors, sha256=guide.digest(path))
            for label, col in [("M5", 11), ("off", 25), ("factory", 26)]:
                wavfile.write(
                    OUT / "audio" / f"{name}-{label}.wav",
                    fs,
                    data[: e.get("seconds", n // fs) * fs, col].astype("<f4"),
                )
        result[name] = info
        del data
        if name not in keep:
            path.unlink()
    (OUT / "parity.json").write_text(
        json.dumps(result, indent=2, allow_nan=False) + "\n"
    )
    return result


if __name__ == "__main__":
    qualify()
