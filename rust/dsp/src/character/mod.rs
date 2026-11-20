//! Shift energy into the woody low mids and soften the pickup's bright edge.
//! A small, band-limited waveshaping contribution follows the actual string.
#![forbid(unsafe_code)]

pub struct Character {
    amount: f64,
    smoothing: f64,
    release: f64,
    envelope: f64,
}

impl Character {
    pub fn new(rate: f64) -> Self {
        Self {
            amount: 0.0,
            smoothing: 1.0 - (-1.0 / (0.015 * rate)).exp(),
            release: (-1.0 / (0.035 * rate)).exp(),
            envelope: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.amount = 0.0;
        self.envelope = 0.0;
    }

    pub fn process(
        &mut self,
        bands: &mut [[Vec<f32>; 2]; 4],
        channels: usize,
        samples: usize,
        target: f64,
    ) {
        let target = target.clamp(0.0, 1.0);
        let [_, wood, presence, edge] = bands;
        for i in 0..samples {
            self.amount += self.smoothing * (target - self.amount);
            let peak = wood
                .iter()
                .take(channels)
                .map(|c| f64::from(c[i]).abs())
                .fold(0.0, f64::max);
            self.envelope = peak.max(self.envelope * self.release);
            for ch in 0..channels {
                let x = f64::from(wood[ch][i]);
                let u = x / self.envelope.max(1e-30);
                // A bounded cubic blend in the 200-800 Hz band. Odd symmetry
                // avoids a DC component. Treble never enters the nonlinearity.
                wood[ch][i] = (x * (1.0 + self.amount * (0.74 - 0.32 * u * u))) as f32;
                presence[ch][i] *= (1.0 - 0.22 * self.amount) as f32;
                edge[ch][i] *= (1.0 - 0.84 * self.amount) as f32;
            }
        }
    }
}

#[cfg(test)]
mod tests;
