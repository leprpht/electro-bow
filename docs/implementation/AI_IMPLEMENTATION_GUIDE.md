# Start here to implement bowed ElectroBow

Build a real-time guitar audio effect. The input supplies a continuous pitch trajectory and evidence of a stroke. The virtual bow keeps doing work on a modeled string as the pick decays. The direct guitar, its amplitude and spectral balance retain aspects of the player and pickup. One voice per audio channel is the release scope. All analysis, gesture state, fractional delays, contact, mixing and memory ownership belong in Rust.

## Which document decides what

| Implementation question | Authoritative reference |
|---|---|
| Which handle actually runs in the VST3? Which code and tests serve other callers? | [Existing DSP audit](../research/EXISTING_DSP_AUDIT.md), especially the callback trace |
| What came from STK, Faust, Woodhouse, YIN and DDSP? Which reference code can be reused? | [Reference research and licenses](../research/BOWED_STRING_REFERENCE_RESEARCH.md) |
| What gets read/written on every sample? Which friction root, phase correction and startup state are required? | [Bowed-string mathematics](../architecture/BOWED_STRING_MATHEMATICS.md), M2–M7 |
| How are Hz, confidence, dynamics, stereo, note transitions and old controls handled? | [Target architecture](../architecture/ELECTROBOW_TARGET_ARCHITECTURE.md), A1–A4 |
| What is the next reviewable change and what must it preserve? | [Migration plan](ELECTROBOW_MIGRATION_PLAN.md), steps 1–7 in dependency order |
| What numerical tolerances, CPU budgets and listening evidence qualify a change? | [DSP validation plan](DSP_VALIDATION_PLAN.md), V01–V22 |

Earlier architecture notes describe prior instruments. The audit pins the base revision and distinguishes historical behavior from the new specification. Verify the actual call graph again if implementing on a later revision.

## Rules for implementation

1. Start with the production baseline. Add the contact, guide and tracker in isolation. Connect them to JUCE only after the controller and output branch work through the Rust harness. Keep each increment buildable.
2. Follow M2's **round-trip** delays and read-before-write convention, M3's **actual-slip** solve, M4's **finite initial history** and M6's force/energy checks. The stick/slip state is per voice. Seed only at a confirmed launch under zero wet gain. Listen and measure the uncolored bridge output before mixing it with direct guitar.
3. Observe pre-swell input and publish candidates on the decimated sample clock. Continuous Hz, confidence loss, unsupported chords and requested rearticulation each have a defined path in A2–A3. Apply the finite hold and release even when a host delivers silence in tiny chunks.
4. Keep the six host IDs, EBOW state readers/writer, bypass ramp, neutral reconstruction, mono/stereo layouts, linked protection and legacy filter-bank ABI. Preserve input-before-output borrowing. No new UI or MIDI conversion. Do not run aubio/STK, neural inference or DSP callbacks in C++. No block-relative clocks, unbounded root finder, runtime allocations or locks.
5. Random texture belongs only to the bounded output coloration. A sustained sine, repeating generated saw, looped input, extra resonator feedback, automatic makeup or final limiter used to impersonate stable bow motion would invalidate the energy and release tests.
6. Run all existing Rust tests and every clause executable at the current observation boundary in the migration validation matrix. Record downstream clauses as pending until their modules exist. Consult the current per-stage authorization before implementing a selector or changing production routing. When production routing changes, also run the processor and packaged VST3 tests, including preset recall, block partitions and active-voice allocation checks. Identify pre-limiter and post-limiter signals in reports. Separate measurements of pitch acquisition time from host-reported latency.

An agent can accept an equation or a regression test with numerical evidence. Accepting the *sound* requires the three brief V22 listening comparisons. Record failures against the named fixture before tuning a threshold or changing the physical model.

Current decisions and replay commands are in [specification sign-off](status/SPECIFICATION_SIGNOFF.md). The [validation ownership matrix](ELECTROBOW_MIGRATION_PLAN.md#validation-ownership) governs dependencies.
