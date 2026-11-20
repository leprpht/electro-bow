//! Four amplitude-complementary, parallel IIR bands. The sum is unity, not
//! an all-pass: L200, L800-L200, L3000-L800, 1-L3000. Individual bands have
//! gentle 6 dB/octave skirts and phase shift; their unprocessed sum does not.

#![forbid(unsafe_code)]

pub const NUM_BANDS: usize = 4;
pub const MAX_CHANNELS: usize = 2;
pub const CROSSOVERS_HZ: [f64; 3] = [200.0, 800.0, 3000.0];

pub struct FilterBank {
    pub(crate) output: crate::output::Output,
    pub(crate) character: crate::character::Character,
    pub(crate) attack: crate::attack::Attack,
    pub(crate) capacity: usize,
    pub(crate) channels: usize,
    pub(crate) valid_samples: usize,
    pub(crate) crossovers_hz: [f64; 3],
    coefficients: [f64; 3],
    pub(crate) states: [[f64; 3]; MAX_CHANNELS],
    pub(crate) bands: [[Vec<f32>; MAX_CHANNELS]; NUM_BANDS],
}

impl FilterBank {
    // Allocation and coefficient calculation occur only during preparation.
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
            output: crate::output::Output::new(rate),
            character: crate::character::Character::new(rate),
            attack: crate::attack::Attack::new(rate),
            capacity,
            channels,
            valid_samples: 0,
            crossovers_hz,
            coefficients,
            states: [[0.0; 3]; MAX_CHANNELS],
            bands,
        })
    }

    pub fn reset(&mut self) {
        self.output.reset();
        self.attack.reset();
        self.character.reset();
        self.states = [[0.0; 3]; MAX_CHANNELS];
        self.valid_samples = 0;
        for band in &mut self.bands {
            for channel in band {
                channel.fill(0.0);
            }
        }
    }

    // FFI validates the whole block before calling this safe, slice-based DSP.
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
mod tests {
    use super::*;

    #[test]
    fn constructor_rejects_invalid_dimensions_and_rates() {
        for rate in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -48000.0] {
            assert!(FilterBank::new(rate, 64, 1).is_none());
        }
        for (capacity, channels) in [(0, 1), (64, 0), (64, 3), (64, usize::MAX)] {
            assert!(FilterBank::new(48000.0, capacity, channels).is_none());
        }
        for channels in [1, MAX_CHANNELS] {
            assert!(FilterBank::new(48000.0, 1, channels).is_some());
        }
    }

    #[test]
    fn crossovers_keep_their_ratios_when_scaled_below_nyquist() {
        for rate in [1000.0, 4000.0, 8000.0, 48000.0, 192000.0] {
            let bank = FilterBank::new(rate, 8, 1).unwrap();
            let expected_high = 3000.0_f64.min(rate * 0.45);
            for (actual, nominal) in bank.crossovers_hz.iter().zip(CROSSOVERS_HZ) {
                assert!((actual - nominal / 3000.0 * expected_high).abs() < 1e-10);
                assert!(*actual < rate * 0.5);
            }
            assert!(bank.crossovers_hz.windows(2).all(|pair| pair[0] < pair[1]));
        }
    }

    #[test]
    fn invalid_samples_match_explicitly_sanitized_input_and_history() {
        let input = [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MAX,
            -f32::MAX,
            0.5,
        ];
        let sanitized = [0.0, 0.0, 0.0, 16.0, -16.0, 0.5];
        let mut actual = FilterBank::new(48000.0, input.len(), 1).unwrap();
        let mut expected = FilterBank::new(48000.0, input.len(), 1).unwrap();
        actual.split_channel(0, &input);
        expected.split_channel(0, &sanitized);
        assert_eq!(actual.bands, expected.bands);
        assert_eq!(actual.states, expected.states);
        let mut reconstructed = [0.0; 6];
        actual.recombine_channel(0, &mut reconstructed);
        for (actual, expected) in reconstructed.into_iter().zip(sanitized) {
            assert!((actual - expected).abs() < 2e-6);
        }
        actual.split_channel(0, &[0.25; 6]);
        expected.split_channel(0, &[0.25; 6]);
        assert_eq!(
            actual.bands, expected.bands,
            "faults must not poison history"
        );
    }

    #[test]
    fn short_blocks_leave_unused_storage_and_other_channel_untouched() {
        let mut bank = FilterBank::new(48000.0, 8, 2).unwrap();
        for band in &mut bank.bands {
            for channel in band {
                channel.fill(123.0);
            }
        }
        bank.split_channel(0, &[1.0, -0.5]);
        for band in &bank.bands {
            assert_eq!(band[0][2..], [123.0; 6]);
            assert_eq!(band[1], [123.0; 8]);
        }
        assert_eq!(bank.states[1], [0.0; 3]);
        let mut output = [42.0; 8];
        bank.recombine_channel(0, &mut output[..2]);
        assert!((output[0] - 1.0).abs() < 1e-7);
        assert!((output[1] + 0.5).abs() < 1e-7);
        assert_eq!(output[2..], [42.0; 6]);
    }

    #[test]
    fn reset_clears_storage_and_history_without_changing_preparation() {
        let mut bank = FilterBank::new(4000.0, 32, 2).unwrap();
        let crossovers = bank.crossovers_hz;
        for channel in 0..2 {
            bank.split_channel(channel, &[1.0; 32]);
        }
        bank.valid_samples = 32;
        bank.reset();
        assert_eq!(bank.valid_samples, 0);
        assert_eq!(bank.states, [[0.0; 3]; 2]);
        assert!(bank.bands.iter().flatten().flatten().all(|&x| x == 0.0));
        assert_eq!(bank.crossovers_hz, crossovers);
        assert_eq!((bank.capacity, bank.channels), (32, 2));
        let mut fresh = FilterBank::new(4000.0, 32, 2).unwrap();
        bank.split_channel(0, &[0.25; 32]);
        fresh.split_channel(0, &[0.25; 32]);
        assert_eq!(bank.bands, fresh.bands);
        assert_eq!(bank.states, fresh.states);
    }
}
