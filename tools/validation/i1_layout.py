"""Proposed contiguous repr(C) arenas. Offsets and limits are bytes; no Vec headers."""

import json
import math

from i1_motion import OUT
from o2_response import require


def layout(fs):
    da, di = int(fs // 8000), int(fs // 16000)
    w = math.ceil(fs / da / 65)
    m = math.ceil(4 * fs / di / 65)
    h = math.floor(0.005 * fs / di + 0.5)

    # All groups use 8-byte alignment, including packed flags and fixed event slots.
    def arena(fields):
        pos = 0
        result = {}
        for key, n in fields:
            result[key] = [pos, n]
            pos += n
        return result, pos

    shared, shared_n = arena(
        [
            ("A1 symmetric FIR f64", 8 * (16 * da + 1)),
            ("I1 symmetric FIR f64", 8 * (16 * di + 1)),
            ("O2 coefficients f64[6]", 48),
            ("rate HP step smooth and prepared scalars f64[16]", 128),
            ("generation tick and transport u64[4]", 32),
            ("shared control smoother f64[24]", 192),
        ]
    )
    lane, lane_n = arena(
        [
            ("HP ring f64", 8 * (32 * da + 1)),
            ("A2 history f64", 8 * (2 * w + 2)),
            ("lag and normalized differences f64", 16 * (w + 2)),
            ("I1 decimated ring f64", 8 * (m + 1 + h + 2)),
            ("monitor and complex bank Epoch", 3880),
            ("Cholesky packed f64[21]", 168),
            ("trial vector and basis f64[6] each", 96),
            ("O2 lanes four times f64[4]", 128),
            ("smoothed c f64[12]", 96),
            ("pending published and active held dB f64[12] each", 192),
            ("source projection XW W phasor and rotation complex[12] each", 768),
            ("source magnitude reliability and gain f64[12] each", 288),
            ("job start end cursor owner deadlines Hann DC and sums u64/f64[20]", 160),
            ("support frames 24 times tick serial generation hz flags u64[5]", 960),
            ("A2 A3 scalars counters filter history and ramps 128 words", 1024),
            (
                "capture tokens serial anchor expiry physical epoch and eligibility 24 words",
                192,
            ),
            ("validated gesture confirm dispose launch and stamps 24 words", 192),
            ("M6 preceding E product fade index and length 4 words", 32),
            ("indices and validity for three circular histories 12 words", 96),
            ("candidate and wet sample scratch f64[32]", 256),
            ("feature and capture finalize scratch f64[48]", 384),
        ]
    )
    control = shared_n + 2 * lane_n
    full, total = arena(
        [
            ("analysis and controls", control),
            ("four physical rings 4096 f64", 131072),
            ("band scratch 8x8192 f32", 262144),
            ("raw scratch 6x8192 f32", 196608),
            ("JUCE wet 2x8192 f32", 65536),
            ("guide scalar and ring descriptors two 1024-byte slots", 2048),
            (
                "DSP filters splitter limiter smoothing host and stream descriptors 8192-byte slot",
                8192,
            ),
        ]
    )
    return dict(
        fs=fs,
        shared=shared,
        lane=lane,
        shared_bytes=shared_n,
        lane_bytes=lane_n,
        analysis_bytes=control,
        full_layout=full,
        total_bytes=total,
    )


def check():
    # Capacity is nondecreasing between adjacent D changes, including ceil and rounded hops.
    rates = [32000, 192000] + [
        math.nextafter(float(i), 0.0) for i in range(40000, 192001, 8000)
    ]
    peak = max(map(layout, rates), key=lambda row: row["total_bytes"])
    require(
        peak["analysis_bytes"] <= 65536 and peak["total_bytes"] <= 0.8 * 1024**2,
        "prepared payload",
    )
    return peak


if __name__ == "__main__":
    (OUT / "layout.json").write_text(json.dumps(check(), indent=2) + "\n")
