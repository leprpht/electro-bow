//! Centered upfirdn with SciPy's odd, DC-normalized Kaiser(10) prototype and zero extension.
use std::f64::consts::PI;
fn i0(x: f64) -> f64 {
    let mut sum = 1.;
    let mut term = 1.;
    for j in 1..=64 {
        term *= (x / (2. * f64::from(j))).powi(2);
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}
pub fn common_window(x: &[f64], rate: usize) -> Vec<f64> {
    window(x, rate, 48000..80000)
}
pub fn window(x: &[f64], rate: usize, interval: std::ops::Range<usize>) -> Vec<f64> {
    if rate == 32000 {
        return x[interval].to_vec();
    }
    let (mut a, mut b) = (rate, 32000);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let up = 32000 / a;
    let down = rate / a;
    let scale = up.max(down);
    let half = 10 * scale;
    let mut h: Vec<_> = (0..=2 * half)
        .map(|i| {
            let t = i as f64 - half as f64;
            let u = t / scale as f64;
            let sinc = if u == 0. {
                1.
            } else {
                (PI * u).sin() / (PI * u)
            };
            sinc * i0(10. * (1. - (t / half as f64).powi(2)).max(0.).sqrt()) / i0(10.)
        })
        .collect();
    let sum = h.iter().sum::<f64>();
    for v in &mut h {
        *v = *v / sum * up as f64;
    }
    interval
        .map(|m| {
            let at = m * down + half;
            let first = at.saturating_sub(2 * half).div_ceil(up);
            let last = (at / up).min(x.len() - 1);
            (first..=last).map(|i| x[i] * h[at - i * up]).sum()
        })
        .collect()
}
