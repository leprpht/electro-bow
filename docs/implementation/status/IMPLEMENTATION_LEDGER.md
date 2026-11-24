# Implementation ledger

## 2026-11-23 · Reconcile the specification with the shipping processor

| Item | Record |
|---|---|
| Base revision | `20d4ee56e8f614e6544bf3456698fa5d3c096887`, branch `feature/bowed-string-engine`. Status documentation is a single working-tree change on that base. |
| Audited revision | `aa1be375020f81b223171cc8feed52b50dccb7d2` is an ancestor. Intervening changes add the seven design documents and one PNG. |
| Files changed | [SPECIFICATION_RECONCILIATION.md](SPECIFICATION_RECONCILIATION.md), [DECISION_LOG.md](DECISION_LOG.md), and this ledger. Captures, the Python environment and temporary offline probes live under ignored `build/spec-reconciliation/`. |
| Equations implemented in production | None. The offline probes evaluate M2–M4 and A1–A2, and measure M6 work/lift consequences. The current product has no Rust bow contact, waveguide, pitch acquisition or gesture controller. |
| Outcome | Migration step 1 has a production baseline for neutral, non-neutral, ABI, preset, bypass, allocation, block, CPU and packaged replay comparisons. A2 has a reproducible octave error between grid notes. V02 and V08 contain inconsistent literal predicates. An extreme positive rate can overflow the current host tail formula. The reconciliation assigns remaining measurements and fixtures to their appropriate test boundaries. |
| Next safe task | Preserve the two counterexamples as proposed V12 fixtures and review A2 candidate selection, V02 static/sliding residuals, V08 sample rounding and the staged gate ownership. Review the extreme-rate host edge separately. After agreement, step 2 can implement the isolated Rust contact and ring against V02/V03. Keep tracker redesign and production routing pending their respective reviews. |
| Unqualified work | Cross-platform binaries, a live DAW deadline run, per-callback percentiles, full target allocation/denormal measurements, live/spatial/paired-pickup recordings and V22 human listening. See individual V01–V22 dispositions. |

## Build and baseline commands

Run from the repository root. This run used macOS 26.5.1 arm64 on an Apple M2 Pro, Apple clang 17.0.0, CMake 4.4.3 Unix Makefiles, local JUCE 9.0.2, rustc 1.100.0 (0ef1bc386). Existing UI dependencies and dist assets satisfied CMake configure; the build regenerated embedded assets. For a fresh tree prepare JUCE and run `(cd UI && npm ci && npm run build)` before configure. Direct Cargo tests used the test profile. CMake used native Release, BUILD_TESTING=ON, the release Rust archive and host-target Cargo tests. [CMake setup](../../../CMakeLists.txt#L14) · [Rust build](../../../cmake/RustDsp.cmake#L60) · [Repository test wrappers and CI commands](../../../README.md#tests)

```sh
. "$HOME/.cargo/env"
git rev-parse HEAD
git merge-base --is-ancestor aa1be375 HEAD
git diff --name-status aa1be375..HEAD
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cmake -S . -B build -DJUCE_PATH="$PWD/JUCE" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build --config Release --parallel 6
ctest --test-dir build -C Release --output-on-failure
rg 'CPU stereo' build/Testing/Temporary/LastTest.log
bundle=build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3
nm -gU "$bundle/Contents/MacOS/ElectroBow" | rg '_GetPluginFactory|_eb_'
shasum -a 256 "$bundle/Contents/MacOS/ElectroBow"
```

The repository also provides `python3 run_tests.py --verbose` for configure/build/CTest and `python3 run_tests.py --no-build --list` for discovery. CI builds with `build.py` and runs CTest with `--no-tests=error`.

For the offline binary tests the system Python 3.9.6 had arm64 wheels available. These are local measurement dependencies. The plugin uses the existing dependency-free Rust crate.

```sh
audit=build/spec-reconciliation
mkdir -p "$audit"
/usr/bin/python3 -m venv "$audit/py"
"$audit/py/bin/python" -m pip install --only-binary=:all: pedalboard==0.9.17 soundfile==0.13.1 numpy==2.0.2 scipy==1.13.1
"$audit/py/bin/python" tools/render_guitar_comparisons.py "$bundle" "$audit/recorded"
shasum -a 256 "$audit"/recorded/*.wav
sed -n '/^```python$/,/^```$/p' docs/implementation/DSP_VALIDATION_PLAN.md | sed '1d;$d' | python3
```

The Python renderer uses the absolute VST3 path, raw DI gain, four explicit preset/input combinations and 127-frame processing, then reloads state and compares 17/1024-frame replays. The current tool adds 1 s silence to its four recordings. The synthetic baseline probe below adds a full 1.3 s after its final impulse. [Renderer, lines 43–72](../../../tools/render_guitar_comparisons.py#L43)

## Observed results

| Command or measurement | Result on this checkout |
|---|---|
| Git ancestry and diff above | Ancestor check exit 0. Eight added documentation/figure files. No difference in source, public header, vendors, tests or build inputs between pin and base. |
| Cargo fmt and Clippy above | Both exited 0. Formatting check passed. Clippy reported no warnings. |
| Cargo test | Exit 0. 59 unit and 3 integration passed, 0 failed, 0 ignored, 0 doctests. Includes zero Rust heap operations across 1000 prepared active-ABI blocks and reset on the existing DC fixture. |
| CMake configure and build | Both exited 0. Built native Release VST3, final ad-hoc signing/verification and test executables. Module manifest identifies ElectroBow 1.6.0 Fx, component CID ABCDEF019182FAEB456C6362456C6277. Mach-O SHA-256 `e2b752927d025a1fe3c90aa093e8bf115685858034e824f027e650d221135fa6`. Export listing contains GetPluginFactory, all five eb_instrument_* functions and all fourteen legacy functions. |
| CTest | Exit 0, 4/4 passed. ElectroBowRustTests 4.99 s, ElectroBowFilterBankTests 1.11 s, ElectroBowPitchDetectorTests .46 s, ElectroBowEditorSmokeTests 2.05 s. Total 8.62 s. |
| Existing Release processor benchmark inside CTest | Stereo 48 kHz, blocks 16/64/256/1024: median .218825/.215758/.213136/.212999% of one core. At 64 frames, 44.95 ns/frame. Seven one-second active-input batches after 100 warmup blocks, including refill and dispatch. Reference V21 deadline percentiles are still outstanding. [Benchmark](../../../tests/FilterBankTests.cpp#L699) |
| Recorded VST3 renderer | Exit 0, four dry/wet pairs. Finite, peak ≤.980001 FS, zero reported latency, reloaded output within 1e−6 FS of the original. Input/output hashes below. |
| Published low-E reference block | Exit 0, both six-second assertions passed for final-second mean-free uncolored velocity. Expected .228131 m/s driven and .000144 m/s lifted, each within .000002. |
| Fixed V01 numeric probe (`baseline.py` below) | 13 rates (1/2/4/8/22.05/32/44.1/48/88.2/96/176.4/192/384 kHz), mono and anti-phase stereo. No random input. C ABI separate and in-place, zero-frame null calls between chunks, and packaged VST3. Worst neutral error 2.98023224e−8 FS, limit 2e−7; reported latency 0. Native CTest checks bypass and v1–v5 separately. |
| Non-neutral cross-check from same probe | Mono 48 kHz/127, controls (50,.5,.5,.5,.5,.5), initial .1 s silent, 2 s 110 Hz pluck (peak .35, decay .28 s), .5 s silent, .25 FS impulse, 1.3 s silent. Max difference from dry .2894026 FS. Packaged VST3 vs its own eb_instrument_process max 1.1712931e−38 FS. Final .30 s peak exactly 0 in both. Tiny differences are at subnormal scale; the JUCE callback installs a denormal guard. |
| Extreme-rate tail calculation | `python3 -c 'print(1200.0/1e-307)'` printed `inf`. The processor rate predicate, host rate setter and wrapper’s infinite-tail branch are identified in the reconciliation. No extreme-rate DAW render was attempted. |
| Independent indexing and contact arithmetic (`math_probe.py`) | 100001 u values at highest normalized frequency: max absolute phase error .00108095188 samples per arm. A 16-slot impulse probe matched lags 1, 3.25 and 14 over four wraps. Independent eager vs lazy reads agree exactly across three 743-slot wraps, nonzero stale storage, launch FIR offsets and a 440→65 Hz retune at sample 100. Upper q=.0855222393351, j=.0544777606649. The static counterexample to a universal sliding residual gives .025 m/s. |
| Isolated spectrum and energy (`reference.py`) | Instrumented the published Python oracle for three-second renders, all 90 V04 spectra/RMS comparisons passed. Minimum η=.990289. Third second at nominal 196 Hz had 196 slip onsets, duty .13475. Seven-second lift at 65.41/82.41/440 Hz changed RMS −68.38/−86.31/−471.79 dB versus early window, continuously driven +.023/−.155/−.002 dB. At 440 Hz pressure-band fractions .09993/.11370/.12217. Speed RMS .13390/.35484 m/s. Research calculations only. |
| Independent stationary tracker probe (`pitch_probe.py`) | Direct f64 differences, HP/FIR and decimation; 112 named rate/note/spectrum combinations at peak .35 FS met clean stationary error bounds with no missing frames. Two additional valid inputs selected the lower octave on 200/200 and 199/199 frames. Their median absolute errors were 1200.03 and 1199.85 cents. This probe grades the A2 estimator by known input frequency; it omits the voice controller, noise and pluck acquisition. |

The digests below identify the exact audio passed to and returned from processing, before PCM24 file quantization. They hash contiguous little-endian f32 sample payloads. The recorded pairs use the tool’s named excerpts, taper, silence padding and controls. To reproduce their payload hashes, run the renderer with its final WAV writer replaced by a checksum reporter for this Python invocation.

```sh
"$audit/py/bin/python" - <<'PYTHON'
import hashlib
import sys
from pathlib import Path
import tools.render_guitar_comparisons as renderer
def hash_audio(path, audio, rate, **kwargs):
    print(Path(path).stem,rate,hashlib.sha256(audio.astype('<f4').tobytes()).hexdigest())
renderer.sf.write = hash_audio
sys.argv = ['render','build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3',
            'build/spec-reconciliation/recorded']
renderer.main()
PYTHON
```

| Fixture | Input SHA-256 | Wet SHA-256 |
|---|---|---|
| Fixed pluck/impulse, 48 kHz | `5206d315d476ce363fb99c0e90884b68f599a3a3e4626e33bfad5c7322588709` | ABI `c4f241531583ac391ec9419a8c21fc7b5f27465e78d93dfcb0aaa64e12fae6de`, VST3 `c7251cd11c37048e8cbd2a645897bcd14e0b87dfec1f650c2797bd5cb5658a94` |
| Low E swell, 44.1 kHz | `f8fe17cdb6a5d3666211b88c0e2736327c0e5d9377723a03879be60c1a2e78f7` | `7451485a58995ce2822486f74ee7044b62b849291c2a3a68dcf417505b867f6e` |
| Staccato, 44.1 kHz | `c930baa6a1a51a55e87f2b6a5a1e5c3cbdecc87fe781635cf2d8c5277a9c814a` | `62ef8979ec0d219b332e1fbaafc3ef649d1af5928614167b85db156940a87b85` |
| Layered D, 44.1 kHz | `04cc6a57a91bb197c60d820c9d95c4e9f0499e42e99fee24470c76241ab50909` | `fc34b0d58f53b35bf916384f5500e42b39a527ab15a9488352c22fdccd7c8ae0` |
| Riff, 48 kHz | `a41b87b4de6e0bbe28bb269683146da9ca0e0deaaf8c2af3a534604abf792898` | `ca30a37f34e6ad71e9e8f0e2993ffde1fbb40a933f46dcbc60ff66773e65a9a0` |

## Replay the audit probes

The pitch probe takes the first decimated output at zero-based host index D−1 and the first hop at analysis index H−1. Its six harmonics use amplitudes 1/k and phases .13k². The target leaves these fixture choices partly open. Both counterexamples have a qualifying trough each hop, so the established-track fallback cannot change the selected octave.

The four offline probes below are included with the ledger for replay from a clean checkout. Extract their named blocks into the ignored build directory, then run them from the repository root. The binary probe uses this macOS bundle layout. The math, scalar oracle and pitch probes are independent of JUCE.

```sh
python3 - <<'PY'
from pathlib import Path
import re
ledger = Path("docs/implementation/status/IMPLEMENTATION_LEDGER.md").read_text()
out = Path("build/spec-reconciliation")
out.mkdir(parents=True, exist_ok=True)
for name, code in re.findall(r"^```python\n# file: (\w+\.py)\n(.*?)^```$", ledger, re.M | re.S):
    (out / name).write_text(code)
PY
audit=build/spec-reconciliation
"$audit/py/bin/python" "$audit/baseline.py"
python3 "$audit/math_probe.py"
"$audit/py/bin/python" "$audit/reference.py"
"$audit/py/bin/python" "$audit/pitch_probe.py"
```

<details>
<summary>Packaged and ABI baseline</summary>

```python
# file: baseline.py
from pathlib import Path
import ctypes as ct
import hashlib
import numpy as np
import soundfile as sf
from pedalboard import load_plugin

bundle = Path("build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3").resolve()
out = Path("build/spec-reconciliation")
out.mkdir(parents=True, exist_ok=True)
lib = ct.CDLL(str(bundle / "Contents/MacOS/ElectroBow"))
ptr = ct.POINTER(ct.c_float)
array = ct.POINTER(ptr)
handle = ct.c_void_p
class Controls(ct.Structure):
    _fields_ = [(name, ct.c_double) for name in
                ("attack_ms", "character", "noise", "saturation", "spectral_movement", "out")]
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
            assert lib.eb_instrument_process(h, a, b, min(127,x.shape[1]-i), settings) == 1
    finally:
        lib.eb_instrument_destroy(h)
    return y

def render(x, fs, settings):
    fx = load_plugin(str(bundle))
    for name, value in zip(("attack_ms","bow_character","noise","saturation","spectral_movement","out"),
                           (getattr(settings, name) for name, _ in Controls._fields_)):
        setattr(fx, name, value)
    y = fx.process(x, fs, buffer_size=127)
    assert fx.reported_latency_samples == 0
    return y

def fixture(fs):
    t = np.arange(2*fs)/fs
    angle = 2*np.pi*110*t
    p = (1-np.exp(-t/.001))*np.exp(-t/.28)*sum(
        np.sin(k*angle+.13*k*k)/k for k in range(1,9) if 110*k < .45*fs)
    p = (p*.35/max(abs(p))).astype("float32")
    # A .35 FS peak pluck, an isolated impulse, and exact digital silence.
    return np.concatenate((np.zeros(fs//10,"f"),p,np.zeros(fs//2,"f"),
                           np.array([.25],"f"),np.zeros(round(1.3*fs),"f")))

neutral = Controls(0,0,.5,.5,.5,.5)
worst = 0.
for fs in (1000,2000,4000,8000,22050,32000,44100,48000,88200,96000,176400,192000,384000):
    p = fixture(fs)
    for x in (p[None,:],np.stack((p,-p))):
        for y in (abi(x,fs,neutral,False), abi(x,fs,neutral,True), render(x,fs,neutral)):
            error = float(np.max(abs(y-x)))
            assert error <= 2e-7, (fs,len(x),error)
            worst = max(worst,error)
print("worst neutral FS",worst)
fs = 48000
x = fixture(fs)[None,:]
settings = Controls(50,.5,.5,.5,.5,.5)
a, y = abi(x,fs,settings,False), render(x,fs,settings)
assert np.max(abs(a-y)) <= 1e-6 and np.max(abs(y-x)) > .01
for name, samples in (("input",x),("abi",a),("vst3",y)):
    path = out / ("fixed-"+name+".wav")
    sf.write(path,samples.T,fs,subtype="FLOAT")
    print(name, hashlib.sha256(samples.astype("<f4").tobytes()).hexdigest(),
          "tail peak",np.max(abs(samples[:,-round(.30*fs):])))
print("wet path difference",np.max(abs(a-y)), "dry/wet difference",np.max(abs(x-y)))
```

</details>

<details>
<summary>Ring and contact arithmetic</summary>

```python
# file: math_probe.py
from math import atan2, ceil, cos, exp, floor, pi, sin
omega = 2*pi*1320/32000
print("phase max", max(abs(atan2(u*sin(omega),1-u+u*cos(omega))/omega-u)
                       for i in range(100001) for u in [i/100000]))
# Inject an impulse after each complete read. Exercise taps through repeated wraps.
for delay in (1, 3.25, 14):
    ring = [0.] * 16
    for n in range(64):
        m = floor(delay)
        u = delay-m
        tap = (1-u)*ring[(n-m)%16] + u*ring[(n-m-1)%16]
        assert tap == (1-u)*(n==m) + u*(n==m+1)
        ring[n%16] = float(n==0)
beta, fs = .127, 48000
Q = [0,.005,.01,.02,.04,.08,.16,.32,.64,1.28]
mu = [.35+.85*exp(-q/.01)+.45*exp(-q/.1) for q in Q]
lo, hi = .08+.1*mu[5], .16+.1*mu[6]
q = .08+.08*(.14-lo)/(hi-lo)
print("upper slip, impulse",q,.14-q,"static residual",.1*mu[0]-.14)
# Independent lazy vs eager integer histories, including stale nonzero storage,
# lengthening both arms at sample 100, wraparound and saturation of seed age.
size = ceil(fs/65)+4
for neck in (False,True):
    def seed(t):
        phase = t/(fs/440) + ((1-beta) if neck else 0)
        return (-1 if neck else 1)*(.1/beta)*(phase-floor(phase)-.5)
    eager = [seed(t) for t in range(-size,0)]
    lazy = [3.25]*size
    def check(n,delay):
        m = floor(delay)
        u = delay-m
        reads=[]
        for lag in (m,m+1):
            age=min(n,size)
            a=lazy[(n-lag)%size] if lag<=age else seed(age-lag)
            reads.append((a,eager[(n-lag)%size]))
        assert (1-u)*reads[0][0]+u*reads[1][0] == (1-u)*reads[0][1]+u*reads[1][1]
    for n in range(3*size):
        P=fs/(440 if n<100 else 65)
        check(n,(1-beta)*P if neck else beta*P-1)
        if not neck:
            check(n,beta*P/2)
            if n==0:
                check(n,beta*P-1+1)
                check(n,beta*P-1+2)
        eager[n%size]=lazy[n%size]=sin(.731*n)
print("lazy taps agree",size)
```

</details>

<details>
<summary>Isolated string measurements</summary>

```python
# file: reference.py
from itertools import product
from pathlib import Path
import numpy as np
from scipy.signal import find_peaks

# Instrument the pinned Python oracle off-line. Keep its ring and contact arithmetic intact.
doc = Path("docs/implementation/DSP_VALIDATION_PLAN.md").read_text()
source = doc.split("```python\n")[1].split("assert abs(final_bridge")[0]
edits = {
    "final_bridge_rms(lift)": "trace(fs, frequency, ratio, speed, lift, seconds)",
    "fs, frequency, speed, ratio, beta = 48000, 82.41, .10, 1.8, .127": "beta = .127",
    "range(6*fs)": "range(seconds*fs)",
    "        if n >= 5*fs:\n            tail.append(read(bridge, n, beta*period/2))":
        "        tail.append((read(bridge, n, beta*period/2), sliding))",
    "return pstdev(tail)": "return np.array(tail).T",
}
for old, new in edits.items():
    assert source.count(old) == 1
    source = source.replace(old, new)
exec(source)

min_eta = 1
for fs, f, ratio in product((32000,44100,48000,96000,192000),
                            (65.41,82.41,196,440,880,1318.51), (1.2,1.8,2.4)):
    p, branch = trace(fs, f, ratio, .1, False, 3)
    y = p[-fs:]
    spectrum = abs(np.fft.rfft((y-y.mean())*np.hanning(fs), 8*fs))
    peak = 1 + np.argmax(spectrum[1:])
    l, m, h = np.log(spectrum[peak-1:peak+2])
    measured = (peak+.5*(l-h)/(l-2*m+h))/8
    hz = np.arange(len(spectrum))/8
    k = np.rint(hz/measured)
    mask = (k>=1) & (k*measured<fs/2) & (abs(hz-k*measured)<=np.maximum(2,.001*k*measured))
    eta = np.sum(spectrum[mask]**2)/np.sum(spectrum[1:]**2)
    min_eta = min(min_eta,eta)
    assert abs(1200*np.log2(measured/f)) <= 12 and eta >= .9
    assert y.std() > .04 and abs(20*np.log10(y.std()/p[-2*fs:-fs].std())) <= 2
    if f <= 880:
        peaks = find_peaks(spectrum)[0]
        order = np.rint(hz[peaks]/f)
        near = abs(hz[peaks]-order*f) <= np.maximum(2,order*f*(2**(15/1200)-1))
        loud = spectrum[peaks] > spectrum[peak]*10**(-36/20)
        assert len(set(order[(order>=2) & near & loud])) >= 3
    if (fs,f,ratio) == (48000,196,1.8):
        slips = branch[-fs:] != 0
        print("nominal onsets/duty", np.sum(slips & (branch[-fs-1:-1]==0)), slips.mean())
print("90 spectra; minimum eta", min_eta)
fs = 48000
for f in (65.41,82.41,440):
    for lift in (True,False):
        p, _ = trace(fs,f,1.8,.1,lift,7)
        print(f,lift,20*np.log10(p[-9600:].std()/p[38400:48000].std()))
for ratio, speed in ((1.2,.1),(1.8,.1),(2.4,.1),(1.8,.06),(1.8,.16)):
    p = trace(fs,440,ratio,speed,False,3)[0,-fs:]
    power = abs(np.fft.rfft((p-p.mean())*np.hanning(fs)))**2
    print(ratio,speed,p.std(),power[1500:6001].sum()/power[1:].sum())
```

</details>

<details>
<summary>Upper-register pitch counterexamples</summary>

```python
# file: pitch_probe.py
from itertools import product
from math import ceil, floor
import numpy as np
from numpy.lib.stride_tricks import sliding_window_view
from scipy.signal import lfilter

def stationary(fs, freq, partials):
    D = fs//8000
    fa = fs/D
    H, W = round(.005*fa), ceil(fa/65)
    K, L = W+1, max(2,floor(fa/1320))
    j = np.arange(32*D+1)
    fir = np.blackman(len(j))*np.sinc(.68*(j-16*D)/D)
    hp = np.exp(-2*np.pi*30/fs)
    t = np.arange(2*fs)/fs
    x = sum(np.sin(2*np.pi*i*freq*t+.13*i*i)/i for i in range(1,partials+1))
    x = np.r_[np.zeros(round(.1*fs)),x*.35/np.max(abs(x))]
    z = lfilter(fir/fir.sum(),[1],lfilter([hp,-hp],[1,-hp],x))[D-1::D]
    errors, missing = [], 0
    for end in range(H-1,len(z),H):
        if end < len(z)-fa:
            continue
        spans = sliding_window_view(z[end-W-K+1:end+1],W)
        d = np.sum((spans[-1]-spans[::-1])**2,axis=1)
        cm = np.r_[1,d[1:]*np.arange(1,K+1)/np.cumsum(d[1:])]
        lag = next((i for i in range(L,K) if cm[i]<=cm[i-1] and
                    cm[i]<cm[i+1] and cm[i]<.15),None)
        if lag is None:
            missing += 1
            continue
        a,b,c = d[lag-1:lag+2]
        e = .5*(a-c)/(a-2*b+c) if a-2*b+c>1e-12 else 0
        f = fa/(lag+np.clip(e,-.5,.5))
        if not 65*.995 <= f <= 1320*1.005:
            missing += 1
            continue
        errors.append(abs(1200*np.log2(np.clip(f,65,1320)/freq)))
    return np.array(errors),missing

rates = (32000,44100,48000,88200,96000,176400,192000)
notes = (65.41,82.41,110,196,329.63,440,880,1318.51)
for fs,f,partials in product(rates,notes,(1,6)):
    err,missing = stationary(fs,f,partials)
    assert not missing and np.median(err)<=12 and np.percentile(err,95)<=25 and max(err)<600
print("112 clean named-grid cases passed")
for fs,f in ((48000,1230.77),(34300,1318.51)):
    err,missing = stationary(fs,f,6)
    print(fs,f,"octave errors/accepted, rejected",sum(err>600),len(err),missing,
          "absolute cents median/p95",np.percentile(err,[50,95]))
```

</details>
