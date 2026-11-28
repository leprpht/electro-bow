use super::{oracle, replay_taps};
use crate::{Result, ensure, evidence, numerics, replay};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    f64::consts::{SQRT_2, TAU},
    path::Path,
};
fn sha(x: impl Iterator<Item = f64>) -> String {
    let mut h = Sha256::new();
    for v in x {
        h.update(v.to_le_bytes());
    }
    format!("{:x}", h.finalize())
}
fn peaks(bins: &[(f64, f64)], spacing: f64) -> Vec<Value> {
    let mut result = vec![];
    let mut start = 0;
    for stop in 1..=bins.len() {
        if stop < bins.len() && bins[stop].0 - bins[stop - 1].0 < 1.5 * spacing {
            continue;
        }
        let band = &bins[start..stop];
        let mut k = 0;
        while k < band.len() {
            let mut end = k;
            while end + 1 < band.len() && band[end + 1].1 == band[k].1 {
                end += 1;
            }
            let mut maxima = Vec::new();
            if k == 0 {
                if end > 0 || band.len() == 1 || band[0].1 >= band[1].1 {
                    maxima.push(0);
                }
            } else if end + 1 < band.len()
                && band[k].1 > band[k - 1].1
                && band[end].1 > band[end + 1].1
            {
                maxima.push((k + end) / 2);
            }
            if end + 1 == band.len() && end != 0 && band[end].1 >= band[end - 1].1 {
                maxima.push(end);
            }
            for i in maxima {
                let threshold = band[i].1 * 10f64.powf(-3. / 20.);
                let left = (0..i).rev().find(|j| band[*j].1 < threshold);
                let right = (i + 1..band.len()).find(|j| band[*j].1 < threshold);
                let cross = |a: usize, b: usize| {
                    band[a].0
                        + (threshold - band[a].1) / (band[b].1 - band[a].1)
                            * (band[b].0 - band[a].0)
                };
                let width = left
                    .zip(right)
                    .map(|(l, r)| cross(r, r - 1) - cross(l, l + 1));
                result.push(
                    json!({"hz":band[i].0,"dB":20.*band[i].1.max(1e-12).log10(),"width_hz":width}),
                );
            }
            k = end + 1;
        }
        start = stop;
    }
    result
}
pub fn run(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let fs = 48000.;
    let factor = 4000f64 / 65.;
    let beta = 3. / factor.ln();
    let mut x: Vec<_> = (0..144000)
        .map(|i| 0.1 * (TAU * beta * 65. * (factor.powf((i as f64 / fs) / 3.) - 1.)).cos())
        .collect();
    x.resize(168000, 0.);
    let excitation = sha(x.iter().copied());
    ensure(
        excitation == "1d5b2ef3816e77f36559ecd0d54d81b60e07371c459bc73a5d8434055c110bd3",
        &format!("chirp input {excitation}"),
    )?;
    let den = numerics::spectrum(&x);
    let spacing = 1. / (x.len() as f64 * (1. / fs));
    let bins: Vec<_> = (0..den.len())
        .filter(|i| {
            (65. ..=4000.).contains(&(*i as f64 * spacing))
                && den[*i].norm() / x.len() as f64 > 1e-12
        })
        .collect();
    let bin_hash = sha(bins.iter().map(|i| *i as f64));
    ensure(
        bin_hash == "ac284ee78ced4166c2c20051fabd211428f400fcf168b34056c4cc9cf0826f6d",
        "sweep eligible bins",
    )?;
    let mut rows = vec![];
    for (d, raw_ref, peak_ref) in [
        (
            0.,
            "24af1f4a72a6ee729497b77b0fb5cd4c16651169ab4d4760b60eaf0c3360a7ea",
            530.,
        ),
        (
            0.5,
            "827a476814468974db61891de203f50a194e52f7b0ace152f601224dbe1a64d7",
            493.4285714285714,
        ),
        (
            1.,
            "24c66348c2db692f0b107d9110b5c80fc7d34291bb2b185ca6687508c0ec6a5a",
            465.1428571428571,
        ),
    ] {
        let q: Vec<_> = replay_taps(&exe, fs, &x, d)?.iter().map(|r| r[3]).collect();
        let hash = sha(q.iter().copied());
        ensure(hash == raw_ref, "sweep actual returned q hash")?;
        let fft = numerics::spectrum(&q);
        let mut max = 0f64;
        let mut h = Sha256::new();
        let mut curve = vec![];
        for &k in &bins {
            let freq = k as f64 * spacing;
            let measured = (fft[k] * SQRT_2) / (0.85 * den[k]);
            let err = (measured - oracle::transfer(fs, freq, d)).norm();
            ensure(err <= 1e-9, "sweep complex transfer")?;
            max = max.max(err);
            h.update(measured.re.to_le_bytes());
            h.update(measured.im.to_le_bytes());
            curve.push((freq, measured.norm()));
        }
        let peaks = peaks(&curve, spacing);
        ensure(
            peaks.len() == 1
                && (peaks[0]["hz"].as_f64().unwrap() - peak_ref).abs() < 1e-12
                && peaks[0]["width_hz"].is_null()
                && peaks[0]["dB"].as_f64().is_some_and(|v| v <= 6.),
            "sweep peak and width",
        )?;
        rows.push(json!({"rate":fs,"depth":d,"interval":[0,x.len()],"tap":"--o2-taps/q[3]","excitation":excitation,"returned":hash,"bins":bin_hash,"quotient":format!("{:x}",h.finalize()),"peaks":peaks,"complex_error":max}));
    }
    Ok(json!({"rows":rows,"producer":evidence::digest(&exe)?}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edge_and_split_bands() {
        let p = peaks(&[(1., 2.), (2., 1.), (3., 3.), (4., 1.), (8., 1.)], 1.);
        assert_eq!(p.len(), 3);
        assert!(p[0]["width_hz"].is_null());
        assert!(p[1]["width_hz"].as_f64().unwrap() > 0.);
        assert!(p[2]["width_hz"].is_null());
    }
}
