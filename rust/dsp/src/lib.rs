//! Rust DSP boundary for JUCE. Keep new DSP in safe modules; raw-pointer code
//! belongs only here. See include/electrobow_dsp.h for the C ownership contract.
#![deny(unsafe_op_in_unsafe_fn)]

pub mod filter_bank;
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
mod tests;
