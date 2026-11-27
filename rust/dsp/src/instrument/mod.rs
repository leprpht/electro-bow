//! Six-control production instrument. Per-stream storage is prepared off the audio thread.
#![forbid(unsafe_code)]

mod body;
// Prepared physical model is private until controller integration.
#[cfg_attr(not(test), allow(dead_code))]
mod friction;
#[cfg_attr(not(test), allow(dead_code))]
mod ring;
#[cfg_attr(not(test), allow(dead_code))]
mod waveguide;

use crate::{attack::Attack, filter_bank::BandSplit, output::Output};
use body::Body;

/// Physical values of the six faceplate controls. Attack is in ms, the rest in 0..1.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct InstrumentControls {
    pub attack_ms: f64,
    pub character: f64,
    pub noise: f64,
    pub saturation: f64,
    pub spectral_movement: f64,
    pub out: f64,
}

impl InstrumentControls {
    pub(crate) fn finite(self) -> bool {
        [
            self.attack_ms,
            self.character,
            self.noise,
            self.saturation,
            self.spectral_movement,
            self.out,
        ]
        .into_iter()
        .all(f64::is_finite)
    }
}

/// Opaque state of the production C API.
pub struct Instrument {
    pub(crate) split: BandSplit,
    attack: Attack,
    body: Body,
    output: Output,
    depths: Option<[f64; 4]>,
    smoothing: f64,
    meter_release: f64,
    levels: [f64; 2],
}

impl Instrument {
    pub(crate) fn new(rate: f64, frames: usize, channels: usize) -> Option<Self> {
        Some(Self {
            split: BandSplit::new(rate, frames, channels)?,
            attack: Attack::new(rate),
            body: Body::new(rate),
            output: Output::new(rate),
            depths: None,
            smoothing: 1.0 - (-1.0 / (0.015 * rate)).exp(),
            meter_release: (-1.0 / (0.035 * rate)).exp(),
            levels: [0.; 2],
        })
    }

    pub(crate) fn reset(&mut self) {
        self.split.reset();
        self.attack.reset();
        self.body.reset();
        self.output.reset();
        self.depths = None;
        self.levels = [0.; 2];
    }

    pub(crate) fn input_level(&self) -> f32 {
        self.levels[0].max(self.levels[1]) as f32
    }

    pub(crate) fn ingest(&mut self, ch: usize, input: &[f32]) {
        let mut level = self.levels[ch];
        self.split.split_channel_observing(ch, input, |sample| {
            level = sample.abs().max(level * self.meter_release);
        });
        self.levels[ch] = level;
    }

    pub(crate) fn render(&mut self, channels: &mut [&mut [f32]], controls: InstrumentControls) {
        let samples = channels[0].len();
        let bands = &mut self.split.bands;
        self.attack
            .process(bands, channels.len(), samples, controls.attack_ms);
        let targets = [
            controls.character,
            controls.noise,
            controls.saturation,
            controls.spectral_movement,
        ]
        .map(|v| v.clamp(0., 1.));
        let depths = self.depths.get_or_insert(targets);
        for i in 0..samples {
            for (value, target) in depths.iter_mut().zip(targets) {
                *value += self.smoothing * (target - *value);
            }
            let [character, noise, saturation, movement] = *depths;
            let (scatter, blend) = self.body.modulation(noise, movement);
            let mix = 0.66 * character * (2.0 - character);
            // The two pickups discard off-resonance energy. A fixed mix curve
            // limits the level drop without following chord peaks.
            let pickup_trim = 1.0 + 0.90 * mix;
            for (ch, output) in channels.iter_mut().enumerate() {
                let input = std::array::from_fn(|band| f64::from(bands[band][ch][i]));
                let direct = input.iter().sum::<f64>();
                let body = self.body.pickup(ch, input, scatter, blend, saturation);
                output[i] = ((direct + mix * (body - direct)) * pickup_trim) as f32;
            }
        }
        self.output.process(channels, controls.out);
    }
}

#[cfg(test)]
mod tests;
