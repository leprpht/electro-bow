use super::Bank;
use crate::{
    Result,
    electrical::{acoustic, oracle, replay_taps, sweep},
    ensure, evidence, numerics, publication,
};
use rustfft::num_complex::Complex64 as C;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    f64::consts::{SQRT_2, TAU},
    fs,
    path::Path,
};
pub(super) fn transfer(rate: f64, pitch: f64, hz: f64, coeff: &[f64; 12]) -> C {
    let rho = (-TAU * 0.22 * pitch / rate).exp();
    let fa = rate / (rate / 16000.).floor();
    let z = C::from_polar(1., -TAU * hz / rate);
    let mut response = C::ONE;
    for (j, gain) in coeff.iter().enumerate() {
        let f = (j + 1) as f64 * pitch;
        let p = C::from_polar(rho, TAU * f / rate);
        let taper = ((0.28 * fa - f) / (0.03 * fa)).clamp(0., 1.);
        response += gain
            * taper
            * (1. - rho).powi(2)
            * ((C::ONE - p * z).powi(-2) + (C::ONE - p.conj() * z).powi(-2));
    }
    response
}
pub fn analytical(fs: f64, pitch: f64, g: &[f64; 12]) -> Result<Vec<[f64; 2]>> {
    let mut peaks = [[0., f64::NEG_INFINITY]; 3];
    for bin in 260..=16000 {
        let f = f64::from(bin) / 4.;
        let t = transfer(fs, pitch, f, g);
        for (j, peak) in peaks.iter_mut().enumerate() {
            let v = 20. * (t * oracle::transfer(fs, f, j as f64 / 2.)).norm().log10();
            if v > peak[1] {
                *peak = [f, v];
            }
        }
    }
    ensure(peaks.iter().all(|p| p[1] <= 6.), "stationary narrow peak")?;
    Ok(peaks.to_vec())
}
pub fn measured(exe: &Path, accepted: &BTreeMap<String, Value>) -> Result<Value> {
    let fs = 48000.;
    let x = sweep::chirp(fs);
    let denominator = numerics::spectrum(&x);
    let spacing = 1. / (x.len() as f64 * (1. / fs));
    let mut results = BTreeMap::new();
    for (j, label) in ["0", "0.5", "1"].into_iter().enumerate() {
        let depth = j as f64 / 2.;
        let q = replay_taps(exe, fs, &x, depth)?
            .iter()
            .map(|r| r[3])
            .collect::<Vec<_>>();
        for name in ["weak-H1", "upper", "lower", "inversion", "opposed"] {
            let coeff: [f64; 12] = serde_json::from_value(
                accepted[&format!("48000/196/0.1/1.8/{name}")]["accepted_dB"].clone(),
            )?;
            let g = coeff.map(|db| (std::f64::consts::LN_10 * db / 20.).exp_m1());
            let mut bank = Bank::new(fs)?;
            let mut input = [0.; 17];
            input[0] = 196.;
            input[5..].copy_from_slice(&g);
            let mut y = Vec::with_capacity(q.len());
            for v in &q {
                input[1] = *v;
                y.push(v + bank.apply(&input)?[0]);
            }
            let values = numerics::spectrum(&y);
            let mut max = 0f64;
            let mut bins = Vec::new();
            for (k, den) in denominator.iter().enumerate() {
                let f = k as f64 * spacing;
                if !(65. ..=4000.).contains(&f) || den.norm() / x.len() as f64 <= 1e-12 {
                    continue;
                }
                let value = values[k] * SQRT_2 / (0.85 * den);
                max = max.max(
                    (value - transfer(fs, 196., f, &g) * oracle::transfer(fs, f, depth)).norm(),
                );
                bins.push((f, value.norm()));
            }
            let peak = sweep::peaks(&bins, spacing)
                .into_iter()
                .max_by(|a, b| {
                    a["dB"]
                        .as_f64()
                        .unwrap()
                        .total_cmp(&b["dB"].as_f64().unwrap())
                })
                .ok_or("missing peak")?;
            ensure(
                peak["dB"].as_f64().is_some_and(|v| v <= 6.) && max <= 1e-9,
                "measured frozen I1 sweep",
            )?;
            results.insert(
                format!("{name}/{label}"),
                json!({"peak":peak,"complex_error":max}),
            );
        }
    }
    Ok(json!(results))
}
pub fn expression(rows: &BTreeMap<String, Value>) -> Result<Value> {
    let num = |v: &Value| v.as_f64().ok_or("missing finite expression measurement");
    let mut pressure = Vec::new();
    let mut speed = Vec::new();
    for field in ["neutral", "depths"] {
        let row = |s: &str, r: &str| &rows[&format!("48000/440/{s}/{r}/opposed")][field][0];
        pressure.push(
            num(&row("0.1", "2.4")["spectral"]["high_fraction"])?
                - num(&row("0.1", "1.2")["spectral"]["high_fraction"])?,
        );
        speed.push(
            20. * (num(&row("0.16", "1.8")["rms_FS"])? / num(&row("0.06", "1.8")["rms_FS"])?)
                .log10(),
        );
    }
    let spread: [f64; 3] = std::array::from_fn(|j| {
        let v = rows
            .iter()
            .filter(|(k, _)| k.contains("/1318.51/"))
            .map(|(_, r)| 20. * r["depths"][j]["rms_FS"].as_f64().unwrap().log10())
            .collect::<Vec<_>>();
        v.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - v.iter().copied().fold(f64::INFINITY, f64::min)
    });
    let mut relative = Vec::new();
    for kind in ["bright", "dark"] {
        let h: [f64; 12] = serde_json::from_value(
            rows[&format!("48000/196/0.1/1.8/{kind}")]["depths"][0]["harmonics"]["peak_FS"].clone(),
        )?;
        let max = h.iter().copied().fold(0., f64::max);
        let h = h.map(|v| (20. * (v / max).log10()).max(-50.));
        let mean = h.iter().sum::<f64>() / 12.;
        relative.push(h.map(|v| v - mean));
    }
    let distance = (relative[0]
        .iter()
        .zip(relative[1])
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        / 12.)
        .sqrt();
    ensure(
        pressure[0] * pressure[1] > 0. && pressure[1].abs() >= 0.5 * pressure[0].abs(),
        "pressure contrast",
    )?;
    ensure(speed[1] >= 0.75 * speed[0], "speed contrast")?;
    ensure(spread.iter().all(|v| *v < 3.), "high-note spread")?;
    ensure(distance >= 1., "source transfer")?;
    Ok(
        json!({"pressure":pressure,"speed_dB":speed,"high_note_spread_dB":spread,"synthetic_distance_dB":distance}),
    )
}
pub fn differential(reference: &Path, root: &Path) -> Result<Value> {
    let old = evidence::strict(&fs::read(
        reference.join("checkout/build/i1/stationary.json"),
    )?)?;
    let new = evidence::strict(&fs::read(publication::current_artifact(
        root,
        "i1-stationary",
        "evidence.json",
    )?)?)?;
    let actual = &new["results"];
    let mut stats = (0, 0., 0.);
    for k in ["expression", "sweep"] {
        acoustic::compare(&old[k], &actual[k], k, &mut stats)?;
    }
    for (name, r) in old["rows"].as_object().ok_or("stationary rows")? {
        let a = &actual["stationary"][name];
        let mut mapped = a.clone();
        for (i, k) in ["wet", "delta", "scale"].iter().enumerate() {
            mapped[k] = a["hashes"][i].clone();
        }
        for (k, v) in r.as_object().unwrap() {
            acoustic::compare(v, &mapped[k], &format!("{name}/{k}"), &mut stats)?;
        }
    }
    Ok(
        json!({"cases":36,"fields":stats.0,"largest_scaled_error":stats.1,"largest_phase_difference_degrees":stats.2}),
    )
}
