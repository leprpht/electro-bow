//! Two short-lived formants per crossover band, with a shared moving pickup.
#![forbid(unsafe_code)]

const CENTERS: [[f64; 2]; 4] = [[114., 181.], [440., 690.], [1060., 1780.], [2820., 4100.]];
const QUALITY: [f64; 4] = [2., 3.6, 5., 5.5];
const PICKUP_GAIN: [f64; 4] = [1.60, 2.0, 2.25, 1.75];

struct Mode {
    numerator: f64,
    feedback: [f64; 2],
    history: [[f64; 2]; 2],
}

impl Mode {
    fn new(normalized_hz: f64, q: f64) -> Self {
        let (sin, cos) = (std::f64::consts::TAU * normalized_hz).sin_cos();
        let alpha = sin / (2.0 * q);
        let norm = 1.0 / (1.0 + alpha);
        Self {
            numerator: alpha * norm,
            feedback: [2.0 * cos * norm, (alpha - 1.0) * norm],
            history: [[0.0; 2]; 2],
        }
    }

    // Transposed direct form II, zeros at DC and Nyquist, unity gain at center.
    fn tick(&mut self, ch: usize, x: f64) -> f64 {
        let s = &mut self.history[ch];
        let drive = self.numerator * x;
        let y = drive + s[0];
        s[0] = s[1] + self.feedback[0] * y;
        s[1] = self.feedback[1] * y - drive;
        y
    }
}

pub(super) struct Body {
    modes: [[Mode; 2]; 4],
    phase: f64,
    increment: f64,
    seed: u32,
    noise_filters: [f64; 2],
    noise_coefficients: [f64; 2],
}

impl Body {
    pub(super) fn new(rate: f64) -> Self {
        let scale = (0.45 / 4100.0_f64).min(1.0 / rate);
        Self {
            modes: std::array::from_fn(|i| CENTERS[i].map(|hz| Mode::new(hz * scale, QUALITY[i]))),
            phase: 0.25,
            increment: (0.57 / rate).min(0.5),
            seed: 1,
            noise_filters: [0.; 2],
            noise_coefficients: [700., 4200.]
                .map(|hz| 1.0 - (-std::f64::consts::TAU * (hz / rate).min(0.45)).exp()),
        }
    }

    pub(super) fn reset(&mut self) {
        self.phase = 0.25;
        self.seed = 1;
        self.noise_filters = [0.; 2];
        for mode in self.modes.iter_mut().flatten() {
            mode.history = [[0.; 2]; 2];
        }
    }

    // Advance shared modulation once per frame, even at zero mix.
    pub(super) fn modulation(&mut self, noise: f64, movement: f64) -> (f64, f64) {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        let white = f64::from(self.seed) * (2.0 / f64::from(u32::MAX)) - 1.0;
        for (state, step) in self.noise_filters.iter_mut().zip(self.noise_coefficients) {
            *state += step * (white - *state);
        }
        let scatter = noise * 0.80 * (self.noise_filters[1] - self.noise_filters[0]);
        let t = 1.0 - 4.0 * (self.phase - 0.5).abs();
        self.phase += self.increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let blend = 0.45 + 0.43 * movement * t * (1.5 - 0.5 * t * t);
        (scatter, blend)
    }

    pub(super) fn pickup(
        &mut self,
        ch: usize,
        input: [f64; 4],
        scatter: f64,
        blend: f64,
        saturation: f64,
    ) -> f64 {
        let mut excitation = input;
        for x in &mut excitation[1..3] {
            let rounded = 1.35 * *x / (1.0 + 2.8 * x.abs());
            *x += saturation * (rounded - *x);
        }
        let grain = (input[0] + input[1]) * scatter;
        excitation[2] += grain;
        excitation[3] += grain;
        let mut pickup = 0.0;
        for ((pair, gain), drive) in self.modes.iter_mut().zip(PICKUP_GAIN).zip(excitation) {
            let a = pair[0].tick(ch, drive);
            let b = pair[1].tick(ch, drive);
            pickup += gain * (a + blend * (b - a));
        }
        pickup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_have_a_unit_peak_and_bounded_release() {
        for rate in [8000., 44100., 96000., 192000.] {
            for (pair, q) in CENTERS.into_iter().zip(QUALITY) {
                for hz in pair {
                    let center = hz * (0.45 / 4100.0_f64).min(1.0 / rate);
                    let mut mode = Mode::new(center, q);
                    let mut error = 0.0_f64;
                    for i in 0..rate as usize / 4 {
                        let x = (std::f64::consts::TAU * center * i as f64).sin();
                        let y = mode.tick(0, x);
                        if i > rate as usize / 8 {
                            error = error.max((y - x).abs());
                        }
                        assert_eq!(mode.tick(1, 0.0), 0.0);
                    }
                    assert!(error < 1e-6, "peak response {error} at {hz} Hz");
                    for _ in 0..rate as usize / 4 {
                        mode.tick(0, 0.);
                    }
                    assert!(mode.history[0].iter().all(|v| v.abs() < 1e-9));
                }
            }
        }
    }
}
