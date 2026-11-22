//! Rust DSP for the ElectroBow JUCE plugin.
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

#[cfg(test)]
mod signal_path_tests;
