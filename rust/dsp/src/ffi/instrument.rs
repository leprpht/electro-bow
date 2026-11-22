//! Six-control VST3 engine. All input is consumed before aliasable output is borrowed.
use crate::instrument::{Instrument, InstrumentControls};
use std::{ptr, slice};

/// Prepare an independent, mono or stereo instrument. Returns null on invalid dimensions.
#[unsafe(no_mangle)]
pub extern "C" fn eb_instrument_create(rate: f64, capacity: i32, channels: i32) -> *mut Instrument {
    if capacity <= 0 || channels <= 0 {
        return ptr::null_mut();
    }
    Instrument::new(rate, capacity as usize, channels as usize)
        .map_or(ptr::null_mut(), |p| Box::into_raw(Box::new(p)))
}

/// # Safety
/// Transfer exclusive ownership of a live instrument handle, or pass null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_instrument_destroy(handle: *mut Instrument) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

/// # Safety
/// Supply null or a live handle with exclusive access for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_instrument_reset(handle: *mut Instrument) {
    if let Some(engine) = unsafe { handle.as_mut() } {
        engine.reset();
    }
}

/// # Safety
/// Supply null or a live handle with serialized access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_instrument_input_level(handle: *const Instrument) -> f32 {
    unsafe { handle.as_ref() }.map_or(0.0, Instrument::input_level)
}

/// # Safety
/// Serialize access to the live handle. Supply one pointer per prepared channel in each
/// array, valid for the frame count. Output channels must be disjoint. Audio may be
/// in-place but must not overlap the handle or the pointer arrays.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eb_instrument_process(
    handle: *mut Instrument,
    input: *const *const f32,
    output: *const *mut f32,
    frames: i32,
    controls: InstrumentControls,
) -> i32 {
    let Some(engine) = (unsafe { handle.as_mut() }) else {
        return 0;
    };
    if !controls.finite() || frames < 0 || frames as usize > engine.split.capacity {
        return 0;
    }
    if frames == 0 {
        return 1;
    }
    if input.is_null() || output.is_null() {
        return 0;
    }
    let count = engine.split.channels;
    let frames = frames as usize;
    let inputs = unsafe { slice::from_raw_parts(input, count) };
    let outputs = unsafe { slice::from_raw_parts(output, count) };
    if inputs.iter().any(|p| p.is_null()) || outputs.iter().any(|p| p.is_null()) {
        return 0;
    }
    for (ch, &p) in inputs.iter().enumerate() {
        engine.ingest(ch, unsafe { slice::from_raw_parts(p, frames) });
    }
    let mut channels: [&mut [f32]; 2] = [&mut [], &mut []];
    for (channel, &p) in channels.iter_mut().zip(outputs) {
        *channel = unsafe { slice::from_raw_parts_mut(p, frames) };
    }
    engine.render(&mut channels[..count], controls);
    1
}
