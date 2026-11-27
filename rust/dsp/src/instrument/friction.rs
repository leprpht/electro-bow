//! M3a–M3b bow junction. All velocities (including k = F_N / 2Z) are m/s.
//! The voice owns branch memory and commits it together with its guarded waves.

const Q: [f64; 10] = [0.0, 0.005, 0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28];
const SLIP_FLOOR: f64 = 1e-12;
const STATIC_MARGIN: f64 = 1e-12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Direction {
    Positive,
    Negative,
}

/// Stick is also the cleared memory after lift. M4 seeds a launch on Slide(+).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Mode {
    Stick,
    Slide(Direction),
}

#[derive(Clone, Copy)]
pub(super) struct ContactInput {
    pub bow_speed: f64,
    pub from_nut: f64,
    pub from_bridge: f64,
    pub k: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ContactResult {
    pub mode: Mode,
    pub slip: f64,
    /// Junction velocity increment j = F / (2Z), in m/s.
    pub j: f64,
    /// Force on the string, in N, at Z = 1 N·s/m.
    pub force: f64,
    pub string_velocity: f64,
    pub to_bridge: f64,
    pub to_nut: f64,
    pub segment_probes: u8,
}

/// No outgoing sample or next mode is available on failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ContactFault;

/// Immutable prepared coefficients for M3a. May be shared by both lanes.
pub(super) struct FrictionCurve {
    mu: [f64; Q.len()],
}

impl FrictionCurve {
    pub fn prepare() -> Self {
        Self {
            mu: Q.map(|q| 0.35 + 0.85 * (-q / 0.01).exp() + 0.45 * (-q / 0.1).exp()),
        }
    }

    /// Evaluate M3 scattering without writing voice, FIR or ring state.
    pub fn solve(
        &self,
        input: ContactInput,
        previous: Mode,
    ) -> Result<ContactResult, ContactFault> {
        let a = input.from_nut;
        let b = input.from_bridge;
        let k = input.k;
        let d = input.bow_speed - a - b;
        // A nonfinite incident wave/speed also produces a nonfinite mismatch.
        if !d.is_finite() || !k.is_finite() || k < 0.0 {
            return Err(ContactFault);
        }

        let limit = k * self.mu[0];
        if !limit.is_finite() {
            return Err(ContactFault);
        }
        let h = d.abs();
        let direction = if d > 0.0 {
            Direction::Positive
        } else {
            Direction::Negative
        };
        let mut probes = 0;
        let (j, slip, mode) = if k == 0.0 {
            (0.0, d, Mode::Stick)
        } else if d == 0.0 || (previous != Mode::Slide(direction) && h <= limit) {
            (d, 0.0, Mode::Stick)
        } else if let Some(q) = self.upper_slip(h, k, &mut probes) {
            if q <= SLIP_FLOOR {
                (d, 0.0, Mode::Stick)
            } else {
                let slip = q.copysign(d);
                (d - slip, slip, Mode::Slide(direction))
            }
        } else if h <= limit + STATIC_MARGIN {
            (d.clamp(-limit, limit), 0.0, Mode::Stick)
        } else {
            return Err(ContactFault);
        };

        let force = 2.0 * j;
        let string_velocity = a + b + j;
        let to_bridge = a + j;
        let to_nut = b + j;
        if ![force, string_velocity, to_bridge, to_nut]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(ContactFault);
        }
        Ok(ContactResult {
            mode,
            slip,
            j,
            force,
            string_velocity,
            to_bridge,
            to_nut,
            segment_probes: probes,
        })
    }

    /// Search the *stored binary64 loads*, highest slip first. The checked
    /// static limit bounds every k*mu; adding a knot (<=1.28) cannot overflow.
    fn upper_slip(&self, h: f64, k: f64, probes: &mut u8) -> Option<f64> {
        let last = Q.len() - 1;
        let tail = k * self.mu[last];
        let mut right = Q[last] + tail;
        if h == right {
            return Some(Q[last]);
        }
        if h > right {
            return Some(h - tail);
        }
        for i in (0..last).rev() {
            *probes += 1;
            let left = Q[i] + k * self.mu[i];
            if left <= h && h <= right {
                // Includes the coincident-load tie, and returns knots exactly.
                if h == right {
                    return Some(Q[i + 1]);
                }
                // left <= h < right implies a finite, positive denominator.
                let fraction = (h - left) / (right - left);
                return Some(Q[i] + (Q[i + 1] - Q[i]) * fraction);
            }
            right = left;
        }
        None
    }
}

#[cfg(test)]
mod tests;
