"""One offline epoch kernel for individual stereo ticks and contiguous committed segments."""

import ctypes as ct
import os
import subprocess
from enum import IntEnum
from functools import lru_cache
from pathlib import Path

import numpy as np
from o2_records import delivered
from o2_response import require

OUT = Path(os.environ.get("ELECTROBOW_I1_OUT", "build/stage-08-2/i1"))


class Site(IntEnum):
    PHASE = 1
    COEFFICIENT = 2
    BANK = 3
    TRIAL = 4
    MOMENT = 5
    NORMALIZE = 6
    GRAM = 7
    RESIDUAL = 8
    WET = 9
    CONVERSION = 10
    PHYSICAL = 20
    ELECTRICAL = 21
    CACHE = 22


class NumericalFault(Exception):
    def __init__(self, site, offset=0):
        self.site, self.offset = Site(site), offset
        super().__init__(self.site.name)


@lru_cache(None)
def library():
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / "i1-stream.so"
    src = Path(__file__).with_suffix(".cpp")
    subprocess.run(
        [
            "clang++",
            "-std=c++17",
            "-O3",
            "-ffp-contract=off",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-shared",
            "-fPIC",
            src,
            "-o",
            path,
        ],
        check=True,
    )
    lib = ct.CDLL(str(path.resolve()))
    pointer = ct.c_void_p
    lib.epoch_size.restype = ct.c_size_t
    lib.epoch_init.argtypes = [pointer]
    lib.epoch_run.argtypes = [
        pointer,
        ct.c_double,
        pointer,
        pointer,
        ct.c_int,
        ct.c_int,
        ct.POINTER(ct.c_int),
    ]
    lib.epoch_stats.argtypes = [pointer, pointer, ct.c_int]
    return lib


class Epoch:
    def __init__(self, fs):
        self.fs = fs
        self.lib = library()
        self.state = ct.create_string_buffer(self.lib.epoch_size())
        self.lib.epoch_init(self.state)
        self.frame = np.zeros((1, 20))
        self.sample = np.zeros_like(self.frame)
        self.c = np.zeros(12)
        self.a = 0.0
        self.veto = True
        self.smooth = -np.expm1(-1 / (0.015 * fs))

    def run(self, frames, output, probe=0):
        done = ct.c_int()
        code = self.lib.epoch_run(
            self.state,
            self.fs,
            frames.ctypes.data,
            output.ctypes.data,
            len(frames),
            probe,
            ct.byref(done),
        )
        if code:
            raise NumericalFault(code, done.value)

    def tick(self, q, m5, pitch, target, trusted, observation, probe=0):
        frame = self.frame[0]
        self.c += self.smooth * (target - self.c)
        frame[0:2] = pitch, m5
        frame[2:6], frame[6:18] = q, self.c
        frame[18:] = observation, trusted
        self.run(self.frame, self.sample, probe)
        self.a = self.sample[0, 8]
        self.veto = bool(self.sample[0, 15])
        return self.sample[0, 7]


def render(q, m5, fs, pitch, coefficients, trusted, outage=(), settings=()):
    """Offline batching only. The native loop consumes exactly one row per commit."""
    epoch = Epoch(fs)
    frames = np.empty((len(q), 20))
    frames[:, 0], frames[:, 1], frames[:, 2:6] = pitch, m5, q
    frames[:, 6:18] = coefficients
    frames[:, 18], frames[:, 19] = 1, trusted
    for lo, hi in outage:
        frames[lo:hi, 18] = 0
    output = np.empty_like(frames)
    epoch.run(frames, output)
    logs = []
    start = 0
    for end in np.flatnonzero(output[:, 10]) + 1:
        r = output[end - 1]
        end = int(end)
        done = end + 16
        selection = output[done] if done < len(q) else None
        require(
            np.array_equal(
                output[end : min(end + 17, len(q)), 9], np.arange(min(17, len(q) - end))
            ),
            "chronological candidate trials",
        )
        logs.append(
            dict(
                start=start,
                end=end,
                conditioned=bool(r[12]),
                bridge_ok=bool(r[13]),
                available=bool(r[14]),
                realized_ok=bool(r[11]),
                observable=bool(frames[start:end, 18].all()),
                trusted=trusted,
                selection_tick=done if selection is not None else None,
                veto=bool(selection[15]) if selection is not None else None,
                scale=float(selection[16]) if selection is not None else None,
            )
        )
        start = end
    return delivered(output[:, 4:8], m5, settings), output[:, 8], logs, output[:, :4]
