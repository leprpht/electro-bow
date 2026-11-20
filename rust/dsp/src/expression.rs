//! Dynamics and contact colour, measured before the onset swell so its gain
//! reduction can never drive the sustain circuit into boosting the pick.
#![forbid(unsafe_code)]
use crate::BowControls;

pub struct Expression {
    smoothing: f64,
    release: f64,
    recent_release: f64,
    recent_peak: f64,
    memory_release: f64,
    level: f64,
    memory: f64,
    lift: f64,
    band_peak: [[f64; 2]; 2],
    controls: [f64; 4],
    initialized: bool,
}

impl Expression {
    pub fn new(rate: f64) -> Self {
        Self {
            smoothing: 1.0 - (-1.0 / (0.012 * rate)).exp(),
            release: (-1.0 / (0.035 * rate)).exp(),
            recent_release: (-1.0 / (0.012 * rate)).exp(),
            recent_peak: 0.0,
            memory_release: (-1.0 / (0.95 * rate)).exp(),
            level: 0.0,
            memory: 0.0,
            lift: 0.0,
            band_peak: [[0.0; 2]; 2],
            controls: [0.0, 0.0, 0.5, 0.0],
            initialized: false,
        }
    }

    pub fn input_level(&self) -> f32 {
        self.level as f32
    }

    pub fn reset(&mut self) {
        self.level = 0.0;
        self.recent_peak = 0.0;
        self.memory = 0.0;
        self.lift = 0.0;
        self.band_peak = [[0.0; 2]; 2];
        self.initialized = false;
    }

    pub fn process(
        &mut self,
        bands: &mut [[Vec<f32>; 2]; 4],
        channels: usize,
        samples: usize,
        params: &BowControls,
    ) {
        let targets = [
            params.sustain,
            params.pressure,
            params.speed,
            params.friction,
        ]
        .map(|x| x.clamp(0.0, 1.0));
        if !self.initialized && samples > 0 {
            self.controls = targets;
            self.initialized = true;
        }
        for i in 0..samples {
            for (value, target) in self.controls.iter_mut().zip(targets) {
                *value += self.smoothing * (target - *value);
            }
            let [sustain, pressure, speed, friction] = self.controls;
            let mut peak = 0.0_f64;
            for ch in 0..channels {
                let input: f64 = bands.iter().map(|band| f64::from(band[ch][i])).sum();
                peak = peak.max(input.abs());
            }
            self.level = peak.max(self.level * self.release);
            self.recent_peak = peak.max(self.recent_peak * self.recent_release);
            // Once the string is muted, forget the preceding stroke. Leave
            // headroom above the 4:1 makeup range for low-string cycles.
            self.memory = self
                .level
                .max(self.memory * self.memory_release)
                .min(6.0 * self.recent_peak);
            // A falling string can earn up to 10 dB of makeup. The floor taper
            // withdraws that gain as the pickup settles back into idle noise.
            let activity = ((self.level - 0.0015) / 0.009).clamp(0.0, 1.0);
            let falling = (self.memory / self.level.max(0.0015) - 1.0).clamp(0.0, 3.0);
            self.lift += self.smoothing * (0.72 * falling * activity - self.lift);
            let compression = (1.0 + 0.26 * pressure) / (1.0 + 1.9 * pressure * self.level);
            // A fresh accent can consume all the available makeup in one
            // cycle. Never carry a decaying string's gain onto that crest.
            let gain = (1.0 + sustain * self.lift.min(0.72 * falling)) * compression;
            let tilt = 2.0 * speed - 1.0;
            let colour = [
                1.0 - 0.12 * tilt,
                1.0 + 0.33 * pressure,
                1.0 + 0.64 * tilt,
                1.0 + 0.82 * tilt,
            ];
            for (b, band) in bands.iter_mut().enumerate() {
                for (ch, data) in band.iter_mut().take(channels).enumerate() {
                    let x = f64::from(data[i]);
                    let contact = if b < 2 {
                        let env = &mut self.band_peak[b][ch];
                        *env = x.abs().max(*env * self.release);
                        let u = x / env.max(1e-30);
                        // Keep the cubic monotonic even at maximum friction
                        // and the lowest band gain (0.88). Folding the crests
                        // back introduces a false bass tone on low fifths.
                        friction * (0.48 - 0.40 * u * u)
                    } else {
                        0.0
                    };
                    data[i] = (x * gain * (colour[b] + contact)) as f32;
                }
            }
        }
    }
}
