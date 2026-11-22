//! String-driven contact noise, low-mid saturation and slow spectral tilt.
//! One noise source and one modulation phase keep stereo modulation coherent.
#![forbid(unsafe_code)]
use crate::PanelControls;

pub(crate) struct Texture {
    depths: [f64; 3],
    initialized: bool,
    smoothing: f64,
    seed: u32,
    noise_filters: [f64; 2],
    coefficients: [f64; 2],
    phase: f64,
    phase_step: f64,
}

impl Texture {
    pub(crate) fn new(rate: f64) -> Self {
        Self {
            depths: [0.0; 3],
            initialized: false,
            smoothing: 1.0 - (-1.0 / (rate * 0.015)).exp(),
            seed: 1,
            noise_filters: [0.0; 2],
            coefficients: [700.0, 4800.0]
                .map(|hz: f64| 1.0 - (-std::f64::consts::TAU * (hz / rate).min(0.45)).exp()),
            phase: 0.25,
            phase_step: (0.8 / rate).min(0.5),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.initialized = false;
        self.seed = 1;
        self.noise_filters = [0.0; 2];
        self.phase = 0.25;
    }

    pub(crate) fn process(
        &mut self,
        bands: &mut [[Vec<f32>; 2]; 4],
        channels: usize,
        samples: usize,
        character: f64,
        panel: PanelControls,
    ) {
        let targets = [panel.noise, panel.saturation, panel.spectral_movement]
            .map(|x| x.clamp(0.0, 1.0) * character.clamp(0.0, 1.0));
        if !self.initialized {
            self.depths = targets;
            self.initialized = true;
        }
        let [low, wood, presence, edge] = bands;
        for i in 0..samples {
            for (depth, target) in self.depths.iter_mut().zip(targets) {
                *depth += self.smoothing * (target - *depth);
                if (*depth - target).abs() < 1e-12 {
                    *depth = target;
                }
            }
            let [noise, saturation, movement] = self.depths;
            self.seed ^= self.seed << 13;
            self.seed ^= self.seed >> 17;
            self.seed ^= self.seed << 5;
            let white = f64::from(self.seed) * (2.0 / f64::from(u32::MAX)) - 1.0;
            for (state, coefficient) in self.noise_filters.iter_mut().zip(self.coefficients) {
                *state += coefficient * (white - *state);
            }
            let grain = noise * 0.48 * (self.noise_filters[1] - self.noise_filters[0]);
            let tilt = movement * (1.0 - 4.0 * (self.phase - 0.5).abs());
            self.phase = (self.phase + self.phase_step).fract();
            for ch in 0..channels {
                let x = f64::from(wood[ch][i]);
                let contact = (f64::from(low[ch][i]) + x) * grain;
                // Odd-symmetric soft knee on the 200–800 Hz band only.
                wood[ch][i] = (x * (1.0 + 0.7 * saturation) / (1.0 + 4.0 * saturation * x.abs())
                    * (1.0 - 0.24 * tilt)) as f32;
                presence[ch][i] *= (1.0 + 0.45 * tilt) as f32;
                edge[ch][i] = (f64::from(edge[ch][i]) * (1.0 + 0.65 * tilt) + contact) as f32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FRAMES: usize = 2000;

    fn render(panel: PanelControls, amount: f64, input: [f32; 4]) -> [[Vec<f32>; 2]; 4] {
        let mut bands = input.map(|x| [vec![x; FRAMES], vec![-x; FRAMES]]);
        Texture::new(8000.0).process(&mut bands, 2, FRAMES, amount, panel);
        bands
    }

    #[test]
    fn noise_depends_on_string_signal_and_tracks_stereo_polarity() {
        let panel = PanelControls {
            noise: 1.0,
            ..PanelControls::NEUTRAL
        };
        let result = render(panel, 1.0, [0.5, 0.0, 0.0, 0.0]);
        assert!(result[3][0].windows(2).any(|s| (s[0] - s[1]).abs() > 0.01));
        for i in 0..FRAMES {
            assert_eq!(result[3][0][i], -result[3][1][i]);
        }
        assert!(
            render(panel, 1.0, [0.0; 4])
                .iter()
                .flatten()
                .flatten()
                .all(|&x| x == 0.0)
        );
    }

    #[test]
    fn saturation_compresses_wood_crests_and_preserves_other_bands() {
        let panel = PanelControls {
            saturation: 1.0,
            ..PanelControls::NEUTRAL
        };
        let loud = render(panel, 1.0, [0.25, 0.8, 0.3, 0.4]);
        let quiet = render(panel, 1.0, [0.25, 0.05, 0.3, 0.4]);
        assert!(loud[1][0][0] / 0.8 < quiet[1][0][0] / 0.05);
        for (b, x) in [(0, 0.25), (2, 0.3), (3, 0.4)] {
            assert_eq!(loud[b][0][0], x);
        }
    }

    #[test]
    fn movement_exchanges_energy_between_bands_over_time() {
        let panel = PanelControls {
            spectral_movement: 1.0,
            ..PanelControls::NEUTRAL
        };
        let result = render(panel, 1.0, [0.2; 4]);
        assert_eq!(result[0][0], vec![0.2; FRAMES]);
        assert!(result[1][0][FRAMES - 1] < result[1][0][0] - 0.02);
        assert!(result[2][0][FRAMES - 1] > result[2][0][0] + 0.04);
        assert!(result[3][0][FRAMES - 1] > result[3][0][0] + 0.06);
    }

    #[test]
    fn zero_character_disables_all_details() {
        let panel = PanelControls {
            noise: 1.0,
            saturation: 1.0,
            spectral_movement: 1.0,
            out: 0.5,
        };
        assert_eq!(
            render(panel, 0.0, [0.1; 4]),
            render(PanelControls::NEUTRAL, 1.0, [0.1; 4])
        );
    }
}
