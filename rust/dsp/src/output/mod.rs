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
mod tests;
