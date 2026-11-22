# ElectroBow timbral instrument

## Phase 1 — audit of v1.5.4 (7971ab1)

The JUCE callback snapshots eleven host parameters once per block, chunks oversized buffers and calls the Rust panel entry point. The TypeScript editor shows six. Attack sets a linked two-region swell. Bow Character shifts the band gains, waveshapes the low mids and multiplies the three texture depths. Noise amplitude-modulates filtered noise with the low bands. Saturation adds another low-mid soft knee. Spectral Movement sweeps a triangle-wave tilt. Out trims the result before the 0.98 linked sample-peak protector.

Pressure, Speed, Friction, Sustain and Resonance also affect every new instance through host-only parameters. The signal encounters level-dependent makeup and low-band friction, then attack, character shaping, texture shaping, recombination, feedback delays, output trim and limiting. Multiple nonlinearities act on the same overlapping notes. Products between chord tones and gain changes on repicks can therefore accumulate. Makeup can lift a falling chord by up to 10 dB, while the edge band can lose 84% of its level from Character alone. A sample-peak ceiling contains peaks but cannot undo upstream intermodulation or spectral loss. These are mechanisms identified in code. There was no reproducible block-boundary click in the baseline checks.

With the v1.5.4 VST3 at factory settings, the repository's staggered recorded D chord peaked at 0.566 dry and 0.450 wet. Wet RMS was 2.53 dB above dry. Zeroing hidden sustain, pressure, friction and resonance gave 1.02 dB above dry at the same faceplate settings. Measured with Pedalboard, 64-frame blocks, original recording gain. The 54 baseline Rust tests and the processor integration executable passed. The native 48 kHz stereo benchmark used 0.270% of one M2 Pro core at 64 frames.

Reuse the complementary crossover, level-relative Attack, linked output protection, channel layout, block chunking, host bypass ramp and editor. Replace the instrument's sound-generating stages and meter the raw input independently of expression processing. Preserve old EBOW chunk decoding and the six surviving parameter IDs. The existing C API has standalone callers and tests. Keep its legacy behavior available separately from the new plugin callback. aubio and STK are linked by historical test targets only.

## Phase 2 — architecture decision

Use the input to excite a fixed bank of short, inharmonic resonances. The intended sound is a pluck that opens into a hollow, vowel-like body. A low string and an overlapping treble string can excite different parts of that body at the same instant. The resonators retain the input's timing and decay in milliseconds. Their frequencies are part of the instrument design.

| Candidate | Sound and real-time tradeoff | Decision |
| --- | --- | --- |
| Windowed FFT spectral-envelope transfer | Flexible harmonic remapping, with window latency and transient smearing that are particularly apparent on muted picks | Use the existing causal crossover for spectral analysis |
| Parallel damped modes | Fixed formants respond to a whole chord, admit a bounded steady-state response and cost a fixed number of operations per frame | Use two unity-peak bandpass modes per crossover band |
| Moving delay taps or granular overlap | Can blur and detune a chord, needs delay storage and interpolation or overlap windows | Obtain short temporal memory from the modes' impulse responses |
| Broadband waveshaping with makeup | Intermodulation changes with chord density and a falling envelope can change the next attack | Confine a single soft knee to the mid-band excitation |

### Signal flow and control law

1. Sanitize the input, measure the linked input peak and split into the existing four complementary bands.
2. Apply the existing Attack swell to the two stereo-linked pairs. The result feeds both the direct and resonant sound. Zero milliseconds passes the pick. A 500 ms setting progressively opens the excitation.
3. Form one excitation per band. Saturation blends in a symmetric soft knee on the two middle bands. Noise scatters a small portion of the signed low-band signal into the upper excitations with one shared, band-limited random modulator. A silent channel supplies zero excitation.
4. Drive two parallel bandpass modes per band. Starting centers in Hz are 114/181, 440/690, 1060/1780 and 2820/4100. Scale centers together at low sample rates. Q ranges from 2 to 5.5. Each mode has unit peak gain. Spectral Movement varies the relative pickup of the two sets with a smooth sub-Hz periodic weight shared by both channels. Frequencies and damping stay fixed during automation. The result moves between two formant patterns without shifting the string pitches.
5. Bow Character blends the shaped direct signal with the modal pickup. Retain a direct contribution at full depth to keep notes between modes audible and supply immediate output. The resonant path runs continuously so turning up Character reveals the current vibration. With Character at zero, Noise, Saturation and Movement are inaudible.
6. Out applies the existing smoothed linear 0–2 trim followed by linked 0.98 sample-peak protection. Avoid automatic loudness makeup. Smooth depth changes in sample time and initialize from the first valid control snapshot.

All coefficients are prepared before rendering. Store two filter histories per mode and channel. The work per frame is independent of pitch count and host block size. Publish exactly the six faceplate parameters, preserve their existing physical units and VST3 IDs, and write a six-value v5 state. When reading v1–v4, validate the entire chunk before mapping surviving fields. Older presets will acquire the new timbre.

Validate neutral reconstruction, first-frame response, modal peaks and release, stereo isolation, silence, overload, rapid automation, all control interactions, buffer partitioning, reset/reload and heap activity. Render recorded low strings, staggered chords and a riff through the packaged VST3. Measure release CPU at several buffer sizes and distinguish actual host playback from offline renders.

## Phases 3 and 4 — implementation and calibration

The production handle is `EbInstrument`. It owns its crossover buffers, Attack envelopes, eight stereo resonators, shared excitation and pickup modulation, meter and output protector. The older `EbFilterBank` ABI still serves direct library callers. Each handle allocates only the stages it uses.

In the first implementation, full Character reduced a recorded low E by 7.39 dB RMS. Tuning the two lower pairs to 102/193 and 315/650 Hz, broadening their bandwidths, and retaining 34% direct signal at full Character filled the gap around 200–300 Hz. Final Q values are 1.6, 2.6, 4.5 and 5.5. The upper pairs retain their design frequencies. The direct/body blend follows `m = 0.66 × c × (2 − c)` and a fixed gain of `1 + 0.90 × m`. This curve depends only on the dial position. A second pick cannot inherit extra makeup from the decay of a previous note.

Bow Character, Noise, Saturation and Spectral Movement each use 15 ms smoothing. Attack and Out use their own time-domain smoothing. The tail reported to the host is 300 ms. A full-depth impulse produces a response on the first sample and falls below 1e-8 FS in the final 10% of the reported tail at 4, 48 and 192 kHz.

## Validation on macOS, November 22, 2026

The universal Release VST3 passed strict code-signature verification and pluginval 1.0.4 at strictness 10 on Apple Silicon and under Rosetta. The runs exercised editor open during processing, parameter fuzzing, automation, restoring state, bus layouts, and 44.1/48/96 kHz processing. The host reported zero samples of latency. The standard host bypass appears alongside the six instrument dials.

Both native and universal builds passed all four CTest suites. The 62 Rust tests cover the shared and legacy components as well as the instrument. New checks include all 64 dial endpoint combinations, the differences between two automation timelines offset by one sample, linear addition of overlapping notes with the swell and saturation at zero, channel isolation, partitioning, reset, fault handling and zero heap operations in the instrument callback. The embedded editor reopened and wrote all six controls. All 22 browser tests passed.

The full native processor, including input refill and dispatch, used 0.241%, 0.216%, 0.218% and 0.213% of one M2 Pro core at 48 kHz stereo with 16, 64, 256 and 1024 frames respectively. Each value is the median of seven one-second batches. The earlier 64-frame baseline was 0.270%. These measurements exclude audio-device scheduling and GUI cost.

### Recorded guitar through the VST3

Measured by loading the final universal binary in Pedalboard. Values are RMS gain relative to the corresponding unprocessed recording, with the Out dial at unity and the three detail dials at noon.

| Recording | Factory (50 ms Attack) | Full Character (50 ms Attack) | Full Character (0 ms Attack) |
| --- | ---: | ---: | ---: |
| Low E, three seconds | −1.02 dB | −1.84 dB | −1.74 dB |
| D chord, four staggered recorded strings | −0.96 dB | −1.66 dB | −1.42 dB |
| Staccato, five seconds | −4.25 dB | −5.11 dB | −2.03 dB |
| Riff, up to five seconds | −0.30 dB | −0.53 dB | −0.28 dB |

The staccato comparison shows the additional attenuation from a 50 ms swell on short strokes. On all four recordings, saved-state replay matched within 1e-6 FS across blocks of 1, 17, 64, 127, 1024 and 4096 samples. Inverted stereo and dry bypass checks passed. A further 6,000 blocks of continuous randomized automation and varying input levels stayed finite and below the 0.98 sample-peak ceiling.

To reproduce the paired listening files, pass the built VST3 and an output directory to `tools/render_guitar_comparisons.py`. The script preserves the recording gain and also checks state recall and partition invariance. Use `ctest --test-dir build -C Release --output-on-failure` for the processor, editor, Rust and legacy regression suites.
