//! Warped analog transfer and chronological differences convolved with a sampled pole pair.
use rustfft::{FftPlanner, num_complex::Complex64 as C};
use std::f64::consts::{PI, SQRT_2, TAU};
pub fn load(fs: f64, hz: f64) -> C {
    let u = (PI * hz / fs).tan() / (PI * 3200. / fs).tan();
    C::ONE / C::new(1. - u * u, SQRT_2 * u)
}
pub fn transfer(fs: f64, hz: f64, depth: f64) -> C {
    let omega = TAU * hz / fs;
    let c = (-TAU * 20. / fs).exp();
    let dc = c * C::new(2. * (omega / 2.).sin().powi(2), omega.sin())
        / (C::ONE - c * C::from_polar(1., -omega));
    let lp = load(fs, hz);
    dc * (lp + 0.025 * depth * (C::ONE - lp))
}
pub fn taps(fs: f64, x: &[f64]) -> Vec<[f64; 3]> {
    let c = (-TAU * 20. / fs).exp();
    let mut h = 0.;
    let mut previous = 0.;
    let mut rows = Vec::with_capacity(x.len());
    for v in x {
        h = c * (v - previous) + c * h;
        previous = *v;
        rows.push([h, 0.85 * h / SQRT_2, 0.]);
    }
    let analog = C::from_polar((PI * 3200. / fs).tan(), 3. * PI / 4.);
    let pole = (C::ONE + analog) / (C::ONE - analog);
    let n = x
        .len()
        .min((1e-22_f64.ln() / pole.norm().ln()).ceil() as usize);
    let scale = (C::ONE - pole).norm_sqr() / 4.;
    let size = (x.len() + n + 1).next_power_of_two();
    let mut a = vec![C::ZERO; size];
    let mut kernel = vec![C::ZERO; size];
    let mut power = pole;
    for k in 0..n {
        let value = scale * power.im / pole.im;
        kernel[k].re += value;
        kernel[k + 1].re += 2. * value;
        kernel[k + 2].re += value;
        power *= pole;
    }
    for (i, row) in rows.iter().enumerate() {
        a[i].re = row[1];
    }
    let mut planner = FftPlanner::new();
    let forward = planner.plan_fft_forward(size);
    forward.process(&mut a);
    forward.process(&mut kernel);
    for (v, k) in a.iter_mut().zip(kernel) {
        *v *= k;
    }
    planner.plan_fft_inverse(size).process(&mut a);
    for (r, v) in rows.iter_mut().zip(a) {
        r[2] = v.re / size as f64;
    }
    rows
}
