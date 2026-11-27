# Stage 04 · Acoustic observation and source identity · 27 November 2026

**Disposition.** Submit one pickup-led architecture for independent review. Retain the qualified bow interaction, ring and driven-guide recurrence. The owner selected the magnetic-pickup reference in this research increment. The stored signals now support a physically different observation and measurable transfer of source differences into a held note. Bow realism, expressive attack quality and reliable perceptual recognition remain open. The shipping plugin is unchanged.

## Revision and authority audit

Entry HEAD `29cf221ec3326a56cc84dbd474ba29f34da3df5c`, branch `feature/bowed-string-engine`, clean index and working tree (`git status --porcelain` empty). The existing ignored Stage 03 and Review 3 renders were available locally. The research changes comprise offline tools, short licensed research fixtures and proposed documentation. No production Rust/C++/FFI/JUCE/UI changes.

| Evidence reviewed | Consequence for this increment |
|---|---|
| [Original source audit](../../research/EXISTING_DSP_AUDIT.md), [reference study](../../research/BOWED_STRING_REFERENCE_RESEARCH.md), M1–M7, A1–A4, migration plan, V01–V22 and AI implementation guide | Actual host uses the six-control Rust instrument, with input split, linked Attack, empirical body and linked output. Guide is isolated. Preserve direct, fallback and ABI. |
| [Reconciliation](SPECIFICATION_RECONCILIATION.md), [specification signoff](SPECIFICATION_SIGNOFF.md), [Review 00.1](REVIEW_00_1_SPECIFICATION.md), [Review 1](REVIEW_01_CONTACT.md), [Review 2](REVIEW_02_RING.md), implementation ledger and decision log | Freeze M3b and M2, previously approved numerical gates, previous-branch policy and M6 fault timing. A2 measurement is separate from its blocked candidate ordering. |
| [Stage 03 implementation](STAGE_03_WAVEGUIDE_IMPLEMENTATION.md), [Review 3](REVIEW_03_WAVEGUIDE.md), actual `friction.rs`, `ring.rs`, `waveguide.rs` and their qualification tools | Rechecked actual pre-write reads, launch seed, paired transactional commits and M5. Used emitted N and negative-time launch state, at current P, to reconstruct both rays downstream. |
| `instrument/mod.rs`, `instrument/body.rs`, `filter_bank/mod.rs`, `attack/mod.rs`, `output/mod.rs`, validation pitch-candidate code and the production control struct | The old pair of modal filters per band is a coloration on the attacked input. No current production analyzer/gesture path provides a 12-partial spectral capture. Keep future analysis and state in Rust. |
| STK/Faust pinned sources, Csound 6.18.1, primary musical-acoustics and pickup research, Q fixtures, Guitar-TECHS and Vaisvil recording | Source locations, licenses, equations and limitations are recorded in the two research reports. No implementation copied from upstream; source audio credits accompany the excerpts. |

## Executed evidence

- Regenerated **152** prescribed Rust renders, including the 60 s hold, all seven rates, bends, vibrato, pressure/speed and bow lift. Compared with the retained independent Review 3 manifest: **152/152 binary64 trace hashes and 12/12 WAV hashes identical**. Each case had one actual successful seed and zero guide faults.
- Remediated continuity in the maintained renderer. Build an instrumented copy with the guide's existing test counters enabled, a counter at actual invalidation boundaries and private ring metadata probes. Immediately after launch and every tick assert the live epoch, original period/amplitude, actual launch/fault/invalidation counts, arm open/clear events, expected cursor modulo capacity and saturated count. No pitch-window substitutions. **46,242,000 ticks and 46,242,152 snapshots**, including extra right context. The late mutation starts at 196 Hz, arms above 210 Hz at tick 57,600, relaunches inside the real guide at 96,000: rejected by the launch-count assertion after 96,001 ticks. The other 11 compiled guide mutations also failed at their nominated predicates.
- The Rust release workspace regression also passed (76 unit and 3 integration tests).
- The normal scorer rejected actual 440 Hz presented as 196 Hz at **+1399.91 cents**. Both `python -O` and `PYTHONOPTIMIZE=1` now stop acceptance execution. The 440 Hz lift start/end AC RMS are .2221719261 and 5.7191331e−25 m/s. Score is **−240 dB**, unfloored diagnostic −471.78718 dB. Exact zero yields −240 dB and JSON null for the unfloored log; a wholly silent lifted last second has no pitch estimate. The separate 10-minute stress produced 28,800,000 ticks, 600 actual launches, zero faults, 1,200 guide invalidations.
- Replayed 15 guide cases at common engineering gain through M5, linearized S5 and selected spatial/load observation. At 440 Hz M5's saw envelope error is **.1784 dB**, old S5 relative gain changes ≤.526 dB. The selected spatial path moves the error to 6.005 dB with a similar centroid. The actual first sample and 10/50 ms RMS establish that a seeded, fully voiced waveform remains under the attack. High-note results still depend on sample rate.
- Acquired a real bowed pickup reference and labeled direct electric guitars under CC BY 4.0. The low-E reference's H2–H6 dB-envelope error falls from 5.93 to 1.67 dB on its calibration second, and from 6.21/4.17 to 2.59/3.15 dB on two withheld seconds of the same performance. No physical pickup or body-response calibration is asserted.
- Independent M5 replays with equal controls produced identical outputs on every source pair. A broad four-band transfer retained just .18–.35 dB distance on the labeled pairs. The selected partial-wise mechanism retained **1.42–2.65 dB**. Across distinct G3 performances within a player the distances can also reach 2.62 dB. Sign preservation of local notches is inconsistent. These findings support source dependence and preclude claiming pickup classification.
- Original-gain stems, three randomized level-matched comparisons and two DI references are available. No human evaluation has been recorded. The attack experiment varies V/R with the same E schedule and lift, making its partial trajectory independently testable. [Listening gate](../ACOUSTIC_VALIDATION_PLAN.md#compact-human-architecture-gate) · [Measurement report](../../research/BOWED_TIMBRE_ACOUSTIC_RESEARCH.md) · [Identity study](../../research/GUITAR_TIMBRE_PRESERVATION.md)

## Proposed changes and release boundary

Replace M5's **audible** single-ray/formant blend with two nut-side rays sampled over a finite virtual aperture and a bounded, loaded electrical transfer. Preserve M5 diagnostic samples. Use causal partial measurements on original pre-swell DI and smooth order-dependent corrections on the actual guide carrier. Add an input-dynamics-dependent cubic bow-speed/force ramp, preserve M4 and M6, and keep A4's direct mix and protection. The prior assumptions that the near-bridge force proxy, small modal blend, brightness scalar and an amplitude ramp would suffice lack support. [Full proposed specification](../../architecture/ELECTROBOW_ACOUSTIC_OUTPUT_SPECIFICATION.md) · [Exact M5/M7, A3/A4, migration, V17/V22 and AI guide amendments](../ACOUSTIC_MIGRATION_PLAN.md#proposed-reconciliation-for-independent-sign-off)

Remaining questions concern pickup geometry across frets, high-band loading and aperture, fine notches at low support, performance repeatability, true formation versus a seeded entrance, control interpretation and the full-graph RT budget. The fixed .174L position is an effective one-recording estimate; .024L width, 3.2 kHz load and feature/gesture constants need independent review. A2 and host deadline evidence remain their original owners' work. The smallest owner task is the three listening comparisons. For robust identity, obtain a same-string, same-player two-pickup repeat set with labeled playing strength and freeze its pass margin before listening. If the seeded entry fails repeatedly, record the specific missing transient and seek separate approval before modifying launch or feedback.

| Item | Recommendation now |
|---|---|
| S2a contact and fractional-delay ring | Retain both qualifications. |
| S2b prescribed guide | Retain numerical qualification, including guide-owned V03–V11/V20. Keep acoustic success separate. |
| Selected observation | Submit two-ray nut-arm spatial velocity to independent math/timing review; then isolated Rust only. |
| Selected output transfer | Submit finite aperture, loaded response, detail/trim and electrical limits for independent transfer and listening review. |
| Source identity | Submit the 12-partial causal capture/correction to independent review; hold perceptual identity signoff for matched repeats and listening. |
| S4a injected controller | Proceed under existing isolation authorization. Apply amended gesture/color semantics only after review. |
| S4b live articulation | Hold until A2, S4a, real gesture/identity evidence and acoustic gate clear. |
| S5 supported wet | Hold new production processing and stage exit. Legacy and crossover parity retain existing authorization. |
| S3/A2 analyzer | Only previously approved conditioning/difference/trough measurements may proceed. Final selector and live tracking remain blocked. |
| S6 production integration | Hold S6R/H/P for separate Rust, native and packaged gates. Independent defensive host task retains its own scope. |
| S7 release | Hold through integrated numerical/RT/host gates and formal human V22. |

[Replay commands and fixture hashes](../ACOUSTIC_VALIDATION_PLAN.md#replay-and-corpus) · [Ordered migration](../ACOUSTIC_MIGRATION_PLAN.md)
