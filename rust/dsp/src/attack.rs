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
