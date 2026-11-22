//! Stereo-linked zero-lookahead sample-peak protection. No gain change below
//! the ceiling until an overload occurs; recovery is shared across channels.
#![forbid(unsafe_code)]
pub struct Output {
    trim: Option<f64>,
    trim_step: f64,
    gain: f64,
    release: f64,
}

impl Output {
    pub fn new(rate: f64) -> Self {
        Self {
            trim: None,
            trim_step: 2.0 / (0.01 * rate),
            gain: 1.0,
            release: (-1.0 / (0.050 * rate)).exp(),
        }
    }
    pub fn reset(&mut self) {
        self.trim = None;
        self.gain = 1.0;
    }
    pub fn process(&mut self, channels: &mut [&mut [f32]], out: f64) {
        let target = out.clamp(0.0, 1.0) * 2.0;
        for i in 0..channels[0].len() {
            let trim = self.trim.get_or_insert(target);
            *trim += (target - *trim).clamp(-self.trim_step, self.trim_step);
            let peak = channels
                .iter()
                .map(|c| {
                    if c[i].is_finite() {
                        f64::from(c[i]).abs()
                    } else {
                        0.0
                    }
                })
                .fold(0.0, f64::max)
                * *trim;
            let required = if peak > 0.98 { 0.98 / peak } else { 1.0 };
            self.gain = required.min(1.0 - (1.0 - self.gain) * self.release);
            for channel in channels.iter_mut() {
                channel[i] = if channel[i].is_finite() {
                    (f64::from(channel[i]) * *trim * self.gain) as f32
                } else {
                    0.0
                };
            }
        }
    }
}

#[cfg(test)]
mod tests;
