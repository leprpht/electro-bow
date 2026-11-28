use super::{Case, Result, TAU, ensure, mean, samples};
use nalgebra::{Matrix3, Vector3};
use rustfft::{FftPlanner, num_complex::Complex64};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
#[derive(Deserialize)]
struct Filter {
    sos: [[f64; 6]; 3],
    initial: [[f64; 2]; 3],
}
fn filter(data: &mut [f64], f: &Filter) {
    let initial = data[0];
    for (s, zi) in f.sos.iter().zip(f.initial) {
        let [mut a, mut b] = zi.map(|v| v * initial);
        for v in &mut *data {
            let x = *v;
            let y = s[0] * x + a;
            a = s[1] * x - s[4] * y + b;
            b = s[2] * x - s[5] * y;
            *v = y;
        }
    }
}
fn analytic_phase(signal: &[f64], f: &Filter) -> Vec<f64> {
    // Three second-order sections, odd extension, SciPy's default 21-sample edge.
    let n = signal.len();
    let pad = 21;
    let mut extended: Vec<_> = (1..=pad)
        .rev()
        .map(|i| 2. * signal[0] - signal[i])
        .chain(signal.iter().copied())
        .chain((1..=pad).map(|i| 2. * signal[n - 1] - signal[n - 1 - i]))
        .collect();
    filter(&mut extended, f);
    extended.reverse();
    filter(&mut extended, f);
    extended.reverse();
    let mut buf: Vec<_> = extended[pad..pad + n]
        .iter()
        .map(|v| Complex64::new(*v, 0.))
        .collect();
    let mut planner = FftPlanner::new();
    planner.plan_fft_forward(n).process(&mut buf);
    for (k, v) in buf.iter_mut().enumerate().skip(1) {
        if k < n.div_ceil(2) {
            *v *= 2.;
        } else if k > n / 2 {
            *v = Complex64::ZERO;
        }
    }
    planner.plan_fft_inverse(n).process(&mut buf);
    let mut prev = buf[0].arg();
    let mut offset = 0.;
    buf.iter()
        .map(|z| {
            let a = z.arg();
            let d = a - prev;
            if d > std::f64::consts::PI {
                offset -= TAU;
            } else if d < -std::f64::consts::PI {
                offset += TAU;
            }
            prev = a;
            a + offset
        })
        .collect()
}
pub fn grade(c: &Case, frames: &[[u8; 96]], out: &Path, name: &str) -> Result<Value> {
    let filters: BTreeMap<String, Filter> =
        serde_json::from_str(include_str!("../../contracts/guide-phase-filter.json"))?;
    let phase = analytic_phase(&samples(frames, 5), &filters[&c.hz.to_string()]);
    let fs = c.rate as f64;
    let n = c.rate as usize;
    let h = (0.006 * fs).round_ties_even() as usize;
    let bend = c.trajectory == "bend";
    let end = if bend { (1.45 * fs) as usize } else { 3 * n };
    let t: Vec<_> = (n..end).map(|i| i as f64 / fs).collect();
    let obs: Vec<_> = (n..end)
        .map(|i| {
            1200. * ((phase[i + h] - phase[i - h]) * (fs / (2. * h as f64 * TAU * c.hz))).log2()
        })
        .collect();
    ensure(
        obs.iter().all(|v| v.is_finite()),
        "physical instantaneous frequency",
    )?;
    let truth = |t: f64| {
        if bend {
            200. * (1. - (t - 1.2).abs() / 0.2).max(0.)
        } else {
            35. * (TAU * 5. * t).sin()
        }
    };
    let center = mean(&obs);
    let mut best = (f64::NEG_INFINITY, 0);
    for ms in 0..=45 {
        let expected: Vec<_> = t
            .iter()
            .map(|ti| truth(ti - f64::from(ms) / 1000.))
            .collect();
        let m = mean(&expected);
        let cross = obs
            .iter()
            .zip(&expected)
            .map(|(a, b)| (a - center) * (b - m))
            .sum::<f64>();
        let norm = expected.iter().map(|v| (v - m).powi(2)).sum::<f64>().sqrt();
        let corr = cross / norm;
        if corr > best.0 {
            best = (corr, ms);
        }
    }
    let lag = best.1;
    let mut aligned = 0.;
    let mut unaligned = 0.;
    let mut max_error = 0f64;
    let mut file = BufWriter::new(File::create(out.join(format!("{name}-phase.f64le")))?);
    for (&ti, &v) in t.iter().zip(&obs) {
        let target = truth(ti);
        let delayed = truth(ti - f64::from(lag) / 1000.);
        aligned += (v - delayed).powi(2);
        unaligned += (v - target).powi(2);
        max_error = max_error.max((v - delayed).abs());
        for word in [ti, v, target, delayed] {
            file.write_all(&word.to_le_bytes())?;
        }
    }
    file.flush()?;
    aligned = (aligned / t.len() as f64).sqrt();
    unaligned = (unaligned / t.len() as f64).sqrt();
    ensure(
        lag <= 40 && aligned <= 15. && max_error < 600.,
        "guide bend/vibrato lag and error",
    )?;
    let step = (0.012 * fs).round_ties_even() as usize;
    let slew: Vec<_> = obs[step..]
        .iter()
        .zip(&obs)
        .map(|(a, b)| (a - b) / (step as f64 / fs))
        .collect();
    let bounds = |v: &[f64]| {
        (
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let (low, high) = bounds(&obs);
    let (slow, fast) = bounds(&slew);
    let mut row = json!({"lag_ms":lag,"unaligned_cents":unaligned,"aligned_cents":aligned,"max_error_cents":max_error,"min_hz":c.hz*2f64.powf(low/1200.),"max_hz":c.hz*2f64.powf(high/1200.),"min_slew_cents_s":slow,"max_slew_cents_s":fast});
    for (col, k) in [(2, "B"), (3, "N"), (5, "pickup")] {
        let v = samples(&frames[..3 * n], col);
        row["max_sample_step"][k] = v
            .windows(2)
            .map(|p| (p[1] - p[0]).abs())
            .fold(0., f64::max)
            .into();
    }
    if !bend {
        let mut best = (f64::INFINITY, 0., 0.);
        for i in 300..=800 {
            let freq = f64::from(i) / 100.;
            let mut gram = Matrix3::zeros();
            let mut rhs = Vector3::zeros();
            for (&ti, &v) in t.iter().zip(&obs) {
                let (sn, cs) = (TAU * freq * ti).sin_cos();
                let z = Vector3::new(1., cs, sn);
                gram += z * z.transpose();
                rhs += z * v;
            }
            let fit = gram
                .cholesky()
                .ok_or("degenerate modulation fit")?
                .solve(&rhs);
            // Residual sum of squares differs from -fit' rhs by the same constant at every frequency.
            let loss = -fit.dot(&rhs);
            if loss < best.0 {
                best = (loss, freq, fit[1].hypot(fit[2]));
            }
        }
        ensure(
            (best.1 - 5.).abs() <= 0.2
                && (25. ..=42.).contains(&best.2)
                && aligned.max(unaligned) <= 6.,
            "physical vibrato rate and depth",
        )?;
        row["modulation_hz"] = best.1.into();
        row["depth_cents"] = best.2.into();
    }
    Ok(row)
}
