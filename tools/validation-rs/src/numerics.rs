use crate::{Result, ensure, evidence};
use rustfft::{FftPlanner, num_complex::Complex64};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{f64::consts::TAU, fs, path::Path};

pub fn sum_numpy(x: &[f64]) -> f64 {
    if x.len() < 8 {
        return x.iter().fold(-0., |s, v| s + v);
    }
    if x.len() > 128 {
        let mid = (x.len() / 2) / 8 * 8;
        return sum_numpy(&x[..mid]) + sum_numpy(&x[mid..]);
    }
    let mut acc: [f64; 8] = x[..8].try_into().unwrap();
    let (groups, rem) = x[8..].as_chunks::<8>();
    for group in groups {
        for (a, v) in acc.iter_mut().zip(group) {
            *a += v;
        }
    }
    let [a, b, c, d, e, f, g, h] = acc;
    rem.iter()
        .fold(((a + b) + (c + d)) + ((e + f) + (g + h)), |s, v| s + v)
}

pub fn hann(n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![1.0];
    }
    (0..n)
        .map(|i| 0.5 - 0.5 * (TAU * i as f64 / (n - 1) as f64).cos())
        .collect()
}

pub fn spectrum(x: &[f64]) -> Vec<Complex64> {
    let mut data: Vec<_> = x.iter().map(|&v| Complex64::new(v, 0.)).collect();
    FftPlanner::new()
        .plan_fft_forward(x.len())
        .process(&mut data);
    data.truncate(x.len() / 2 + 1);
    data
}

pub fn partials(x: &[f64], rate: f64, hz: f64, count: usize) -> Vec<Complex64> {
    let weights = hann(x.len());
    let mean = x.iter().sum::<f64>() / x.len() as f64;
    let weight = weights.iter().sum::<f64>();
    (1..=count)
        .map(|k| hz * k as f64)
        .take_while(|f| *f < rate / 2.)
        .map(|f| {
            2. * x
                .iter()
                .zip(&weights)
                .enumerate()
                .map(|(i, (&v, &w))| {
                    Complex64::from_polar(w * (v - mean), -TAU * f * (i as f64 / rate))
                })
                .sum::<Complex64>()
                / weight
        })
        .collect()
}

#[derive(Deserialize)]
struct Case {
    case: String,
    samples: Vec<f64>,
    rms: f64,
    peak: f64,
    mean: f64,
    db: f64,
    hann: Vec<f64>,
    fft: Vec<[f64; 2]>,
    projections: Vec<[f64; 2]>,
}
pub fn differential(reference: &Path) -> Result<Value> {
    let path = reference.join("numerics.json");
    ensure(
        evidence::digest(&path)?
            == "679cf166c7771b166eb7e8a87d96ce7468df77f6501ac47924b0321071501e98",
        "frozen numerical corpus digest",
    )?;
    let cases: Vec<Case> = serde_json::from_value(evidence::strict(&fs::read(path)?)?)?;
    let mut report = Vec::new();
    for case in cases {
        let x = &case.samples;
        let n = x.len() as f64;
        let mean = x.iter().sum::<f64>() / n;
        let peak = x.iter().copied().map(f64::abs).fold(0., f64::max);
        let rms = (x.iter().map(|v| v * v).sum::<f64>() / n).sqrt();
        let db = 20. * rms.max(1e-12).log10();
        let mut errors = Vec::new();
        let mut compare = |name: &str, a: &[f64], b: &[f64]| -> Result<()> {
            ensure(a.len() == b.len(), "numeric row length")?;
            let mut max = 0.0_f64;
            for (i, (actual, wanted)) in a.iter().zip(b).enumerate() {
                let e = (actual - wanted).abs();
                ensure(
                    e <= 1e-12,
                    &format!("{} {name}[{i}] {actual} vs {wanted}", case.case),
                )?;
                max = max.max(e);
            }
            errors.push(json!({"field":name,"words":a.len(),"max_abs_error":max}));
            Ok(())
        };
        compare(
            "levels",
            &[mean, peak, rms, db],
            &[case.mean, case.peak, case.rms, case.db],
        )?;
        compare("hann", &hann(x.len()), &case.hann)?;
        for (name, actual, wanted) in [
            ("rfft", spectrum(x), case.fft),
            ("partials", partials(x, 32000., 4000., 12), case.projections),
        ] {
            let words: Vec<_> = actual.iter().flat_map(|z| [z.re, z.im]).collect();
            compare(name, &words, wanted.as_flattened())?;
        }
        report.push(json!({"case":case.case,"comparisons":errors}));
    }
    Ok(report.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analytical_boundaries() {
        assert_eq!(hann(1), [1.0]);
        assert_eq!(hann(3), [0., 1., 0.]);
        let bins = spectrum(&[1., -1., 1., -1.]);
        assert_eq!(
            bins,
            [Complex64::ZERO, Complex64::ZERO, Complex64::new(4., 0.)]
        );
        assert!(partials(&[0.; 7], 32000., 16000., 12).is_empty());
        assert_eq!(partials(&[0.; 7], 32000., 8000., 12), [Complex64::ZERO]);
        assert_eq!((2.5f64).round_ties_even(), 2.);
    }
}
