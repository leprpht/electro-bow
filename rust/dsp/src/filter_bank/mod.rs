//! Four amplitude-complementary, parallel IIR bands. The sum is unity, not
//! an all-pass: L200, L800-L200, L3000-L800, 1-L3000. Individual bands have
//! gentle 6 dB/octave skirts and phase shift; their unprocessed sum does not.

#![forbid(unsafe_code)]

/// Number of bands, ordered from lowest to highest frequency.
pub const NUM_BANDS: usize = 4;
/// Maximum number of channels processed by one bank.
pub const MAX_CHANNELS: usize = 2;
/// Nominal crossover frequencies in Hz, scaled together at low sample rates.
pub const CROSSOVERS_HZ: [f64; 3] = [200.0, 800.0, 3000.0];

/// Filter history, effect stages and preallocated band buffers for one audio stream.
pub struct FilterBank {
    pub(crate) texture: crate::texture::Texture,
    pub(crate) expression: crate::expression::Expression,
    pub(crate) resonance: crate::resonance::Resonance,
    pub(crate) output: crate::output::Output,
    pub(crate) character: crate::character::Character,
    pub(crate) attack: crate::attack::Attack,
    pub(crate) split: BandSplit,
}

impl FilterBank {
    /// Prepares the legacy bow-effect handle.
    pub fn new(rate: f64, capacity: usize, channels: usize) -> Option<Self> {
        let split = BandSplit::new(rate, capacity, channels)?;
        Some(Self {
            texture: crate::texture::Texture::new(rate),
            expression: crate::expression::Expression::new(rate),
            resonance: crate::resonance::Resonance::new(rate)?,
            output: crate::output::Output::new(rate),
            character: crate::character::Character::new(rate),
            attack: crate::attack::Attack::new(rate),
            split,
        })
    }

    /// Clears the legacy stages and their shared crossover.
    pub fn reset(&mut self) {
        self.texture.reset();
        self.expression.reset();
        self.resonance.reset();
        self.output.reset();
        self.attack.reset();
        self.character.reset();
        self.split.reset();
    }
}

/// Causal crossover and its planar scratch buffers, shared by both C entry points.
pub(crate) struct BandSplit {
    pub(crate) capacity: usize,
    pub(crate) channels: usize,
    pub(crate) valid_samples: usize,
    pub(crate) crossovers_hz: [f64; 3],
    coefficients: [f64; 3],
    pub(crate) states: [[f64; 3]; MAX_CHANNELS],
    pub(crate) bands: [[Vec<f32>; MAX_CHANNELS]; NUM_BANDS],
}

impl BandSplit {
    /// Prepares buffers for up to `capacity` frames at `rate` Hz. Returns `None`
    /// for invalid dimensions, an invalid rate, or a failed buffer reservation.
    pub fn new(rate: f64, capacity: usize, channels: usize) -> Option<Self> {
        if !rate.is_finite()
            || rate <= 0.0
            || capacity == 0
            || !(1..=MAX_CHANNELS).contains(&channels)
        {
            return None;
        }
        let scale = (0.45 * rate / CROSSOVERS_HZ[2]).min(1.0);
        let crossovers_hz = CROSSOVERS_HZ.map(|cutoff| cutoff * scale);
        let coefficients = crossovers_hz.map(|cutoff| {
            let g = (std::f64::consts::PI * cutoff / rate).tan();
            g / (1.0 + g)
        });
        let mut bands: [[Vec<f32>; MAX_CHANNELS]; NUM_BANDS] = Default::default();
        for band in &mut bands {
            for channel in band {
                channel.try_reserve_exact(capacity).ok()?;
                channel.resize(capacity, 0.0);
            }
        }
        Some(Self {
            capacity,
            channels,
            valid_samples: 0,
            crossovers_hz,
            coefficients,
            states: [[0.0; 3]; MAX_CHANNELS],
            bands,
        })
    }

    /// Clears all stage histories and buffered audio for the next stream.
    pub fn reset(&mut self) {
        self.states = [[0.0; 3]; MAX_CHANNELS];
        self.valid_samples = 0;
        for band in &mut self.bands {
            for channel in band {
                channel.fill(0.0);
            }
        }
    }

    /// Splits a validated input block and advances this channel’s crossover history.
    pub(crate) fn split_channel(&mut self, channel: usize, input: &[f32]) {
        let state = &mut self.states[channel];
        let [low_band, low_mid_band, high_mid_band, high_band] = &mut self.bands;
        let outputs = low_band[channel][..input.len()]
            .iter_mut()
            .zip(&mut low_mid_band[channel][..input.len()])
            .zip(&mut high_mid_band[channel][..input.len()])
            .zip(&mut high_band[channel][..input.len()]);
        for (&sample, (((b0, b1), b2), b3)) in input.iter().zip(outputs) {
            let x = if sample.is_finite() {
                f64::from(sample).clamp(-16.0, 16.0)
            } else {
                0.0
            };
            let mut low = [0.0; 3];
            for ((low, state), coefficient) in
                low.iter_mut().zip(state.iter_mut()).zip(self.coefficients)
            {
                let v = (x - *state) * coefficient;
                *low = v + *state;
                *state = *low + v;
            }
            *b0 = low[0] as f32;
            *b1 = (low[1] - low[0]) as f32;
            *b2 = (low[2] - low[1]) as f32;
            *b3 = (x - low[2]) as f32;
        }
        // Preserve the original -600 dBFS floor at chunk boundaries.
        for value in state {
            if value.abs() < 1.0e-30 {
                *value = 0.0;
            }
        }
    }

    /// Sums the four bands for one channel into the supplied frame range.
    pub(crate) fn recombine_channel(&self, channel: usize, output: &mut [f32]) {
        let [b0, b1, b2, b3] = &self.bands;
        let inputs = b0[channel]
            .iter()
            .zip(&b1[channel])
            .zip(&b2[channel])
            .zip(&b3[channel]);
        for (sample, (((b0, b1), b2), b3)) in output.iter_mut().zip(inputs) {
            *sample = (f64::from(*b0) + f64::from(*b1) + f64::from(*b2) + f64::from(*b3)) as f32;
        }
    }
}

#[cfg(test)]
mod tests;
