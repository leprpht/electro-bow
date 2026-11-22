//! Rust DSP boundary for JUCE. Keep new DSP in safe modules; raw-pointer code
//! belongs only here. See include/electrobow_dsp.h for the C ownership contract.
#![deny(unsafe_op_in_unsafe_fn)]

mod attack;
mod character;
mod expression;
pub mod filter_bank;
mod output;
mod resonance;
mod texture;
use filter_bank::FilterBank;
use std::{ptr, slice};

#[unsafe(no_mangle)]
pub extern "C" fn eb_filter_bank_create(
    rate: f64,
    capacity: i32,
    channels: i32,
) -> *mut FilterBank {
    if capacity <= 0 || channels <= 0 {
        return ptr::null_mut();
    }
    FilterBank::new(rate, capacity as usize, channels as usize)
        .map_or(ptr::null_mut(), |bank| Box::into_raw(Box::new(bank)))
}

/// # Safety
/// `bank` must be null or a live handle from create, owned exclusively by this
/// call. Destroy exactly once; no borrowed band pointers may be used afterward.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_destroy(bank: *mut FilterBank) {
    if !bank.is_null() {
        // SAFETY: the caller transfers ownership of the Box allocated by create.
        drop(unsafe { Box::from_raw(bank) });
    }
}

/// # Safety
/// Non-null handles must be live and exclusively accessed for the duration of
/// every API call. External band pointers must not be accessed during a call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_reset(bank: *mut FilterBank) {
    // SAFETY: guaranteed by the handle contract; null is a no-op.
    if let Some(bank) = unsafe { bank.as_mut() } {
        bank.reset();
    }
}

/// # Safety
/// Same handle contract as reset. For positive `samples`, `input` must point to
/// `channels` readable pointers, each to `samples` readable floats. Input must
/// not overlap bank storage. No input memory is retained after the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_split(
    bank: *mut FilterBank,
    input: *const *const f32,
    samples: i32,
) -> i32 {
    // SAFETY: handle ownership is the caller's responsibility.
    let Some(bank) = (unsafe { bank.as_mut() }) else {
        return 0;
    };
    bank.valid_samples = 0;
    if samples < 0 || samples as usize > bank.capacity {
        return 0;
    }
    if samples == 0 {
        return 1;
    }
    if input.is_null() {
        return 0;
    }
    // SAFETY: pointer-array length is guaranteed by the caller's contract.
    let inputs = unsafe { slice::from_raw_parts(input, bank.channels) };
    if inputs.iter().any(|input| input.is_null()) {
        return 0;
    }
    for (channel, &input) in inputs.iter().enumerate() {
        // SAFETY: length, lifetime and disjointness from bank guaranteed by caller.
        let input = unsafe { slice::from_raw_parts(input, samples as usize) };
        bank.split_channel(channel, input);
    }
    bank.valid_samples = samples as usize;
    1
}

/// # Safety
/// Same handle contract as reset. `output` must contain `channels` writable
/// pointers with room for the most recent split's valid samples. Outputs must
/// not overlap each other or bank storage. They may reuse the previous input.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_recombine(
    bank: *const FilterBank,
    output: *const *mut f32,
) -> i32 {
    // SAFETY: caller supplies a live handle or null.
    let Some(bank) = (unsafe { bank.as_ref() }) else {
        return 0;
    };
    if bank.valid_samples == 0 {
        return 1;
    }
    if output.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees pointer-array length.
    let outputs = unsafe { slice::from_raw_parts(output, bank.channels) };
    if outputs.iter().any(|output| output.is_null()) {
        return 0;
    }
    for (channel, &output) in outputs.iter().enumerate() {
        // SAFETY: caller guarantees writable length and no overlap with bank.
        let output = unsafe { slice::from_raw_parts_mut(output, bank.valid_samples) };
        bank.recombine_channel(channel, output);
    }
    1
}

/// # Safety
/// Same handle contract as reset. The returned pointer is owned by Rust and
/// remains allocated until destroy. Reacquire after split/reset; access only
/// between API calls, and never free or resize it. Only the most recent split's sample count contains valid audio.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_band(
    bank: *mut FilterBank,
    band: i32,
    channel: i32,
) -> *mut f32 {
    // SAFETY: caller supplies a live exclusive handle or null.
    let Some(bank) = (unsafe { bank.as_mut() }) else {
        return ptr::null_mut();
    };
    if band < 0
        || band as usize >= filter_bank::NUM_BANDS
        || channel < 0
        || channel as usize >= bank.channels
    {
        return ptr::null_mut();
    }
    bank.bands[band as usize][channel as usize].as_mut_ptr()
}

/// # Safety
/// Same live-handle and serialized-access contract as reset.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_samples(bank: *const FilterBank) -> i32 {
    // SAFETY: caller supplies a live handle or null.
    unsafe { bank.as_ref() }.map_or(0, |bank| bank.valid_samples as i32)
}

/// # Safety
/// Same handle contract as reset. `output` must point to three writable doubles,
/// disjoint from bank storage. No pointer is retained after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_crossovers(
    bank: *const FilterBank,
    output: *mut f64,
) -> i32 {
    // SAFETY: caller supplies a live handle or null.
    let Some(bank) = (unsafe { bank.as_ref() }) else {
        return 0;
    };
    if output.is_null() {
        return 0;
    }
    // SAFETY: output has room for three doubles and does not overlap bank.
    unsafe { ptr::copy_nonoverlapping(bank.crossovers_hz.as_ptr(), output, 3) };
    1
}

#[cfg(test)]
mod final_tests;
#[cfg(test)]
mod tests;

/// Linked pre-effect peak envelope for metering.
///
/// # Safety
/// Same serialized handle contract as eb_filter_bank_samples.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_bow_input_level(bank: *const FilterBank) -> f32 {
    // SAFETY: caller supplies a live handle or null, with exclusive access.
    unsafe { bank.as_ref() }.map_or(0.0, |bank| bank.expression.input_level())
}

/// Apply spectral attack control to the last split, before recombination.
/// # Safety
/// Same exclusive live-handle contract as reset; borrowed band pointers must
/// not be accessed during this call. Finite milliseconds are clamped to 0..500.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_attack(bank: *mut FilterBank, milliseconds: f64) -> i32 {
    let Some(bank) = (unsafe { bank.as_mut() }) else {
        return 0;
    };
    if !milliseconds.is_finite() {
        return 0;
    }
    bank.attack.process(
        &mut bank.bands,
        bank.channels,
        bank.valid_samples,
        milliseconds,
    );
    1
}

/// Apply Bow Character to current bands after Attack and before recombination.
/// # Safety
/// Same exclusive live-handle and borrowed-buffer contract as reset.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_character(bank: *mut FilterBank, amount: f64) -> i32 {
    // SAFETY: caller supplies an exclusive live handle or null.
    let Some(bank) = (unsafe { bank.as_mut() }) else {
        return 0;
    };
    if !amount.is_finite() {
        return 0;
    }
    bank.character
        .process(&mut bank.bands, bank.channels, bank.valid_samples, amount);
    1
}

/// Physical control values at the C boundary. Unitless controls range from
/// zero to one. Bow speed has a neutral midpoint.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BowControls {
    pub attack_ms: f64,
    pub character: f64,
    pub sustain: f64,
    pub resonance_ms: f64,
    pub pressure: f64,
    pub speed: f64,
    pub friction: f64,
}

impl BowControls {
    pub const NEUTRAL: Self = Self {
        attack_ms: 0.0,
        character: 0.0,
        sustain: 0.0,
        resonance_ms: 0.0,
        pressure: 0.0,
        speed: 0.5,
        friction: 0.0,
    };
}

/// Full signal path with linked output peak protection. Zero samples are a no-op.
/// # Safety
/// Same live-handle, buffer lengths and non-overlap contracts as split/recombine.
/// Input and output may alias each other, but neither may alias bank storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_process(
    bank: *mut FilterBank,
    input: *const *const f32,
    output: *const *mut f32,
    samples: i32,
    attack: f64,
    character: f64,
) -> i32 {
    // SAFETY: identical buffer and ownership contract.
    unsafe {
        eb_bow_process(
            bank,
            input,
            output,
            samples,
            BowControls {
                attack_ms: attack,
                character,
                ..BowControls::NEUTRAL
            },
        )
    }
}

/// Complete seven-control signal path.
/// # Safety
/// Identical ownership, buffer and aliasing requirements to eb_filter_bank_process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_bow_process(
    bank: *mut FilterBank,
    input: *const *const f32,
    output: *const *mut f32,
    samples: i32,
    controls: BowControls,
) -> i32 {
    // SAFETY: same exclusive bank and caller buffer contract.
    unsafe {
        eb_bow_process_panel(
            bank,
            input,
            output,
            samples,
            controls,
            PanelControls::NEUTRAL,
        )
    }
}

/// Normalized faceplate parameters. Out is a linear amplitude trim with unity at 0.5.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PanelControls {
    pub noise: f64,
    pub saturation: f64,
    pub spectral_movement: f64,
    pub out: f64,
}

impl PanelControls {
    pub const NEUTRAL: Self = Self {
        noise: 0.0,
        saturation: 0.0,
        spectral_movement: 0.0,
        out: 0.5,
    };
}

/// Full signal path, including the three character details and output trim.
/// # Safety
/// Identical ownership, buffer and aliasing requirements to eb_filter_bank_process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_bow_process_panel(
    bank: *mut FilterBank,
    input: *const *const f32,
    output: *const *mut f32,
    samples: i32,
    controls: BowControls,
    panel: PanelControls,
) -> i32 {
    let finite = [
        controls.attack_ms,
        controls.character,
        controls.sustain,
        controls.resonance_ms,
        controls.pressure,
        controls.speed,
        controls.friction,
        panel.noise,
        panel.saturation,
        panel.spectral_movement,
        panel.out,
    ]
    .into_iter()
    .all(f64::is_finite);
    if bank.is_null() || !finite || samples < 0 {
        return 0;
    }
    if samples == 0 {
        return 1;
    }
    // SAFETY: handle is exclusively owned by caller; inspect metadata only.
    let count = unsafe { (*bank).channels };
    if output.is_null() {
        return 0;
    }
    // SAFETY: caller provides configured number of output pointers.
    let pointers = unsafe { slice::from_raw_parts(output, count) };
    if pointers.iter().any(|p| p.is_null()) {
        return 0;
    }
    // SAFETY: forward the same exclusive handle and validated caller buffers.
    unsafe {
        if eb_filter_bank_split(bank, input, samples) == 0 {
            return 0;
        }
        let processor = &mut *bank;
        processor
            .expression
            .process(&mut processor.bands, count, samples as usize, &controls);
        eb_filter_bank_attack(bank, controls.attack_ms);
        eb_filter_bank_character(bank, controls.character);
        let processor = &mut *bank;
        processor.texture.process(
            &mut processor.bands,
            count,
            samples as usize,
            controls.character,
            panel,
        );
        eb_filter_bank_recombine(bank, output);
    }
    let mut channels: [&mut [f32]; 2] = [&mut [], &mut []];
    for (channel, &pointer) in channels.iter_mut().zip(pointers) {
        // SAFETY: output channels are disjoint and writable for samples frames.
        *channel = unsafe { slice::from_raw_parts_mut(pointer, samples as usize) };
    }
    // SAFETY: output storage cannot alias the exclusive bank handle.
    unsafe {
        (*bank)
            .resonance
            .process(&mut channels[..count], controls.resonance_ms);
        (*bank).output.process(&mut channels[..count], panel.out);
    }
    1
}
