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
