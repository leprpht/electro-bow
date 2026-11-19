//! Input-derived spectral friction. No oscillator, pitch model, voices or additive noise.
#![forbid(unsafe_code)]

pub struct Character {
    amount: f64,
    smoothing: f64,
    envelope_decay: f64,
    reference_rate: f64,
    drift_rate: f64,
    noise_rate: f64,
    envelope: [f64; 4],
    reference: [f64; 4],
    drift: f64,
    noise: f64,
    seed: u32,
}
impl Character {
    pub fn new(rate: f64) -> Self {
        Self {
            amount: 0.0,
            smoothing: 1.0 - (-1.0 / (0.020 * rate)).exp(),
            envelope_decay: (-1.0 / (0.015 * rate)).exp(),
            reference_rate: 1.0 - (-1.0 / (0.080 * rate)).exp(),
            drift_rate: 1.0 - (-1.0 / (0.120 * rate)).exp(),
            noise_rate: 1.0 - (-std::f64::consts::TAU * 1800.0_f64.min(rate * 0.15) / rate).exp(),
            envelope: [0.0; 4],
            reference: [0.0; 4],
            drift: 0.0,
            noise: 0.0,
            seed: 0x613e9a5b,
        }
    }
    pub fn reset(&mut self) {
        self.amount = 0.0;
        self.envelope = [0.0; 4];
        self.reference = [0.0; 4];
        self.drift = 0.0;
        self.noise = 0.0;
        self.seed = 0x613e9a5b;
    }
    pub fn process(
        &mut self,
        bands: &mut [[Vec<f32>; 2]; 4],
        channels: usize,
        samples: usize,
        target: f64,
    ) {
        let target = target.clamp(0.0, 1.0);
        for i in 0..samples {
            self.amount += self.smoothing * (target - self.amount);
            self.seed ^= self.seed << 13;
            self.seed ^= self.seed >> 17;
            self.seed ^= self.seed << 5;
            let random = f64::from(self.seed) / f64::from(u32::MAX) * 2.0 - 1.0;
            self.noise += self.noise_rate * (random - self.noise);
            // Irregular bounded spectral movement; no periodic modulation or delay line.
            self.drift += self.drift_rate * (random - self.drift);
            let movement = (self.drift * 20.0).clamp(-1.0, 1.0);
            for (b, band) in bands.iter_mut().enumerate() {
                let peak = band
                    .iter()
                    .take(channels)
                    .map(|c| f64::from(c[i]).abs())
                    .fold(0.0, f64::max);
                self.envelope[b] = peak.max(self.envelope[b] * self.envelope_decay);
                self.reference[b] += self.reference_rate * (self.envelope[b] - self.reference[b]);
                let relative = if self.envelope[b] > 1e-30 {
                    (self.reference[b] / self.envelope[b]).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let a = self.amount;
                let spectral = match b {
                    1 => 1.0 + a * (0.08 + 0.12 * relative + 0.015 * movement),
                    2 => 1.0 + a * (0.04 + 0.10 * relative - 0.02 * movement),
                    3 => 1.0 - a * (0.10 + 0.15 * relative),
                    _ => 1.0,
                };
                // Noise modulates existing upper-band samples: exactly zero input
                // produces exactly zero texture, even while detector history decays.
                let friction = if b >= 2 {
                    1.0 + 0.12 * a * a * self.noise * relative
                } else {
                    1.0
                };
                for channel in band.iter_mut().take(channels) {
                    let x = f64::from(channel[i]);
                    // Bounded soft compression only in middle bands. At most 3%
                    // departure from linear; avoids broadband hard-clipping IMD.
                    let bend = if (b == 1 || b == 2) && self.envelope[b] > 1e-30 {
                        let u = x / self.envelope[b];
                        1.0 - 0.03 * a * u * u / (1.0 + u * u)
                    } else {
                        1.0
                    };
                    channel[i] = (x * spectral * friction * bend) as f32;
                }
                if self.envelope[b] < 1e-30 {
                    self.envelope[b] = 0.0;
                }
                if self.reference[b] < 1e-30 {
                    self.reference[b] = 0.0;
                }
            }
        }
    }
}
