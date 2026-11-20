//! Stereo-linked zero-lookahead sample-peak protection. No gain change below
//! the ceiling until an overload occurs; recovery is shared across channels.
#![forbid(unsafe_code)]
pub struct Output {
    gain: f64,
    release: f64,
}

impl Output {
    pub fn new(rate: f64) -> Self {
        Self {
            gain: 1.0,
            release: (-1.0 / (0.050 * rate)).exp(),
        }
    }
    pub fn reset(&mut self) {
        self.gain = 1.0;
    }
    pub fn process(&mut self, channels: &mut [&mut [f32]]) {
        for i in 0..channels[0].len() {
            let peak = channels
                .iter()
                .map(|c| {
                    if c[i].is_finite() {
                        f64::from(c[i]).abs()
                    } else {
                        0.0
                    }
                })
                .fold(0.0, f64::max);
            let required = if peak > 0.98 { 0.98 / peak } else { 1.0 };
            self.gain = required.min(1.0 - (1.0 - self.gain) * self.release);
            for channel in channels.iter_mut() {
                channel[i] = if channel[i].is_finite() {
                    (f64::from(channel[i]) * self.gain) as f32
                } else {
                    0.0
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Output;

    #[test]
    fn signal_at_or_below_ceiling_is_unchanged() {
        let original = [0.0, 0.2, -0.5, 0.98, -0.98];
        let mut samples = original;
        Output::new(48000.0).process(&mut [&mut samples]);
        assert_eq!(samples, original);
    }

    #[test]
    fn overload_in_either_channel_controls_both_channels() {
        for overload_on_left in [true, false] {
            let mut loud = [2.0, -4.0];
            let mut quiet = [0.5, -1.0];
            let mut output = Output::new(48000.0);
            if overload_on_left {
                output.process(&mut [&mut loud, &mut quiet]);
            } else {
                output.process(&mut [&mut quiet, &mut loud]);
            }
            for (actual, expected) in loud.into_iter().zip([0.98, -0.98]) {
                assert!((actual - expected).abs() < 1e-7);
            }
            for (actual, expected) in quiet.into_iter().zip([0.245, -0.245]) {
                assert!((actual - expected).abs() < 1e-7);
            }
        }
    }

    #[test]
    fn non_finite_samples_are_silenced_without_muting_the_other_channel() {
        let mut invalid = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
        let mut finite = [0.25, -0.5, 0.75];
        Output::new(48000.0).process(&mut [&mut invalid, &mut finite]);
        assert_eq!(invalid, [0.0; 3]);
        assert_eq!(finite, [0.25, -0.5, 0.75]);
    }

    #[test]
    fn release_recovers_over_time_and_reset_restores_unity() {
        for rate in [8000.0, 48000.0, 192000.0] {
            let mut output = Output::new(rate);
            output.process(&mut [&mut [4.0]]);
            let mut recovery = vec![0.5; (rate * 0.25) as usize];
            output.process(&mut [&mut recovery]);
            assert!(recovery[0] < 0.13, "gain must not recover instantly");
            assert!(recovery.windows(2).all(|pair| pair[0] <= pair[1]));
            // Five release time constants should recover over 99% of unity.
            assert!((recovery.last().unwrap() - 0.5).abs() < 0.003);
            output.process(&mut [&mut [4.0]]);
            output.reset();
            let mut quiet = [0.5];
            output.process(&mut [&mut quiet]);
            assert_eq!(quiet, [0.5]);
        }
    }

    #[test]
    fn overload_and_recovery_are_independent_of_block_size() {
        let source: Vec<_> = (0..4096)
            .map(|i| if i % 503 == 0 { -4.0 } else { 0.25 })
            .collect();
        let mut whole = source.clone();
        Output::new(48000.0).process(&mut [&mut whole]);
        for size in [1, 17, 256] {
            let mut partitioned = source.clone();
            let mut output = Output::new(48000.0);
            for block in partitioned.chunks_mut(size) {
                output.process(&mut [block]);
            }
            assert_eq!(partitioned, whole);
        }
    }

    #[test]
    fn empty_block_preserves_gain_history() {
        let mut output = Output::new(48000.0);
        let mut reference = Output::new(48000.0);
        output.process(&mut [&mut [2.0]]);
        reference.process(&mut [&mut [2.0]]);
        output.process(&mut [&mut []]);
        let mut actual = [0.5];
        let mut expected = actual;
        output.process(&mut [&mut actual]);
        reference.process(&mut [&mut expected]);
        assert_eq!(actual, expected);
    }
}
