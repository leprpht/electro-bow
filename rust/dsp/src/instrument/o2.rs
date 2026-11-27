//! Provisional electrical response to O1 velocity, outside the guide transaction.
use std::f64::consts::{PI, SQRT_2, TAU};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct PrepareError;
#[derive(Debug, PartialEq, Eq)]
pub(super) struct OutputFault;

pub(super) struct O2Coefficients {
    c: f64,
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl O2Coefficients {
    pub fn prepare(fs: f64) -> Result<Self, PrepareError> {
        if !(32_000.0..=192_000.0).contains(&fs) {
            return Err(PrepareError);
        }
        let c = (-TAU * 20.0 / fs).exp();
        let k = (PI * 3200.0 / fs).tan();
        let d = 1.0 + SQRT_2 * k + k * k;
        let b0 = k * k / d;
        let b1 = 2.0 * b0;
        let b2 = b0;
        let a1 = 2.0 * (k * k - 1.0) / d;
        let a2 = (1.0 - SQRT_2 * k + k * k) / d;
        // Second-order Jury conditions for 1 + a1 z^-1 + a2 z^-2.
        if ![c, b0, b1, b2, a1, a2].into_iter().all(f64::is_finite)
            || !(0.0 < c && c < 1.0 && a2.abs() < 1.0 && a1.abs() < 1.0 + a2)
        {
            return Err(PrepareError);
        }
        Ok(Self {
            c,
            b0,
            b1,
            b2,
            a1,
            a2,
        })
    }
}

#[derive(Default)]
pub(super) struct O2Lane {
    v: f64,
    h: f64,
    z1: f64,
    z2: f64,
}

impl O2Lane {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn tick(&mut self, coeff: &O2Coefficients, v: f64, noise: f64) -> Result<f64, OutputFault> {
        Ok(self.advance(coeff, v, noise)?[3])
    }

    // The isolated replay can observe all four instantaneous taps of this update.
    fn advance(
        &mut self,
        coeff: &O2Coefficients,
        v: f64,
        noise: f64,
    ) -> Result<[f64; 4], OutputFault> {
        if !(-8.0..=8.0).contains(&v)
            || !(0.0..=1.0).contains(&noise)
            || ![self.v, self.h, self.z1, self.z2]
                .into_iter()
                .all(f64::is_finite)
        {
            return Err(OutputFault);
        }
        let h = coeff.c * ((self.h + v) - self.v);
        let a = 0.85 * h / SQRT_2;
        let l = coeff.b0 * a + self.z1;
        let z1 = (coeff.b1 * a - coeff.a1 * l) + self.z2;
        let z2 = coeff.b2 * a - coeff.a2 * l;
        let q = l + (0.025 * noise) * (a - l);
        if ![h, a, l, z1, z2, q].into_iter().all(f64::is_finite) {
            return Err(OutputFault);
        }
        *self = Self { v, h, z1, z2 };
        Ok([h, a, l, q])
    }
}

#[cfg(test)]
mod tests;
