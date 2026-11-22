//! Rust DSP for the ElectroBow JUCE plugin.
//! Use [`eb_bow_process_panel`] for a complete block, or [`eb_filter_bank_split`]
//! and [`eb_filter_bank_recombine`] for direct access to the four bands.
#![deny(unsafe_op_in_unsafe_fn)]

mod attack;
mod character;
mod expression;
mod ffi;
pub mod filter_bank;
mod output;
mod resonance;
mod texture;

pub use ffi::*;

/// Physical control values at the C boundary. Unitless controls range from
/// zero to one. Bow speed has a neutral midpoint.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BowControls {
    /// Onset swell time in milliseconds (0 to 500).
    pub attack_ms: f64,
    /// Amount of bowed coloration and overall depth of the faceplate details.
    pub character: f64,
    /// Makeup gain depth for a decaying note.
    pub sustain: f64,
    /// Body decay time in milliseconds (0 to 1000).
    pub resonance_ms: f64,
    /// Compression and low-mid emphasis.
    pub pressure: f64,
    /// Spectral tilt, with neutral balance at 0.5.
    pub speed: f64,
    /// Strength of the low-band contact waveshaping.
    pub friction: f64,
}

impl BowControls {
    /// Bow settings for an uncolored signal before output protection.
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

/// Normalized faceplate parameters. Out is a linear amplitude trim with unity at 0.5.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PanelControls {
    /// Depth of string-driven contact noise.
    pub noise: f64,
    /// Amount of soft saturation in the low-mid band.
    pub saturation: f64,
    /// Depth of the slow spectral tilt.
    pub spectral_movement: f64,
    /// Linear output trim from silence at 0 to double amplitude at 1.
    pub out: f64,
}

impl PanelControls {
    /// Disables the three character details and sets the output trim to unity.
    pub const NEUTRAL: Self = Self {
        noise: 0.0,
        saturation: 0.0,
        spectral_movement: 0.0,
        out: 0.5,
    };
}

#[cfg(test)]
mod signal_path_tests;
