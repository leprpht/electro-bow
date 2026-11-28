//! Exercise the analytical electrical and V17 graders on original defective signals.
use super::oracle;
use crate::{Result, ensure, identity::stationary, numerics};
use rustfft::num_complex::Complex64 as C;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    f64::consts::{PI, SQRT_2, TAU},
};
const FS: usize = 48000;

fn filtered(k: f64, blocked: &[f64], mut z: [f64; 2]) -> Vec<f64> {
    let d = 1. + SQRT_2 * k + k * k;
    let b = k * k / d;
    let a1 = 2. * (k * k - 1.) / d;
    let a2 = (1. - SQRT_2 * k + k * k) / d;
    blocked
        .iter()
        .map(|h| {
            let x = 0.85 * h / SQRT_2;
            let y = b * x + z[0];
            z = [2. * b * x - a1 * y + z[1], b * x - a2 * y];
            y
        })
        .collect()
}
fn transfer(x: &[f64], fs: f64) -> Result<()> {
    let spectrum = numerics::spectrum(x);
    ensure(
        spectrum.iter().enumerate().all(|(i, z)| {
            (*z * SQRT_2 / 0.85 - oracle::transfer(fs, i as f64 * fs / x.len() as f64, 0.)).norm()
                <= 1e-9
        }),
        "complex transfer",
    )
}
fn taps(h: f64, a: f64, l: f64, q: f64, common: f64) -> Result<()> {
    ensure(
        (a - 0.85 * h / SQRT_2).abs() <= 1e-9
            && (q - (l + 0.0125 * (a - l))).abs() <= 1e-9
            && (common - SQRT_2 * q).abs() <= 1e-9,
        "complex transfer",
    )
}
fn scored(f: f64, source: &[f64], reference: &[f64]) -> Result<Value> {
    let mut raw = vec![[0.; 30]; FS];
    for (r, v) in raw.iter_mut().zip(reference) {
        for j in [3, 11, 21, 22, 24, 25, 26, 27] {
            r[j] = *v;
        }
    }
    let wet = source.iter().map(|v| [*v; 4]).collect::<Vec<_>>();
    stationary::grade(&raw, &wet, FS as u32, f, 0)
}
pub fn run() -> Result<BTreeMap<String, Value>> {
    let mut reports = BTreeMap::new();
    let mut reject = |name: &str, expected: &str, result: Result<()>| -> Result<()> {
        let failed = result
            .err()
            .ok_or_else(|| format!("surviving {name}"))?
            .to_string();
        ensure(
            failed == expected,
            &format!("{name} first failure {failed}"),
        )?;
        reports.insert(name.to_string(), json!({"first":failed,"status":1}));
        Ok(())
    };
    let fs = FS as f64;
    let mut impulse = vec![0.; FS / 2];
    impulse[0] = 1.;
    let rows = oracle::taps(fs, &impulse);
    let h: Vec<_> = rows.iter().map(|v| v[0]).collect();
    let k = (PI * 3200. / fs).tan();
    let l = filtered(k, &h, [0.; 2]);
    transfer(&l, fs)?;
    for (name, samples, rate) in [
        ("no prewarp", filtered(PI * 3200. / fs, &h, [0.; 2]), fs),
        ("no DC blocker", filtered(k, &impulse, [0.; 2]), fs),
        ("old reset state", filtered(k, &h, [0.1, -0.1]), fs),
        ("single rate fixed cutoff", l.clone(), 32000.),
    ] {
        reject(name, "complex transfer", transfer(&samples, rate))?;
    }
    let a1 = 2. * (k * k - 1.) / (1. + SQRT_2 * k + k * k);
    let a2 = (1. - SQRT_2 * k + k * k) / (1. + SQRT_2 * k + k * k);
    let root = (a1 - (a1 * a1 + 4. * a2).sqrt()) / 2.;
    reject(
        "wrong denominator sign",
        "unstable poles",
        ensure(root.abs() < 1., "unstable poles"),
    )?;
    let unstable = C::ONE / (C::ONE - 1.001 * C::from_polar(1., -0.1));
    reject(
        "unstable pole",
        "complex transfer",
        ensure(
            (unstable - oracle::transfer(fs, 0.1 * fs / TAU, 0.)).norm() <= 1e-9,
            "complex transfer",
        ),
    )?;
    let q: Vec<_> = rows.iter().map(|[_, a, l]| l + 0.0125 * (a - l)).collect();
    let peak = q.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    for name in [
        "gain after filter input",
        "duplicate conversion",
        "duplicate trim",
        "excess detail",
        "per-render normalization",
    ] {
        let verdict = rows.iter().zip(&q).try_for_each(|(&[h, a, l], &q)| {
            taps(
                h,
                match name {
                    "gain after filter input" => h,
                    "duplicate conversion" => a * 0.85,
                    "duplicate trim" => a / SQRT_2,
                    _ => a,
                },
                l,
                if name == "excess detail" {
                    l + 0.1 * (a - l)
                } else {
                    q
                },
                if name == "per-render normalization" {
                    q / peak
                } else {
                    SQRT_2 * q
                },
            )
        });
        reject(name, "complex transfer", verdict)?;
    }
    let silent = oracle::taps(fs, &[0.; 256]);
    ensure(silent.iter().flatten().all(|v| *v == 0.), "exact silence")?;
    reject(
        "silent offset",
        "silence feedthrough",
        ensure(
            silent.iter().all(|v| v[2] + 1e-16 == 0.),
            "silence feedthrough",
        ),
    )?;
    let base = |f: f64| {
        (0..FS)
            .map(|i| (TAU * f * i as f64 / fs).sin())
            .collect::<Vec<_>>()
    };
    let m5 = base(196.);
    let zero = vec![0.; FS];
    let silence = scored(196., &zero, &zero)?;
    ensure(
        silence["final_over_M5"].is_null() && silence["spectral"].is_null(),
        "unavailable silent metrics",
    )?;
    reject(
        "zero RMS denominator",
        "ineligible RMS denominator",
        ensure(
            silence["passes"]["level"] == true,
            "ineligible RMS denominator",
        ),
    )?;
    let doubled = m5.iter().map(|v| 2. * v).collect::<Vec<_>>();
    let level = scored(196., &doubled, &m5)?["final_over_M5"]
        .as_f64()
        .ok_or("double wet level")?;
    reject(
        "double wet level",
        "original M5 level",
        ensure(level.abs() <= 3., "original M5 level"),
    )?;
    let self_db = scored(196., &doubled, &doubled)?["final_over_M5"]
        .as_f64()
        .ok_or("self level")?;
    reject(
        "wrong denominator self-reference",
        "complex transfer",
        ensure((level - self_db).abs() <= 1e-9, "complex transfer"),
    )?;
    let upper = m5
        .iter()
        .zip(base(392.))
        .map(|(v, w)| 0.5 * v + 2. * w)
        .collect::<Vec<_>>();
    let nyquist = m5
        .iter()
        .enumerate()
        .map(|(i, v)| 0.5 * v + if i % 2 == 0 { 0.6 } else { -0.6 })
        .collect::<Vec<_>>();
    for (name, report) in [
        ("dominant H2", scored(196., &upper, &m5)?),
        ("dominant Nyquist", scored(196., &nyquist, &m5)?),
        (
            "15.04 cents flat",
            scored(440., &base(440. * 2f64.powf(-15.04 / 1200.)), &base(440.))?,
        ),
        ("invalid bridge anchor", scored(196., &m5, &upper)?),
    ] {
        if name == "dominant Nyquist" {
            ensure(report["strongest_hz"] == 24000., "Nyquist endpoint")?;
        }
        reject(
            name,
            "strongest fundamental",
            ensure(
                report["passes"]["fundamental"] == true,
                "strongest fundamental",
            ),
        )?;
    }
    Ok(reports)
}
