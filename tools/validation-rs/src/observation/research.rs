//! Stage 04 acoustic observation alternatives and frozen research diagnostics.
use super::{Case, read};
use crate::{
    Result,
    electrical::{acoustic, carrier, oracle},
    ensure, evidence, guide, numerics, publication, replay,
};
use rustfft::{FftPlanner, num_complex::Complex64 as C};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    f64::consts::{PI, SQRT_2, TAU},
    fs,
    path::Path,
};
fn db(x: f64) -> f64 {
    20. * x.max(1e-12).log10()
}
fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|a| a * a).sum::<f64>() / x.len() as f64).sqrt()
}
fn mean(x: &[f64]) -> f64 {
    x.iter().sum::<f64>() / x.len() as f64
}
fn stddev(x: &[f64]) -> f64 {
    let m = mean(x);
    (x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}
fn biquad(x: &[f64], b: [f64; 3], a: [f64; 2]) -> Vec<f64> {
    let mut z = [0.; 2];
    x.iter()
        .map(|v| {
            let y = b[0].mul_add(*v, z[0]);
            z = [(b[1] * v + z[1]) - a[0] * y, b[2] * v - a[1] * y];
            y
        })
        .collect()
}
pub(super) fn bands(x: &[f64], fs: f64) -> [Vec<f64>; 4] {
    let [l, m, h] = [200., 800., 3000.].map(|f| {
        let g = (PI * f / fs).tan();
        let b = g / (1. + g);
        biquad(x, [b, b, 0.], [(g - 1.) / (g + 1.), 0.])
    });
    [
        l.clone(),
        m.iter().zip(&l).map(|(a, b)| a - b).collect(),
        h.iter().zip(&m).map(|(a, b)| a - b).collect(),
        x.iter().zip(h).map(|(a, b)| a - b).collect(),
    ]
}
fn legacy(x: &[f64], fs: f64) -> Vec<f64> {
    let mut sum = vec![0.; x.len()];
    for ((band, pair), (q, w)) in bands(x, fs)
        .iter()
        .zip([[102., 193.], [315., 650.], [1060., 1780.], [2820., 4100.]])
        .zip([(1.6, 1.6), (2.6, 2.), (4.5, 2.25), (5.5, 1.75)])
    {
        for (f, mix) in pair.into_iter().zip([0.55, 0.45]) {
            let theta = TAU * f / fs;
            let a = theta.sin() / (2. * q);
            let c = a / (1. + a);
            let mode = biquad(
                band,
                [c, 0., -c],
                [-2. * theta.cos() / (1. + a), (1. - a) / (1. + a)],
            );
            for (s, v) in sum.iter_mut().zip(mode) {
                *s += w * mix * v;
            }
        }
    }
    x.iter()
        .zip(sum)
        .map(|(a, b)| 0.85 * a + 0.15 * b)
        .collect()
}
fn pickup(raw: &[[f64; 30]], c: &Case) -> Result<Vec<f64>> {
    let fs = f64::from(c.rate);
    let p0 = fs / c.hz;
    let mut history = Vec::new();
    let mut spatial = Vec::new();
    for (i, r) in raw.iter().enumerate() {
        history.push(r[3]);
        let p = fs / super::frequency(c, i);
        let ray = |s| read(&history, s, p0, c.speed);
        ensure(
            (ray((1. - 0.127) * p) + r[0]).abs() <= 1e-12,
            "research signed nut return",
        )?;
        spatial.push(
            [0.162, 0.174, 0.186]
                .into_iter()
                .zip([1. / 6., 4. / 6., 1. / 6.])
                .map(|(s, w)| w * (ray((s - 0.127) * p / 2.) - ray((2. - s - 0.127) * p / 2.)))
                .sum::<f64>(),
        );
    }
    let dc = (-TAU * 20. / fs).exp();
    let blocked = biquad(&spatial, [dc, -dc, 0.], [-dc, 0.])
        .into_iter()
        .map(|v| 0.85 * v)
        .collect::<Vec<_>>();
    let g = (PI * 3200. / fs).tan();
    let den = 1. + SQRT_2 * g + g * g;
    let b = g * g / den;
    Ok(biquad(
        &blocked,
        [b, 2. * b, b],
        [2. * (g * g - 1.) / den, (1. - SQRT_2 * g + g * g) / den],
    ))
}
fn spectral(x: &[f64], fs: u32, f: f64) -> Value {
    let m = mean(x);
    let mut w = x
        .iter()
        .zip(numerics::hann(x.len()))
        .map(|(v, w)| (v - m) * w)
        .collect::<Vec<_>>();
    w.resize(8 * x.len(), 0.);
    let p = numerics::spectrum(&w);
    let (mut total, mut center, mut hf, mut residual) = (0., 0., 0., 0.);
    for (k, v) in p.iter().enumerate() {
        let hz = k as f64 * fs as f64 / w.len() as f64;
        let power = v.norm_sqr();
        center += hz * power;
        if k == 0 {
            continue;
        }
        total += power;
        if hz >= 6000. {
            hf += power;
        }
        let j = (hz / f).round_ties_even();
        if !(j >= 1. && j * f < fs as f64 / 2. && (hz - j * f).abs() <= 2f64.max(0.001 * j * f)) {
            residual += power;
        }
    }
    let voiced = stddev(x) > 1e-12;
    json!({"centroid_Hz":voiced.then(||center/total),"hf_fraction":voiced.then(||hf/total),"residual_fraction":voiced.then(||residual/total)})
}
fn stats(x: &[f64], raw: &[[f64; 30]], c: &Case, f: f64) -> Value {
    let fs = c.rate as usize;
    let tail = &x[x.len() - fs..];
    let h = acoustic::harmonics(tail, c.rate, f);
    let mut out = spectral(tail, c.rate, f);
    let peak = x.iter().map(|v| v.abs()).fold(0., f64::max);
    let p = &x[..fs / 100];
    let q = &x[..fs / 20];
    let edges = (x.len() - fs..x.len())
        .filter(|&t| t > 0 && raw[t][10] != 0. && raw[t - 1][10] == 0.)
        .collect::<Vec<_>>();
    let cycles = edges
        .windows(2)
        .map(|e| {
            std::array::from_fn::<_, 128, _>(|i| {
                let d = i as f64 * (e[1] - e[0]) as f64 / 128.;
                let a = e[0] + d as usize;
                x[a] + d.fract() * (x[a + 1] - x[a])
            })
        })
        .collect::<Vec<_>>();
    let mut cycle_residual = None;
    if !cycles.is_empty() && stddev(tail) > 1e-12 {
        let avg: [f64; 128] =
            std::array::from_fn(|j| cycles.iter().map(|w| w[j]).sum::<f64>() / cycles.len() as f64);
        let sigma = stddev(&avg);
        if sigma > 1e-12 {
            cycle_residual = Some(
                (cycles
                    .iter()
                    .flat_map(|w| w.iter().zip(avg).map(|(a, b)| (a - b).powi(2)))
                    .sum::<f64>()
                    / (128 * cycles.len()) as f64)
                    .sqrt()
                    / sigma,
            );
        }
    }
    let seven = (0..7)
        .map(|i| h["dBr"][i].as_f64())
        .collect::<Option<Vec<_>>>();
    let saw = seven.as_ref().map(|v| {
        rms(&v
            .iter()
            .enumerate()
            .map(|(j, v)| v + db((j + 1) as f64))
            .collect::<Vec<_>>())
    });
    let tilt = seven.map(|v| {
        let o = (1..=7).map(|i| f64::from(i).log2()).collect::<Vec<_>>();
        let m = mean(&o);
        let y = mean(&v);
        o.iter().zip(v).map(|(a, b)| (a - m) * (b - y)).sum::<f64>()
            / o.iter().map(|x| (x - m).powi(2)).sum::<f64>()
    });
    let values = json!({"first":x[0],"rms10":rms(p),"rms50":rms(q),"ac10":stddev(p),"ac50":stddev(q),"rms_steady":rms(tail),"ac_steady":stddev(tail),"peak":peak,"above_ceiling":x.iter().filter(|v|v.abs()>0.98).count(),"required_reduction_db":db(peak/0.98).max(0.),"clipped":x.iter().filter(|v|v.abs()>=1.).count(),"harmonics":h,"attack50":acoustic::harmonics(q,c.rate,f),"saw7_db":saw,"tilt_db_oct":tilt,"cycle_residual":cycle_residual});
    out.as_object_mut()
        .unwrap()
        .extend(values.as_object().unwrap().clone());
    if c.trajectory == "lift" {
        out["lift_windows"] = json!(
            [fs - fs / 20, fs, fs + fs / 10, 2 * fs].map(|n| acoustic::harmonics(
                &x[n..n + fs / 20],
                c.rate,
                f
            ))
        );
    }
    out
}
fn tap(z: C, d: f64) -> C {
    z.powu(d.floor() as u32) * (1. - d.fract() + d.fract() * z)
}
fn scatter(raw: &[[f64; 30]], fs: u32, f: f64) -> Value {
    let p = fs as f64 / 440.;
    let cols = [2, 3, 5, 6].map(|j| {
        numerics::partials(
            &raw[raw.len() - fs as usize..]
                .iter()
                .map(|r| r[j])
                .collect::<Vec<_>>(),
            fs.into(),
            f,
            10,
        )
    });
    let mut nut = Vec::new();
    let mut rb = Vec::new();
    let mut rp = Vec::new();
    for k in 0..10 {
        let z = C::from_polar(1., -TAU * (k + 1) as f64 * f / fs as f64);
        let hn = tap(z, (1. - 0.127) * p);
        let hb = 0.98 * (0.125 + 0.75 * z + 0.125 * z * z) * tap(z, 0.127 * p - 1.);
        nut.push(db((1. - hn).norm()));
        rb.push(((1. - hb * hn) * cols[0][k] - (1. - hn) * cols[3][k]).norm());
        rp.push((cols[2][k] - tap(z, 0.127 * p / 2.) * cols[0][k]).norm());
    }
    json!({"eighth_B_N_p_j_dBr":cols.each_ref().map(|v|db((v[7]/v[0]).norm())),"nut_numerator_7_8_9_dB":&nut[6..9],"return_residual":rms(&rb)/cols[0][0].norm(),"pickup_residual":rms(&rp)/cols[2][0].norm()})
}
pub fn run(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let cases = carrier::cases()?;
    let mut results = BTreeMap::new();
    let mut wav = BTreeMap::new();
    let mut planner = FftPlanner::new();
    for line in include_str!("../../../validation/fixtures/o1_sha256.tsv").lines() {
        let name = line
            .split_once('\t')
            .and_then(|(_, file)| file.strip_suffix(".f64le"))
            .ok_or("frozen observation case")?;
        let c = cases.get(name).ok_or("unknown observation case")?;
        let raw = carrier::render(&exe, c)?;
        let n = c.rate as usize * c.seconds as usize;
        let stem = name.split_once('_').unwrap().1.rsplit_once('_').unwrap().0;
        let nominal = &cases[&format!("steady_{stem}_3s")];
        let baseline = carrier::render(&exe, nominal)?;
        let fs = c.rate as usize;
        let f = guide::spec(
            &baseline[baseline.len() - fs..]
                .iter()
                .map(|r| r[5])
                .collect::<Vec<_>>(),
            c.hz,
            false,
            &mut planner,
        )?["hz"]
            .as_f64()
            .ok_or("nominal pitch")?;
        let m5 = raw.iter().map(|r| r[11]).collect::<Vec<_>>();
        let electrical = pickup(&raw, c)?;
        let s5 = legacy(&m5[..n], c.rate.into());
        let mut measurement = json!({});
        for (label, signal) in [("M5", &m5), ("S5", &s5), ("pickup", &electrical)] {
            let x = &signal[..n];
            let mut row = stats(x, &raw, c, f);
            if matches!(c.trajectory.as_str(), "vibrato" | "bend") && label != "S5" {
                row["tracking"] = guide::phase::auxiliary(c, signal)?;
            }
            if c.rate == 96000 && c.hz == 1318.51 && label != "S5" {
                let end = 32000 * c.seconds as usize;
                let resampled = acoustic::resample::window(x, fs, end - 32000..end);
                row["decimated_32k"] = spectral(&resampled, 32000, f);
            }
            if c.rate == 48000
                && [82.41, 196., 440.].contains(&c.hz)
                && name.ends_with("R1.8_V0.1_3s")
                && c.trajectory == "steady"
            {
                let filename = format!("{}-{label}.wav", c.hz);
                fs::write(
                    out.join(&filename),
                    super::float_wav(48000, &x.iter().map(|v| *v as f32).collect::<Vec<_>>()),
                )?;
                wav.insert(filename.clone(), evidence::digest(&out.join(filename))?);
            }
            measurement[label] = row;
        }
        if name == "steady_48000_440_R1.8_V0.1_3s" {
            measurement["scattering"] = scatter(&raw, c.rate, f);
        }
        results.insert(name.to_string(), measurement);
    }
    let mut impulse = vec![0.; 48000];
    impulse[0] = 1.;
    let response = numerics::spectrum(&legacy(&impulse, 48000.));
    results.insert(
        "legacy_response_db".into(),
        json!(
            (1..=7)
                .map(|i| db(response[440 * i].norm()))
                .collect::<Vec<_>>()
        ),
    );
    results.insert(
        "legacy_extrema_db".into(),
        json!([
            db(response[65..]
                .iter()
                .map(|z| z.norm())
                .fold(f64::INFINITY, f64::min)),
            db(response[65..].iter().map(|z| z.norm()).fold(0., f64::max))
        ]),
    );
    let response = |hz: f64| {
        let z = C::from_polar(1., -TAU * hz / 48000.);
        let p = 48000. / 440.;
        let spatial = [0.162, 0.174, 0.186]
            .into_iter()
            .zip([1. / 6., 4. / 6., 1. / 6.])
            .map(|(s, w)| w * (tap(z, (s - 0.127) * p / 2.) - tap(z, (2. - s - 0.127) * p / 2.)))
            .sum::<C>();
        spatial * oracle::transfer(48000., hz, 0.) * 0.85
    };
    results.insert(
        "pickup_group_delay_ms_at_orders_1_to_7".into(),
        json!(
            (1..=7)
                .map(|j| {
                    let f = 440. * j as f64;
                    -(response(f + 1.) / response(f - 1.)).arg() * 1000. / (2. * TAU)
                })
                .collect::<Vec<_>>()
        ),
    );
    ensure(
        results.len() == 18 && wav.len() == 9,
        "stage 04 observation inventory",
    )?;
    Ok(json!({"measurements":results,"wav":wav}))
}
pub fn compare(reference: &Path, root: &Path) -> Result<Value> {
    let historical = reference.join("checkout/build/stage-04");
    let old = evidence::strict(&fs::read(historical.join("observation.json"))?)?;
    let new = evidence::strict(&fs::read(publication::current_artifact(
        root,
        "research-observation",
        "evidence.json",
    )?)?)?;
    let mut errors = (0, 0., 0.);
    acoustic::compare(
        &old,
        &new["results"]["measurements"],
        "Stage 04",
        &mut errors,
    )?;
    for (k, h) in new["results"]["wav"].as_object().unwrap() {
        ensure(
            *h == evidence::digest(&historical.join(k))?,
            &format!("Stage 04 WAV {k}"),
        )?;
    }
    Ok(
        json!({"cases":15,"wavs":9,"fields":errors.0,"max_scaled_error":errors.1,"max_phase_error_deg":errors.2}),
    )
}
