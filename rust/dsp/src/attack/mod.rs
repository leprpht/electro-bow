//! A linked, level-relative swell on the original audio. Tracking a slower
//! envelope underneath a fast one softens re-picks without a trigger or gate.
#![forbid(unsafe_code)]

pub struct Attack {
    rate: f64,
    rise: f64,
    release: f64,
    smoothing: f64,
    coefficient: f64,
    held: f64,
    fast: f64,
    bowed: f64,
    initialized: bool,
}

impl Attack {
    pub fn new(rate: f64) -> Self {
        Self {
            rate,
            rise: 1.0 - (-1.0 / (0.002 * rate)).exp(),
            release: (-1.0 / (0.025 * rate)).exp(),
            smoothing: 1.0 - (-1.0 / (0.008 * rate)).exp(),
            coefficient: 1.0,
            held: 0.0,
            fast: 0.0,
            bowed: 0.0,
            initialized: false,
        }
    }

    pub fn reset(&mut self) {
        self.held = 0.0;
        self.fast = 0.0;
        self.bowed = 0.0;
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
            let mut peak = 0.0_f64;
            for ch in 0..channels {
                let input: f64 = bands.iter().map(|b| f64::from(b[ch][i])).sum();
                peak = peak.max(input.abs());
            }
            self.held = peak.max(self.held * self.release);
            self.fast += self.rise * (self.held - self.fast);
            // Discharge between strokes, but retain enough envelope across
            // individual low-string cycles to avoid low-frequency ripple.
            let follow = if self.fast < self.bowed {
                self.coefficient.max(1.0 - self.release)
            } else {
                self.coefficient
            };
            self.bowed += follow * (self.fast - self.bowed);
            let gain = if self.fast > 1e-30 {
                (1.10 * self.bowed / self.fast).min(1.0)
            } else {
                1.0
            };
            for band in bands.iter_mut() {
                for channel in band.iter_mut().take(channels) {
                    channel[i] *= gain as f32;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
