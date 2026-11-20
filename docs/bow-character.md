# Bow Character

Branch: `feature/bow-character`. The production algorithm is safe Rust in
`rust/dsp/src/character/mod.rs`, called after Attack and before band recombination.
The signal chain is original mono/stereo guitar → complementary filter bank →
Attack → Bow Character → sum. No MIDI, pitch detection, event classification,
voice separation, synthesized strings, or STK Bowed runs in this chain. The
production target no longer links aubio or STK; their older component tests remain.

## One coordinated control

Bow Character is normalized 0–1 with 20 ms parameter smoothing. Zero retains the
smooth Attack-processed signal exactly; middle settings introduce body and modest
texture; maximum increases spectral responsiveness and friction. Attack remains
independent. For a soft onset, use Attack above zero; Character alone does not
turn a dry pick into a slow onset.

Each band has a stereo-linked peak envelope with 15 ms decay and an 80 ms slower
reference. Their level-relative ratio drives the spectral balance. Middle bands
receive bounded, changing emphasis while high-band energy is gently reduced.
Low-band samples are untouched by Character, preserving low fundamentals.
Unlike fixed EQ, the balance follows the evolving envelope history.

Friction is multiplicative: a deterministic pseudorandom stream, smoothed around
1.8 kHz (limited at low sample rates), modulates existing upper-band samples.
Depth grows quadratically with Character and follows the envelope ratio. There
is no additive noise source; zero band samples remain exactly zero. This keeps
the texture tied to the input's timing and level. Filtering can retain short
normal IIR tails after the original input ends; there is no synthetic sustain.

Middle-band nonlinear shaping is `x * (1 - 0.03*a*u²/(1+u²))`, with
`u=x/linkedEnvelope`. Its departure from linear is bounded below 3%, and it is
not applied to the low/high bands. This provides weak harmonic redistribution
without broadband clipping. It still creates some intermodulation on chords;
it is deliberately limited rather than claimed to be absent.

A second, slow filter on the random sequence creates bounded irregular spectral
movement, at most ±1.5% and ±2% in the two middle bands at maximum. There is no
periodic LFO, variable delay, or pitch modulation. Both stereo channels share
control values/randomness; identical stereo input remains identical.

Rejected approaches include an independent noise bed (audible without input),
strong broadband saturation (chord intermodulation), periodic modulation (obvious
chorus/vibrato), and static EQ alone (no temporal response). This implementation
is a conservative input-derived character processor, not a physical bow model.
It does not manufacture sustained musical content after the guitar decays.

## Control and sessions

The UI exposes Attack and Bow Character; obsolete bow controls are hidden. The
native bridge validates finite control values and clamps Character to 0–1. New
sessions serialize version 2 with Character appended. Version 1 sessions still
load, defaulting Character to zero. Invalid/truncated version-2 payloads are
rejected. No host-automation parameter system was added; controls use the
existing UI bridge and plugin state mechanism.

## Audio comparisons

Generate reproducible eight-second, mono 48 kHz PCM16 WAVs:

```sh
cargo run -p electrobow-dsp --release --example character_comparison -- build/bow-audio
```

The example generates a **synthetic guitar-like fixture**, not a recording:
quiet then louder harmonic plucks/chords, followed by approximately 13 Hz fast
picking. The synthesis is confined to the offline example, never the plugin.
Output files share the same source, Attack=250, and recording gain; no loudness
normalization is applied. Only Character changes between processed files.

| File in `build/bow-audio/` | Character | RMS | Peak |
| --- | ---: | ---: | ---: |
| `00-input-synthetic.wav` | unprocessed | — | — |
| `01-smooth.wav` | 0 | 0.05128494 | 0.20995954 |
| `02-textured.wav` | 0.5 | 0.05373088 | 0.21579082 |
| `03-rough.wav` | 1 | 0.05618700 | 0.22506167 |

The generator is committed; generated audio is kept in the ignored build tree.
These support comparison, not a claim that the result has been perceptually
validated as a bowed instrument. Recorded electric-guitar listening tests are
still needed for subjective tuning, especially pickup noise and dense chords.

## Tests and limits

Rust tests cover chord/fast-picking fixtures at 8, 44.1, 48, 96, and 192 kHz,
three Character settings, mono signal processing, stereo matching, silence,
reset reproducibility, changing chunk sizes, and tenfold playing-level scaling
within `1e-7` sample error. The C++ integration suite checks actual UI-parameter
routing, session version 2 roundtrip, version 1 migration, and oversized blocks.
The editor smoke test checks the two-control UI and native Character values.

The real-time allocator test includes Attack and maximum Character over 1,000
stereo blocks: no allocations, reallocations, or frees. Coefficients are prepared
offline, state is fixed-size, and all processing is continuous without locks.

At maximum Character, a 900/1300 Hz two-tone test measures **−35.392 dBr** energy
outside the two fundamentals, including intentional texture and spectral motion.
The regression limit is 5% residual RMS. This is a test-specific artifact budget,
not a universal IMD/aliasing guarantee. There is no oversampling; bounded drive
and avoiding high-band nonlinearity limit aliasing but cannot eliminate it.

Algorithmic buffering latency remains **0 samples**. Parameter/envelope response
times do not delay audio. Minimum Character retains prior Attack behavior and
minimum Attack/Character retains the original filter-bank reconstruction.

Run validation with:

```sh
cargo test --locked --offline -- --nocapture
cargo clippy --all-targets --locked --offline -- -D warnings
cmake --build build --config Release --parallel 8
ctest --test-dir build --build-config Release --output-on-failure
```

## CPU measurements

Apple M2 Pro, macOS arm64 Release, stereo 48 kHz, maximum Attack and Character,
median of seven batches using the integration benchmark:

| Block frames | One-core CPU | Time per stereo frame |
| ---: | ---: | ---: |
| 16 | 0.2029% | 42.26 ns |
| 64 | 0.1997% | 41.60 ns |
| 256 | 0.1896% | 39.50 ns |
| 1024 | 0.1963% | 40.89 ns |

Includes filter splitting, both processors, recombination, JUCE dispatch, and
benchmark checksum overhead. Excludes host/device scheduling and GUI overhead;
these are local microbenchmarks, not real-time scheduling guarantees.

Validation completed locally: all nine Rust unit tests, Clippy with warnings
as errors, all four CTest suites (including the real embedded editor), and the
macOS arm64 Release VST3 build passed. Windows/Linux and universal macOS builds
were not rerun for this change.
