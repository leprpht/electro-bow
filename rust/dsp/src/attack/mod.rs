//! A linked, level-relative swell on the original audio. Tracking a slower
//! envelope underneath a fast one softens re-picks without a trigger or gate.
#![forbid(unsafe_code)]

pub struct Attack {
    rate: f64,
    rise: f64,
    release: f64,
    smoothing: f64,
    coefficient: f64,
    envelopes: [Swell; 2],
    initialized: bool,
}

#[derive(Default)]
struct Swell {
    held: f64,
    fast: f64,
    bowed: f64,
}

impl Swell {
    fn gain(&mut self, peak: f64, rise: f64, release: f64, coefficient: f64) -> f32 {
        self.held = peak.max(self.held * release);
        self.fast += rise * (self.held - self.fast);
        // Discharge between strokes while retaining the envelope across
        // individual low-string cycles.
        let follow = if self.fast < self.bowed {
            coefficient.max(1.0 - release)
        } else {
            coefficient
        };
        self.bowed += follow * (self.fast - self.bowed);
        if self.fast > 1e-30 {
            (1.10 * self.bowed / self.fast).min(1.0) as f32
        } else {
            1.0
        }
    }
}

impl Attack {
    pub fn new(rate: f64) -> Self {
        Self {
            rate,
            rise: 1.0 - (-1.0 / (0.002 * rate)).exp(),
            release: (-1.0 / (0.025 * rate)).exp(),
            smoothing: 1.0 - (-1.0 / (0.008 * rate)).exp(),
            coefficient: 1.0,
            envelopes: Default::default(),
            initialized: false,
        }
    }

    pub fn reset(&mut self) {
        self.envelopes = Default::default();
        self.initialized = false;
    }

    pub fn process(
        &mut self,
        bands: &mut [[Vec<f32>; 2]; 4],
        channels: usize,
        samples: usize,
        milliseconds: f64,
    ) {
        if samples == 0 {
            return;
        }
        // Time to reach 90% of a steady amplitude.
        let time = milliseconds.clamp(0.0, 500.0) * 0.001;
        let target = if time == 0.0 {
            1.0
        } else {
            1.0 - (-std::f64::consts::LN_10 / (time * self.rate)).exp()
        };
        if !self.initialized {
            self.coefficient = target;
            self.initialized = true;
        }
        for i in 0..samples {
            self.coefficient += self.smoothing * (target - self.coefficient);
            // The complementary pairs meet at 800 Hz. A ringing bass
            // string must not hold open the treble envelope on the next pick.
            for (pair, envelope) in bands.chunks_exact_mut(2).zip(&mut self.envelopes) {
                let mut peak = 0.0_f64;
                for ch in 0..channels {
                    let input = f64::from(pair[0][ch][i]) + f64::from(pair[1][ch][i]);
                    peak = peak.max(input.abs());
                }
                let gain = envelope.gain(peak, self.rise, self.release, self.coefficient);
                for band in pair {
                    for channel in band.iter_mut().take(channels) {
                        channel[i] *= gain;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
