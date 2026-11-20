//! Continuous, level-relative spectral transient control. No event detector,
//! pitch tracker, lookahead, gate, or onset-triggered amplitude ramp.
#![forbid(unsafe_code)]

pub struct Attack {
    rate: f64,
    last_target: f64,
    slow_coefficient: f64,
    fast_release: f64,
    control: f64,
    amount: f64,
    fast: [f64; 4],
    slow: [f64; 4],
    gain: [f64; 4],
}
impl Attack {
    pub fn new(rate: f64) -> Self {
        Self {
            rate,
            last_target: 0.0,
            slow_coefficient: 1.0 - (-1.0 / (0.005 * rate)).exp(),
            fast_release: (-1.0 / (0.003 * rate)).exp(),
            control: 1.0 - (-1.0 / (0.001 * rate)).exp(),
            amount: 0.0,
            fast: [0.0; 4],
            slow: [0.0; 4],
            gain: [1.0; 4],
        }
    }
    pub fn reset(&mut self) {
        self.fast = [0.0; 4];
        self.slow = [0.0; 4];
        self.gain = [1.0; 4];
        self.amount = 0.0;
    }
    pub fn process(
        &mut self,
        bands: &mut [[Vec<f32>; 2]; 4],
        channels: usize,
        samples: usize,
        milliseconds: f64,
    ) {
        let target = (milliseconds / 500.0).clamp(0.0, 1.0);
        // Coefficients computed once per chunk, never per sample.
        if target != self.last_target {
            self.slow_coefficient = 1.0 - (-1.0 / ((0.005 + 0.145 * target) * self.rate)).exp();
            self.last_target = target;
        }
        let slow_coefficient = self.slow_coefficient;
        let weights = [0.0, 0.15, 0.65, 1.0];
        for i in 0..samples {
            self.amount += self.control * (target - self.amount);
            for (b, band) in bands.iter_mut().enumerate() {
                let peak = band
                    .iter()
                    .take(channels)
                    .map(|c| f64::from(c[i]).abs())
                    .fold(0.0, f64::max);
                self.fast[b] = peak.max(self.fast[b] * self.fast_release);
                self.slow[b] += slow_coefficient * (self.fast[b] - self.slow[b]);
                let novelty = if self.fast[b] > 1e-30 {
                    (1.0 - self.slow[b] / self.fast[b]).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let desired = 1.0 - 0.75 * weights[b] * self.amount * novelty;
                self.gain[b] += self.control * (desired - self.gain[b]);
                for channel in band.iter_mut().take(channels) {
                    channel[i] *= if target == 0.0 && self.amount < 1e-12 {
                        1.0
                    } else {
                        self.gain[b] as f32
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Attack;

    fn fixture() -> [[Vec<f32>; 2]; 4] {
        std::array::from_fn(|_| {
            std::array::from_fn(|channel| {
                (0..2048)
                    .map(|i| {
                        let sample = if i % 257 < 32 { 0.8 } else { 0.1 };
                        if channel == 0 { sample } else { -sample * 0.5 }
                    })
                    .collect()
            })
        })
    }

    #[test]
    fn zero_amount_is_exact_bypass() {
        let original = fixture();
        let mut actual = original.clone();
        Attack::new(48000.0).process(&mut actual, 2, 2048, 0.0);
        assert_eq!(actual, original);
    }

    #[test]
    fn finite_controls_are_clamped_to_supported_range() {
        for (requested, clamped) in [(-10.0, 0.0), (500.0 * 2.0, 500.0)] {
            let mut actual = fixture();
            let mut expected = actual.clone();
            Attack::new(48000.0).process(&mut actual, 2, 2048, requested);
            Attack::new(48000.0).process(&mut expected, 2, 2048, clamped);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn reset_after_control_changes_matches_fresh_processor() {
        let mut processor = Attack::new(48000.0);
        // Exercise coefficient/control history at several settings before reset.
        for amount in [500.0, 500.0 * 0.25, 500.0 * 0.75] {
            processor.process(&mut fixture(), 2, 2048, amount);
        }
        processor.reset();
        let mut actual = fixture();
        let mut expected = actual.clone();
        processor.process(&mut actual, 2, 2048, 500.0 * 0.75);
        Attack::new(48000.0).process(&mut expected, 2, 2048, 500.0 * 0.75);
        assert_eq!(actual, expected);
    }

    #[test]
    fn processing_respects_sample_count_and_configured_channels() {
        let original = fixture();
        let mut actual = original.clone();
        Attack::new(48000.0).process(&mut actual, 1, 1024, 500.0);
        for (processed, source) in actual.iter().zip(&original) {
            assert_eq!(processed[1], source[1], "inactive channel");
            assert_eq!(processed[0][1024..], source[0][1024..], "unused tail");
        }
        assert_eq!(actual[0], original[0], "lowest band must remain unchanged");
        assert_ne!(actual[3][0][..1024], original[3][0][..1024]);
    }

    #[test]
    fn stereo_link_preserves_opposite_polarity_channel_ratio() {
        let mut actual = fixture();
        Attack::new(48000.0).process(&mut actual, 2, 2048, 500.0);
        for band in actual {
            for (left, right) in band[0].iter().zip(&band[1]) {
                assert!((left * -0.5 - right).abs() < 1e-7);
            }
        }
    }

    #[test]
    fn silence_stays_silent_after_signal_history() {
        let mut processor = Attack::new(48000.0);
        processor.process(&mut fixture(), 2, 2048, 500.0);
        let mut silence = std::array::from_fn(|_| std::array::from_fn(|_| vec![0.0; 2048]));
        processor.process(&mut silence, 2, 2048, 500.0);
        assert!(silence.iter().flatten().flatten().all(|&x| x == 0.0));
    }
}
