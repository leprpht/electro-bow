import ctypes as ct
import hashlib
from pathlib import Path

import numpy as np
import soundfile as sf
from pedalboard import load_plugin

bundle = Path("build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3").resolve()
out = Path("build/spec-signoff/plugin-baseline")
out.mkdir(parents=True, exist_ok=True)
lib = ct.CDLL(str(bundle / "Contents/MacOS/ElectroBow"))
ptr = ct.POINTER(ct.c_float)
array = ct.POINTER(ptr)
handle = ct.c_void_p


class Controls(ct.Structure):
    _fields_ = [
        (name, ct.c_double)
        for name in (
            "attack_ms",
            "character",
            "noise",
            "saturation",
            "spectral_movement",
            "out",
        )
    ]


lib.eb_instrument_create.argtypes = [ct.c_double, ct.c_int32, ct.c_int32]
lib.eb_instrument_create.restype = handle
lib.eb_instrument_destroy.argtypes = [handle]
lib.eb_instrument_destroy.restype = None
lib.eb_instrument_process.argtypes = [handle, array, array, ct.c_int32, Controls]
lib.eb_instrument_process.restype = ct.c_int32


def abi(x, fs, settings, inplace):
    y = x.copy() if inplace else np.empty_like(x)
    src = y if inplace else x
    h = lib.eb_instrument_create(fs, 127, len(x))
    assert h
    try:
        for i in range(0, x.shape[1], 127):
            assert lib.eb_instrument_process(h, None, None, 0, settings) == 1
            pointers = ptr * len(x)
            a = pointers(*(ch[i:].ctypes.data_as(ptr) for ch in src))
            b = pointers(*(ch[i:].ctypes.data_as(ptr) for ch in y))
            assert (
                lib.eb_instrument_process(h, a, b, min(127, x.shape[1] - i), settings)
                == 1
            )
    finally:
        lib.eb_instrument_destroy(h)
    return y


def render(x, fs, settings):
    fx = load_plugin(str(bundle))
    for name, value in zip(
        (
            "attack_ms",
            "bow_character",
            "noise",
            "saturation",
            "spectral_movement",
            "out",
        ),
        (getattr(settings, name) for name, _ in Controls._fields_),
    ):
        setattr(fx, name, value)
    y = fx.process(x, fs, buffer_size=127)
    assert fx.reported_latency_samples == 0
    return y


def fixture(fs):
    t = np.arange(2 * fs) / fs
    angle = 2 * np.pi * 110 * t
    p = (
        (1 - np.exp(-t / 0.001))
        * np.exp(-t / 0.28)
        * sum(
            np.sin(k * angle + 0.13 * k * k) / k
            for k in range(1, 9)
            if 110 * k < 0.45 * fs
        )
    )
    p = (p * 0.35 / max(abs(p))).astype("float32")
    # A .35 FS peak pluck, an isolated impulse, and exact digital silence.
    return np.concatenate(
        (
            np.zeros(fs // 10, "f"),
            p,
            np.zeros(fs // 2, "f"),
            np.array([0.25], "f"),
            np.zeros(round(1.3 * fs), "f"),
        )
    )


neutral = Controls(0, 0, 0.5, 0.5, 0.5, 0.5)
worst = 0.0
for fs in (
    1000,
    2000,
    4000,
    8000,
    22050,
    32000,
    44100,
    48000,
    88200,
    96000,
    176400,
    192000,
    384000,
):
    p = fixture(fs)
    for x in (p[None, :], np.stack((p, -p))):
        for y in (
            abi(x, fs, neutral, False),
            abi(x, fs, neutral, True),
            render(x, fs, neutral),
        ):
            error = float(np.max(abs(y - x)))
            assert error <= 2e-7, (fs, len(x), error)
            worst = max(worst, error)
print("worst neutral FS", worst)
fs = 48000
x = fixture(fs)[None, :]
settings = Controls(50, 0.5, 0.5, 0.5, 0.5, 0.5)
a, y = abi(x, fs, settings, False), render(x, fs, settings)
assert np.max(abs(a - y)) <= 1e-6 and np.max(abs(y - x)) > 0.01
for name, samples in (("input", x), ("abi", a), ("vst3", y)):
    path = out / ("fixed-" + name + ".wav")
    sf.write(path, samples.T, fs, subtype="FLOAT")
    print(
        name,
        hashlib.sha256(samples.astype("<f4").tobytes()).hexdigest(),
        "tail peak",
        np.max(abs(samples[:, -round(0.30 * fs) :])),
    )
print(
    "wet path difference", np.max(abs(a - y)), "dry/wet difference", np.max(abs(x - y))
)
