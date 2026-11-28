//! Independent full-window projection of the original injected stationary DI captures.
use crate::{Result, ensure, numerics, pitch};
use rustfft::num_complex::Complex64;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    f64::consts::{LN_10, TAU},
};
pub type Updates = Vec<(usize, [f64; 12])>;
pub(super) fn stimulus(rate: usize, hz: f64, kind: &str) -> Vec<f64> {
    (0..rate)
        .map(|i| {
            let t = i as f64 / rate as f64;
            (1..=12)
                .map(|h| {
                    let k = f64::from(h);
                    let amp = if kind == "weak-H1" {
                        match h {
                            1 => 0.01,
                            2 => 0.2,
                            _ => 0.08 / k,
                        }
                    } else {
                        let gain = match kind {
                            "missing-six" if h == 6 => 0.,
                            "dip-six" if h == 6 => 0.35,
                            "bright" if h >= 5 => 2.5,
                            "dark" if h >= 5 => 0.15,
                            _ => 1.,
                        };
                        0.12 / k * gain
                    };
                    amp * (TAU * k * hz * t).cos()
                })
                .sum()
        })
        .collect()
}
pub(super) fn target(x: &[f64], fa: f64, hz: f64) -> Option<[f64; 12]> {
    let n = x.len();
    let orders = ((0.28 * fa / hz).floor() as usize).min(12);
    let window = numerics::hann(n);
    let mean = x.iter().sum::<f64>() / n as f64;
    let total = window.iter().sum::<f64>();
    let amp: Vec<_> = (1..=orders)
        .map(|k| {
            let p = x
                .iter()
                .zip(&window)
                .enumerate()
                .map(|(i, (v, w))| {
                    Complex64::from_polar((v - mean) * w, -TAU * hz * i as f64 / fa * k as f64)
                })
                .sum::<Complex64>();
            (2. * p / total).norm()
        })
        .collect();
    let floor = (0.00032f64 * 3.)
        .max(1e-5)
        .max(0.015 * amp.iter().copied().fold(0., f64::max));
    let weights: Vec<_> = amp
        .iter()
        .map(|a| ((a - floor) / (3. * floor)).clamp(0., 1.))
        .collect();
    if weights.iter().filter(|v| **v > 0.).count() < 2 {
        return None;
    }
    let levels: Vec<_> = amp
        .iter()
        .enumerate()
        .map(|(i, v)| 20. * ((i + 1) as f64 * v.max(floor)).log10())
        .collect();
    let avg =
        weights.iter().zip(&levels).map(|(w, v)| w * v).sum::<f64>() / weights.iter().sum::<f64>();
    let mut out = [0.; 12];
    for i in 0..orders {
        out[i] = (0.45 * (levels[i] - avg) * weights[i]).clamp(-5., 5.);
    }
    Some(out)
}
pub fn updates(input: &[f64], rate: usize, hz: f64) -> Result<Updates> {
    ensure(
        rate >= 16000 && hz.is_finite() && (65. ..=1320.).contains(&hz),
        "DI projection rate/pitch",
    )?;
    let d = rate / 16000;
    let da = rate / 8000;
    let fs = rate as f64;
    let fa = fs / d as f64;
    let hop = d * (0.005 * fa + 0.5).floor() as usize;
    let ha = da * (0.005 * fs / da as f64 + 0.5).floor() as usize;
    let m = (4. * fa / hz + 0.5).floor() as usize;
    let count = (0.30 * fs).round_ties_even() as usize;
    ensure(input.len() >= count, "DI capture coverage")?;
    let taps = pitch::fir(d);
    let coeff = (-TAU * 30. / fs).exp();
    let level_a = -(-1. / (0.004 * fs)).exp_m1();
    let mut latest = 0.;
    let mut hp = vec![];
    let mut dec = vec![];
    let mut support = BTreeMap::new();
    let mut rms_state = 0.;
    let mut amp_hp_delay = 0.;
    let mut held = [0.; 12];
    let mut pending: Option<[f64; 12]> = None;
    let mut result = vec![];
    let alpha = -(-(hop as f64) / (fs * 0.040)).exp_m1();
    for (i, sample) in input[..count]
        .iter()
        .copied()
        .chain(std::iter::repeat_n(0., 3 * ha))
        .enumerate()
    {
        let prev = hp.last().copied().unwrap_or(0.);
        hp.push(coeff * (prev + sample - latest));
        latest = sample;
        let a = coeff.mul_add(sample, amp_hp_delay);
        amp_hp_delay = (-coeff).mul_add(sample, coeff * a);
        let energy = level_a.mul_add(a * a, rms_state);
        rms_state = (1. - level_a) * energy;
        if (i + 1) % d == 0 {
            let v = taps.iter().zip(hp.iter().rev()).map(|(t, s)| t * s).sum();
            dec.push(v);
        }
        if (i + 1) % ha == 0 {
            support.insert(i, energy.sqrt() >= 0.001 && i < count);
        }
        if i == 2 * ha - 1 {
            result.push((i, held));
        }
        if (i + 1) % hop != 0 {
            continue;
        }
        if let Some(trial) = pending.take()
            && support.last_key_value().is_some_and(|(_, v)| *v)
        {
            for (j, val) in held.iter_mut().enumerate() {
                *val += alpha * (trial[j] - *val);
            }
            result.push((i, held.map(|v| (LN_10 * v / 20.).exp_m1())));
        }
        let span = (m - 1) * d;
        if i < span + 32 * d || i - span - 16 * d < (0.025 * fs).ceil() as usize {
            continue;
        }
        let center = i - span - 16 * d;
        let first = (center + 1) / ha * ha;
        if first == 0 || !support.range(first - 1..=i).all(|(_, ok)| *ok) {
            continue;
        }
        pending = target(&dec[dec.len() - m..], fa, hz);
    }
    Ok(result)
}
pub fn run() -> Result<Value> {
    let contract: Value = serde_json::from_str(include_str!("../../contracts/stationary.json"))?;
    let mut rows = BTreeMap::new();
    for kind in ["bright", "dark", "dip-six", "missing-six", "weak-H1"] {
        let x = stimulus(48000, 196., kind);
        let actual = updates(&x, 48000, 196.)?;
        let frozen: Updates = serde_json::from_value(contract["schedules"][kind][1].clone())?;
        ensure(
            frozen.len() == actual.len(),
            &format!(
                "capture {kind} update count {} expected {}",
                actual.len(),
                frozen.len()
            ),
        )?;
        let mut worst = 0f64;
        for ((tick, g), (old, w)) in actual.iter().zip(frozen) {
            ensure(*tick == old, "capture publication sample")?;
            for (a, b) in g.iter().zip(w) {
                let err = (a - b).abs();
                ensure(err <= 2e-11, "relative-order coefficient")?;
                worst = worst.max(err);
            }
        }
        rows.insert(
            kind,
            json!({"updates":actual,"max_coefficient_error":worst}),
        );
    }
    ensure(
        updates(&vec![0.; 16000], 48000, 196.)?.len() == 1,
        "unvoiced capture publication",
    )?;
    Ok(json!({"injected_sources":rows}))
}
