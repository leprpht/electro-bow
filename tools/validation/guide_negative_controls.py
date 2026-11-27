"""Exercise the maintained renderer and scorer at their normal failure boundaries."""

import json
import os
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

import guide_qualification as guide
from guide_audit_build import build


def main():
    with TemporaryDirectory() as tmp:
        guide.OUT = Path(tmp)
        guide.BINARY = guide.OUT / "renderer"
        build(guide.BINARY, late_relaunch=True)
        try:
            guide.dynamic(48000, 196, "bend")
        except RuntimeError as error:
            if "guide launch count after 96001 ticks" not in str(error):
                raise
        else:
            raise AssertionError("two-second relaunch survived full-length scoring")
        print("actual second launch rejected at tick 96000")
        build(guide.BINARY)
        entry, rows = guide.render(48000, 440)
        guide.render = lambda *args, **kwargs: (entry, rows)
        try:
            guide.static_case(48000, 196, 1.8)
        except AssertionError as error:
            _, _, stats = error.args[0]
            if not (1390 < stats["cents"] < 1410):
                raise
            print(f"440 Hz labeled 196 rejected at {stats['cents']:.2f} cents")
        else:
            raise AssertionError("wrong pitch passed")
    for flags, env in [(["-O"], {}), ([], {"PYTHONOPTIMIZE": "1"})]:
        result = subprocess.run(
            [sys.executable, *flags, str(Path(guide.__file__))],
            capture_output=True,
            check=False,
            env=os.environ | env,
        )
        if (
            not result.returncode
            or b"requires enabled Python assertions" not in result.stderr
        ):
            raise AssertionError("optimized scorer started")
    zero = guide.decay_levels(0.2, 0.0)
    if zero != {
        "start_rms": 0.2,
        "end_rms": 0.0,
        "delta_db": -240.0,
        "unfloored_delta_db": None,
    }:
        raise AssertionError(zero)
    json.dumps(zero, allow_nan=False)
    for ratio in [0.0, 1e-90, 1e-12]:
        if guide.db(ratio) != -240:
            raise AssertionError(ratio)
    print("optimized invocation and environment rejected; zero and subfloor = -240 dB")


if __name__ == "__main__":
    main()
