//! Independent fixed-bin Stage 08 monitor used for the previous-stationary audition.
use crate::{Result, ensure};
use rustfft::num_complex::Complex64;
use serde_json::{Value, json};
use std::f64::consts::TAU;
fn power(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64
}
pub(super) fn allowed(x: &[f64], phase: &[Complex64], m5: f64, base: f64, recovery: bool) -> bool {
    let n = x.len() as f64;
    let mut sums = [Complex64::ZERO; 4];
    for (i, (&v, &z)) in x.iter().zip(phase).enumerate() {
        sums[0].re += v;
        sums[1] += v * z;
        sums[2] += v * z * z;
        sums[3].re += if i % 2 == 0 { v } else { -v };
    }
    let [dc, h1, h2, ny] = sums.map(|z| z.norm() / n);
    let p = power(x);
    let used = dc * dc + 2. * h1 * h1 + 2. * h2 * h2 + ny * ny;
    let rest = (p - used).max(0.);
    let factor = 10f64.powf(if recovery { 2.85 } else { 3. } / 10.);
    let margin = 10f64.powf(if recovery { 0.30 } else { 0.05 } / 20.);
    m5 > 1e-24
        && base > 1e-24
        && h1 > 1e-12
        && h1 >= margin * h2.max(ny).max((rest / 2.).sqrt()) + 1e-12
        && p >= m5 / factor
        && p <= m5 * factor
        && p >= base / factor
        && p <= base * factor
}
pub fn scales(frames: &[[f64; 20]], output: &[[f64; 20]], fs: f64) -> Result<Vec<f64>> {
    Ok(monitor(frames, output, fs)?.0)
}
pub(super) fn monitor(
    frames: &[[f64; 20]],
    output: &[[f64; 20]],
    fs: f64,
) -> Result<(Vec<f64>, Vec<Value>)> {
    let mut log = Vec::new();
    let mut scale = vec![0.; frames.len()];
    let mut next = 0f64;
    let mut target = 0f64;
    let mut previous = 0f64;
    let mut ready = 0;
    let mut start = 0;
    let ramp = (0.005 * fs + 0.5).floor().max(1.);
    while start < frames.len() {
        let length = 2 * (4. * fs / frames[start][0] + 0.5).floor() as usize;
        let end = (start + length).min(frames.len());
        let cut = (start + 17).min(end);
        for (a, b, dest) in [(start, cut, target), (cut, end, next)] {
            let initial = previous;
            for (i, s) in scale[a..b].iter_mut().enumerate() {
                *s = initial
                    + (dest - initial).signum() * (dest - initial).abs().min((i + 1) as f64 / ramp);
                previous = *s;
            }
        }
        target = next;
        if end - start != length {
            break;
        }
        let window = &frames[start..end];
        let data = &output[start..end];
        let phase = (0..length)
            .map(|i| Complex64::from_polar(1., -TAU * 8. / length as f64 * i as f64))
            .collect::<Vec<_>>();
        let m5 = window.iter().map(|r| r[1]).collect::<Vec<_>>();
        let mp = power(&m5);
        let (lo, hi) = window.iter().fold((f64::INFINITY, 0f64), |(l, h), r| {
            (l.min(r[0]), h.max(r[0]))
        });
        let conditioned = 1200. * (hi / lo).log2() <= 25.;
        let observable = window.iter().all(|r| r[18] != 0.);
        let bridge_ok = conditioned && allowed(&m5, &phase, mp, mp, false);
        let available = observable && bridge_ok;
        let test = |amount: Option<f64>, recovery: bool| {
            let mut wet = [vec![], vec![], vec![]];
            for j in 0..3 {
                let q = window.iter().map(|r| r[j + 2]).collect::<Vec<_>>();
                wet[j] = q
                    .iter()
                    .zip(data)
                    .enumerate()
                    .map(|(i, (q, d))| q + amount.unwrap_or(scale[start + i]) * d[j])
                    .collect();
                if !allowed(&wet[j], &phase, mp, power(&q), recovery) {
                    return false;
                }
            }
            let diff = wet[1]
                .iter()
                .zip(&wet[0])
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>();
            power(&diff) <= 10f64.powf(if recovery { -2.85 } else { -2.8 }) * power(&wet[0])
        };
        let realized_ok = conditioned && test(None, false);
        let neutral_ok = conditioned && test(Some(0.), true);
        let usable = available && realized_ok && neutral_ok;
        ready = if usable { (ready + 1).min(2) } else { 0 };
        next = 0.;
        if ready == 2 {
            next = (0..=16)
                .rev()
                .map(|j| f64::from(j) / 16.)
                .find(|s| test(Some(*s), true))
                .ok_or("neutral admissible ray")?;
        }
        log.push(json!({"start":start,"end":end,"conditioned":conditioned,"observable":observable,"trusted":true,"available":available,"bridge_ok":bridge_ok,"neutral_ok":neutral_ok,"realized_ok":realized_ok,"veto":ready!=2,"scale":next}));
        start = end;
    }
    ensure(
        scale.iter().all(|v| (0. ..=1.).contains(v)),
        "fixed-bin monitor scale",
    )?;
    Ok((scale, log))
}
