//! C entry points shared with the JUCE adapter via `include/electrobow_dsp.h`.
//! Each handle owns one stream. The caller serializes access to its state and buffers.

use crate::filter_bank::{self, FilterBank};
use crate::{BowControls, PanelControls};
use std::{ptr, slice};

/// Prepares a mono or stereo bank for blocks of up to `capacity` frames at `rate` Hz.
/// Returns null if preparation fails.
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

/// Releases the handle and its band buffers.
///
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

/// Clears stream history and the valid frame count while retaining allocated buffers.
///
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

/// Splits each input channel into four bands. Returns 1 on success and 0 on failure.
///
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

/// Sums the current bands into each output channel. An empty split succeeds.
///
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

/// Borrows the start of one band buffer, or returns null for invalid indices or a null bank.
///
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

/// Returns the valid frame count from the most recent split, or zero for a null bank.
///
/// # Safety
/// Same live-handle and serialized-access contract as reset.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_filter_bank_samples(bank: *const FilterBank) -> i32 {
    // SAFETY: caller supplies a live handle or null.
    unsafe { bank.as_ref() }.map_or(0, |bank| bank.valid_samples as i32)
}

/// Writes the three effective crossover frequencies in ascending order, in Hz.
///
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

#[cfg(test)]
mod tests;
