"""Replay-owned physical clock. A corrected wet package can only cite this record."""

import csv
import json
from dataclasses import dataclass, replace
from hashlib import sha256
from pathlib import Path
from weakref import WeakValueDictionary

import numpy as np
from guide_qualification import digest
from o2_response import require


def sample_hash(x):
    return sha256(np.ascontiguousarray(x, dtype="<f8")).hexdigest()


def publish(path, binary, controls, fs, pitch, commits):
    count = len(controls)
    require(commits == [count, 1, 0, 1, count, count], "unsuccessful physical epoch")
    raw = np.memmap(path, mode="r", dtype="<f8").reshape(count, 30)
    record = {
        "fixture": path.stem,
        "rate": fs,
        "pitch": pitch,
        "epoch": 1,
        "start": 0,
        "end": count,
        "count": count,
        "window": [count - fs, count] if count >= fs else None,
        "source": digest(path),
        "executable": digest(binary),
        "invocation": ["--o2-guide", str(fs)],
        "controls": sample_hash(controls),
        "taps": {
            str(i): sample_hash(raw[:, i]) for i in (3, 11, 21, 22, 24, 25, 26, 27)
        },
        "commits": commits,
    }
    path.with_suffix(".json").write_text(json.dumps(record, sort_keys=True))


@dataclass(frozen=True)
class WetTrace:
    samples: np.ndarray
    source: str
    rate: int
    pitch: float
    epoch: int
    start_tick: int
    end_tick: int
    count: int
    window: tuple
    settings: tuple
    m5_hash: str
    off_hash: str
    depth: float

    def tag(self):
        return tuple(v for k, v in vars(self).items() if k != "samples")


def producer_clock(m5, off, fs, pitch, settings):
    path = Path(m5.filename) if isinstance(m5, np.memmap) else None
    require(path is not None and path.is_file(), "missing producing carrier")
    record = json.loads(path.with_suffix(".json").read_text())
    from o2_bench import BINARY

    n = len(m5)
    canonical = np.memmap(path, dtype="<f8", mode="r").reshape(n, 30)
    taps = {
        str(i): sample_hash(canonical[:, i]) for i in (3, 11, 21, 22, 24, 25, 26, 27)
    }
    require(taps == record["taps"], "canonical Rust tap schema")
    require(
        record["fixture"] == path.stem
        and record["source"] == digest(path)
        and record["executable"] == digest(BINARY)
        and record["invocation"] == ["--o2-guide", str(fs)]
        and record["commits"] == [n, 1, 0, 1, n, n]
        and (
            record["rate"],
            record["pitch"],
            record["epoch"],
            record["start"],
            record["end"],
            record["count"],
            record["window"],
        )
        == (fs, pitch, 1, 0, n, n, [n - fs, n])
        and n >= fs
        and record["taps"]["11"] == sample_hash(m5)
        and record["taps"]["25"] == sample_hash(off),
        "physical clock/tap provenance",
    )
    with (
        Path(__file__)
        .with_name("fixtures")
        .joinpath("o2_replay_sha256.tsv")
        .open() as f
    ):
        frozen = {
            r["case"]: r["retained_O2"] for r in csv.DictReader(f, delimiter="\t")
        }
    if path.stem in frozen or path.parent.name == "traces":
        require(frozen[path.stem] == record["source"], "frozen carrier digest")
        kind, rate, hz, ratio, speed, seconds = path.stem.split("_")
        require(
            (
                int(rate),
                float(hz),
                int(seconds[:-1]) * fs + (fs // 4 if kind == "vibrato" else 0),
            )
            == (fs, pitch, n),
            "carrier fixture clock",
        )
        from guide_qualification import trajectory

        v, r = float(speed[1:]), float(ratio[1:])
        expected = np.column_stack(
            (
                pitch * np.exp2(trajectory(np.arange(n) / fs, kind) / 1200),
                np.full(n, v),
                np.full(n, r * v / 1.65),
            )
        )
        if kind == "lift":
            expected[fs:, 2] = 0
        require(
            sample_hash(expected) == record["controls"], "producing control schedule"
        )
    ident = digest(path.with_suffix(".json"))
    return WetTrace(
        off,
        ident,
        fs,
        pitch,
        1,
        0,
        n,
        n,
        (n - fs, n),
        settings,
        sample_hash(m5),
        sample_hash(off),
        0,
    ), taps


def check_pair(
    m5, off, fs, pitch, settings, noise, corrected, corrected_off, clock, columns
):
    ref, taps = producer_clock(m5, off, fs, pitch, settings)
    require(
        all(isinstance(r, WetTrace) for r in (clock, corrected, corrected_off)),
        "paired final wet traces required",
    )
    require(
        clock.tag() == corrected_off.tag() == ref.tag()
        and corrected.tag() == replace(ref, depth=noise).tag()
        and corrected.samples.shape == corrected_off.samples.shape == m5.shape
        and all(sample_hash(x) == taps[str(i)] for i, x in columns),
        "final wet alignment/settings/reference differ",
    )

    record = getattr(corrected.samples, "record", None)
    index = (0, 0.5, 1).index(noise)
    require(
        isinstance(record, OutputRecord)
        and _captured.get(id(record)) is record
        and record.settings == settings
        and record == getattr(corrected_off.samples, "record", None)
        and record.carrier == ref.source
        and record.count == ref.count
        and index < len(record.hashes)
        and sample_hash(corrected.samples) == record.hashes[index]
        and sample_hash(corrected_off.samples) == record.hashes[0],
        "synchronized wet producer samples",
    )


@dataclass(frozen=True)
class OutputRecord:
    """Captured at the output of the producer, before the measurement consumer."""

    carrier: str
    count: int
    hashes: tuple
    settings: tuple


_captured = WeakValueDictionary()


class Delivered(np.ndarray):
    def __array_finalize__(self, source):
        self.record = getattr(source, "record", None)


def delivered(samples, m5, settings=()):
    output = np.asarray(samples).view(Delivered)
    output.record = OutputRecord(
        digest(Path(m5.filename).with_suffix(".json"))
        if isinstance(m5, np.memmap)
        and Path(m5.filename).with_suffix(".json").is_file()
        else "",
        len(samples),
        tuple(sample_hash(col) for col in samples.T),
        settings,
    )
    _captured[id(output.record)] = output.record
    return output
