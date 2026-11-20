//! A small damped acoustic space driven by the recombined string signal.
//! Four unequal delays and an orthogonal scattering matrix spread reflections
//! in time. Each feedback path loses 60 dB over the selected decay time.
#![forbid(unsafe_code)]

const LENGTHS: [f64; 4] = [0.0071, 0.0097, 0.0137, 0.0179];

struct Line {
    frames: Vec<[f32; 2]>,
    index: usize,
    damped: [f64; 2],
    log_decay: f64,
    feedback: f64,
}

pub struct Resonance {
    lines: [Line; 4],
    damping: f64,
    smoothing: f64,
    mix: f64,
}

impl Resonance {
    pub fn new(rate: f64) -> Option<Self> {
        let mut lines = LENGTHS.map(|_| Line {
            frames: Vec::new(),
            index: 0,
            damped: [0.0; 2],
            log_decay: 0.0,
            feedback: 0.0,
        });
        for (line, seconds) in lines.iter_mut().zip(LENGTHS) {
            let size = (seconds * rate).round().max(1.0) as usize;
            line.frames.try_reserve_exact(size).ok()?;
            line.frames.resize(size, [0.0; 2]);
            line.log_decay = -1000.0_f64.ln() * size as f64 / rate;
        }
        Some(Self {
            lines,
            damping: 1.0 - (-std::f64::consts::TAU * 3100.0 / rate).exp(),
            smoothing: 1.0 - (-1.0 / (0.015 * rate)).exp(),
            mix: 0.0,
        })
    }

    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.frames.fill([0.0; 2]);
            line.index = 0;
            line.damped = [0.0; 2];
            line.feedback = 0.0;
        }
        self.mix = 0.0;
    }

    pub fn process(&mut self, channels: &mut [&mut [f32]], ms: f64) {
        let time = ms.clamp(0.0, 1000.0) * 0.001;
        let target_mix = 0.30 * time / (time + 0.14);
        let targets = self.lines.each_ref().map(|line| {
            if time > 0.0 {
                (line.log_decay / time).exp()
            } else {
                0.0
            }
        });
        for i in 0..channels[0].len() {
            self.mix += self.smoothing * (target_mix - self.mix);
            for (line, target) in self.lines.iter_mut().zip(targets) {
                line.feedback += self.smoothing * (target - line.feedback);
            }
            for (ch, channel) in channels.iter_mut().enumerate() {
                let input = f64::from(channel[i]);
                let mut returns = [0.0; 4];
                for (ret, line) in returns.iter_mut().zip(&mut self.lines) {
                    line.damped[ch] +=
                        self.damping * (f64::from(line.frames[line.index][ch]) - line.damped[ch]);
                    *ret = line.damped[ch];
                }
                let [a, b, c, d] = returns;
                // Householder reflection. Distinct input/output sign patterns
                // avoid a large coherent first echo or DC buildup.
                let half_sum = 0.5 * (a + b + c + d);
                for ((line, ret), sign) in self
                    .lines
                    .iter_mut()
                    .zip(returns)
                    .zip([0.5, -0.5, 0.5, 0.5])
                {
                    line.frames[line.index][ch] =
                        (sign * input + line.feedback * (ret - half_sum)) as f32;
                }
                channel[i] = (input + self.mix * 0.5 * (a + b - c + d)) as f32;
            }
            for line in &mut self.lines {
                line.index += 1;
                if line.index == line.frames.len() {
                    line.index = 0;
                }
            }
        }
    }
}
