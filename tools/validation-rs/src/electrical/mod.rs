pub mod carrier;
pub mod oracle;
mod timing;
use crate::{Result, ensure, evidence, numerics, process, replay};
use rustfft::num_complex::Complex64 as C;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    f64::consts::{PI, SQRT_2, TAU},
    fs,
    path::Path,
    process::Command,
};

pub fn replay_taps(exe: &Path, fs: f64, x: &[f64], d: f64) -> Result<Vec<[f64; 8]>> {
    scheduled_taps(exe, fs, x, |_| d)
}

pub fn scheduled_taps(
    exe: &Path,
    fs: f64,
    x: &[f64],
    depth: impl Fn(usize) -> f64,
) -> Result<Vec<[f64; 8]>> {
    let mut data = Vec::with_capacity(x.len() * 16);
    for (i, v) in x.iter().enumerate() {
        data.extend(v.to_le_bytes());
        data.extend(depth(i).to_le_bytes());
    }
    let o = replay::stream(exe, &["--o2-taps".into(), fs.to_string()], &data)?;
    ensure(o.stdout.len() == 64 * x.len(), "electrical tap frame count")?;
    Ok(o.stdout
        .as_chunks::<64>()
        .0
        .iter()
        .map(|r| {
            std::array::from_fn(|i| f64::from_le_bytes(r[8 * i..8 * i + 8].try_into().unwrap()))
        })
        .collect())
}
fn compare(actual: &[[f64; 8]], expected: &[[f64; 3]], d: f64) -> Result<[f64; 4]> {
    ensure(actual.len() == expected.len(), "electrical frame count")?;
    let mut worst = [0.0_f64; 4];
    for (n, (row, &[h, a, l])) in actual.iter().zip(expected).enumerate() {
        for (j, wanted) in [h, a, l, (1. - 0.025 * d) * l + 0.025 * d * a]
            .into_iter()
            .enumerate()
        {
            let e = (row[j] - wanted).abs();
            if !(0.0..=1e-9).contains(&e) {
                return Err(format!(
                    "tick={n} tap={} expected={wanted} actual={}",
                    ["h", "a", "l", "q"][j],
                    row[j]
                )
                .into());
            }
            worst[j] = worst[j].max(e);
        }
    }
    Ok(worst)
}
fn projection(rows: &[[f64; 8]], fs: f64, f: f64) -> [C; 4] {
    let mut values = [C::ZERO; 4];
    for (i, row) in rows.iter().enumerate() {
        let t = i as f64 / fs;
        let phase = C::from_polar(1., -TAU * f * t);
        let q = (row[3] * SQRT_2 / 0.85) * phase;
        values[0] += q;
        values[1] += t * q;
        values[2] += row[1] * phase;
        values[3] += row[2] * phase;
    }
    values
}
fn rate(exe: &Path, fs: f64) -> Result<Value> {
    let coeffs = process::check(Command::new(exe).args(["--o2-coeff", &fs.to_string()]))?;
    let coeff: [f64; 6] = serde_json::from_value(evidence::strict(&coeffs.stdout)?)?;
    let k = (PI * 3200. / fs).tan();
    let den = 1. + SQRT_2 * k + k * k;
    let b = k * k / den;
    let expected = [
        (-TAU * 20. / fs).exp(),
        b,
        2. * b,
        b,
        2. * (k * k - 1.) / den,
        (1. - SQRT_2 * k + k * k) / den,
    ];
    ensure(
        coeff
            .iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() <= 1e-14)
            && coeff[2] == 2. * coeff[1]
            && coeff[1] == coeff[3],
        "prepared coefficients",
    )?;
    let radius = coeff[5].sqrt();
    ensure(
        coeff[4] * coeff[4] < 4. * coeff[5]
            && (0.0..1.0).contains(&radius)
            && (0.0..1.0).contains(&coeff[0]),
        "stable poles",
    )?;
    let n = ((0.5 * fs).ceil() as usize).next_power_of_two();
    let mut impulse = vec![0.; n];
    impulse[0] = 1.;
    let mut drives = vec![("impulse", impulse)];
    for name in ["constant", "step", "sine", "multitone"] {
        drives.push((
            name,
            (0..4096)
                .map(|i| {
                    let t = f64::from(i) / fs;
                    match name {
                        "constant" => -0.375,
                        "step" => f64::from(t > 0.01),
                        "sine" => (TAU * 196. * t).sin(),
                        _ => (TAU * 1318.51 * t).cos() + 0.45 * (TAU * 6151. * t).sin(),
                    }
                })
                .collect(),
        ));
    }
    let step = oracle::taps(fs, &vec![1.; (0.1 * fs).ceil() as usize]);
    let mut differential = BTreeMap::new();
    let mut responses = vec![];
    let mut steps = vec![];
    let mut cutoff = None;
    for (label, x) in drives {
        let expected = oracle::taps(fs, &x);
        for d in [0., 0.5, 1.] {
            let rows = replay_taps(exe, fs, &x, d)?;
            differential.insert(format!("{label}/{d}"), compare(&rows, &expected, d)?);
            if label != "impulse" {
                continue;
            }
            if d == 0. {
                let (mut lo, mut hi) = (1000., 6000.);
                for _ in 0..37 {
                    let mid = 0.5 * (lo + hi);
                    let [_, _, a, l] = projection(&rows, fs, mid);
                    if (l / a).norm() > 1. / SQRT_2 {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                let hz = (lo + hi) / 2.;
                ensure((hz - 3200.).abs() < 1e-6, "measured electrical cutoff")?;
                cutoff = Some(hz);
            }
            let q: Vec<_> = rows.iter().map(|r| r[3] * SQRT_2 / 0.85).collect();
            let mut max = 0.;
            let mut worst = None;
            for (i, v) in numerics::spectrum(&q).into_iter().enumerate() {
                let f = i as f64 * fs / n as f64;
                let error = (v - oracle::transfer(fs, f, d)).norm();
                ensure(error <= 1e-9, "electrical FFT complex transfer")?;
                if error > max || worst.is_none() {
                    max = error;
                    worst = Some([f, v.re, v.im]);
                }
            }
            let terminal = rows
                .last()
                .unwrap()
                .iter()
                .map(|v| v.abs())
                .fold(0., f64::max);
            ensure(terminal < 1e-10, "impulse terminal")?;
            let mut points = vec![];
            for f in [
                0.,
                20.,
                82.41,
                196.,
                440.,
                1000.,
                3200.,
                6000.,
                12000.,
                fs / 2.,
            ] {
                let [z, moment, a, l] = projection(&rows, fs, f);
                let blocked = oracle::transfer(fs, f, 40.) * 0.85 / SQRT_2;
                ensure(
                    (z - oracle::transfer(fs, f, d)).norm() <= 1e-9
                        && (a - blocked).norm() <= 1e-9
                        && (l - blocked * oracle::load(fs, f)).norm() <= 1e-9
                        && (z - (l + 0.025 * d * (a - l)) * SQRT_2 / 0.85).norm() <= 1e-9,
                    "electrical complex probe",
                )?;
                let magnitude = (f != 0.).then(|| (l / a).norm());
                if let Some(m) = magnitude {
                    ensure(
                        (m - oracle::load(fs, f).norm()).abs() <= 1e-9,
                        "load magnitude",
                    )?;
                }
                let eligible = f != 0. && (d != 0. || f != fs / 2.);
                let delay = eligible.then(|| (moment / z).re);
                let delta = delay.map(|v| {
                    let ref_delay = -(oracle::transfer(fs, f + 0.001, d)
                        / oracle::transfer(fs, f - 0.001, d))
                    .arg()
                        / (0.002 * TAU);
                    (v - ref_delay).abs()
                });
                if let Some(error) = delta {
                    ensure(error < 1e-8, "group delay")?;
                }
                points.push(json!({"hz":f,"real":z.re,"imag":z.im,"a":[a.re,a.im],"l":[l.re,l.im],"time_moment":[moment.re,moment.im],"load_magnitude":magnitude,"phase_deg":eligible.then(||z.arg().to_degrees()),"delay_s":delay,"delay_error_s":delta}));
            }
            responses.push(json!({"depth":d,"points":points,"max_complex_error":max,"worst_bin":worst,"terminal":terminal}));
            let o = process::check(Command::new(exe).args([
                "--o2-step",
                &fs.to_string(),
                &d.to_string(),
            ]))?;
            let values: Vec<f64> = std::str::from_utf8(&o.stdout)?
                .split_whitespace()
                .map(str::parse)
                .collect::<std::result::Result<_, _>>()?;
            ensure(
                values.len() == 8
                    && values.iter().all(|v| v.is_finite())
                    && values[0] == (60. * fs).ceil()
                    && values[4] == 1.
                    && values[5..].iter().all(|v| v.abs() < 1e-10)
                    && (0.0..1e-10).contains(&values[3]),
                "long step tail",
            )?;
            let blended = |r: &[f64; 3]| r[2] + 0.025 * d * (r[1] - r[2]);
            let peak = step.iter().map(|r| blended(r).abs()).fold(0., f64::max);
            ensure(
                (values[1] - blended(&step[0])).abs() <= 1e-9 && (values[2] - peak).abs() <= 1e-9,
                "long step first/peak",
            )?;
            steps.push(json!({"depth":d,"frames":values[0],"first":values[1],"peak":values[2],"max_after_one_second":values[3],"terminal":&values[4..]}));
        }
    }
    let life = process::check(Command::new(exe).args(["--o2-lifecycle", &fs.to_string()]))?;
    Ok(
        json!({"rate":fs,"coefficients":coeff,"poles":[radius,radius],"impulse_frames":n,"cutoff_hz":cutoff,"response":responses,"long_step":steps,"differential":differential,"lifecycle":String::from_utf8(life.stdout)?}),
    )
}
pub fn bench(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let mut rows = Vec::new();
    for fs in [
        32000.,
        32000.0f64.next_up(),
        44100.,
        44150.5,
        48000.,
        48100.,
        88200.,
        96000.,
        176400.,
        192000.0f64.next_down(),
        192000.,
    ] {
        rows.push(rate(&exe, fs).map_err(|e| format!("rate {fs}: {e}"))?);
    }
    Ok(json!({"rates":rows,"producer_sha256":evidence::digest(&exe)?}))
}
pub fn timing(out: &Path) -> Result<Value> {
    timing::run(out)
}
#[derive(serde::Deserialize)]
struct Mutants {
    #[serde(rename = "FILTERS")]
    filters: BTreeMap<String, [String; 3]>,
    #[serde(rename = "LIFECYCLE")]
    lifecycle: BTreeMap<String, [String; 4]>,
}
pub fn mutants(out: &Path) -> Result<Value> {
    let spec: Mutants = serde_json::from_str(include_str!("../../contracts/o2-mutations.json"))?;
    let mut rows = vec![];
    let mut x = [0.; 32];
    x[0] = 1.;
    let expected = oracle::taps(48000., &x);
    for (name, [site, edit, tap]) in spec.filters {
        let scratch = tempfile::tempdir_in(out)?;
        let exe = replay::build(scratch.path(), Some(("o2", &site, &edit)))?;
        let samples = replay_taps(&exe, 48000., &x, 0.5)?;
        let error = compare(&samples, &expected, 0.5)
            .err()
            .ok_or("escaped filter mutant")?
            .to_string();
        ensure(
            error.contains(&format!("tap={tap} ")),
            &format!("{name}: {error}"),
        )?;
        rows.push(json!({"name":name,"predicate":format!("tap={tap}"),"first_failure":error,"producer_sha256":evidence::digest(&exe)?}));
    }
    for (name, [module, site, edit, marker]) in spec.lifecycle {
        let scratch = tempfile::tempdir_in(out)?;
        let exe = replay::build(scratch.path(), Some((&module, &site, &edit)))?;
        let o = Command::new(&exe)
            .args(["--o2-lifecycle", "48000"])
            .output()?;
        let stderr = String::from_utf8(o.stderr)?;
        ensure(
            !o.status.success() && stderr.contains(&marker),
            &format!("{name}: expected {marker}"),
        )?;
        fs::write(out.join(format!("{name}.stderr")), stderr)?;
        rows.push(json!({"name":name,"predicate":marker,"exit":o.status.code(),"producer_sha256":evidence::digest(&exe)?}));
    }
    Ok(rows.into())
}
