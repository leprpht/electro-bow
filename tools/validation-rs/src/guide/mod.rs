//! Grade prescribed waveguide motion at the existing 12-column physical boundary.
mod phase;
use crate::{
    Result, ensure, evidence, numerics,
    observation::{self, Case},
    replay,
};
use rustfft::{FftPlanner, num_complex::Complex64};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::TAU,
    fs,
    path::Path,
};

fn mean(x: &[f64]) -> f64 {
    x.iter().sum::<f64>() / x.len() as f64
}
fn stddev(x: &[f64]) -> f64 {
    let m = mean(x);
    (x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}
fn db(x: f64) -> f64 {
    20. * x.max(1e-12).log10()
}
fn spec(x: &[f64], target: f64, orders: bool, planner: &mut FftPlanner<f64>) -> Result<Value> {
    let n = x.len();
    let center = mean(x);
    let mut buf = vec![Complex64::ZERO; 8 * n];
    for ((b, v), w) in buf.iter_mut().zip(x).zip(numerics::hann(n)) {
        b.re = (v - center) * w;
    }
    planner.plan_fft_forward(8 * n).process(&mut buf);
    let a: Vec<_> = buf[..=4 * n].iter().map(|z| z.norm()).collect();
    let mut peaks = vec![];
    let mut i = 1;
    while i + 1 < a.len() {
        if a[i - 1] < a[i] {
            let mut end = i;
            while end + 1 < a.len() && a[end + 1] == a[i] {
                end += 1;
            }
            if end + 1 < a.len() && a[end + 1] < a[i] {
                peaks.push((i + end) / 2);
            }
            i = end;
        }
        i += 1;
    }
    let mut peak = *peaks
        .first()
        .ok_or("physical spectrum lacks local maximum")?;
    for &idx in &peaks {
        if a[idx] > a[peak] {
            peak = idx;
        }
    }
    let [l, m, r] = [a[peak - 1], a[peak], a[peak + 1]].map(f64::ln);
    let f = (peak as f64 + 0.5 * (l - r) / (l - 2. * m + r)) / 8.;
    ensure(f.is_finite() && f > 0., "physical peak frequency")?;
    let mut total = 0.;
    let mut admitted = 0.;
    let mut bright = 0.;
    let mut by_order = vec![0.];
    for (i, &amp) in a.iter().enumerate().skip(1) {
        let hz = i as f64 / 8.;
        let h = (hz / f).round_ties_even();
        let p = amp * amp;
        total += p;
        if (12000..=48000).contains(&i) {
            bright += p;
        }
        if h >= 1. && h * f < (n as f64 / 2.) && (hz - h * f).abs() <= 2f64.max(0.001 * h * f) {
            admitted += p;
            if orders {
                by_order.resize(by_order.len().max(h as usize + 1), 0.);
                by_order[h as usize] += p;
            }
        }
    }
    let upper: BTreeSet<_> = peaks
        .iter()
        .filter_map(|idx| {
            let hz = *idx as f64 / 8.;
            let h = (hz / target).round_ties_even();
            (h >= 2.
                && (hz - h * target).abs() <= 2f64.max(h * target * (2f64.powf(15. / 1200.) - 1.))
                && a[*idx] > a[peak] * 10f64.powf(-36. / 20.))
            .then_some(h as usize)
        })
        .collect();
    let mut stats = json!({"hz":f,"cents":1200.*(f/target).log2(),"eta":admitted/total,"upper_partials":upper,"rms":stddev(x),"bright_fraction":bright/total});
    if orders {
        stats["power_by_harmonic"] =
            json!(by_order[1..].iter().map(|v| v / total).collect::<Vec<_>>());
    }
    Ok(stats)
}
fn value(row: &Value, name: &str) -> f64 {
    row[name]
        .as_f64()
        .expect("internal finite physical statistic")
}
fn samples(frames: &[[u8; 96]], column: usize) -> Vec<f64> {
    frames
        .iter()
        .map(|f| f64::from_le_bytes(f[column * 8..column * 8 + 8].try_into().unwrap()))
        .collect()
}
fn stationary(c: &Case, frames: &[[u8; 96]], planner: &mut FftPlanner<f64>) -> Result<Value> {
    let n = c.rate as usize;
    let start = frames.len() - n;
    let before = &frames[start - n..start];
    let last = &frames[start..];
    let mut row = json!({});
    for (name, col) in [("bridge", 2), ("pickup", 5)] {
        let x = samples(last, col);
        let mut s = spec(&x, c.hz, c.rate == 48000 && c.hz == 440., planner)?;
        let drift = db(value(&s, "rms") / stddev(&samples(before, col)));
        ensure(
            value(&s, "cents").abs() <= 12. && value(&s, "eta") >= 0.90,
            "physical stationary cents/harmonic energy",
        )?;
        ensure(
            value(&s, "rms") > 0.04 && drift.abs() <= 2.,
            "physical stationary sustain",
        )?;
        ensure(
            c.hz > 880. || s["upper_partials"].as_array().unwrap().len() >= 3,
            "physical upper partials",
        )?;
        s["drift_db"] = drift.into();
        row[name] = s;
    }
    let branch = samples(&frames[start - 1..], 10);
    let onsets: Vec<_> = branch
        .windows(2)
        .enumerate()
        .filter(|(_, b)| b[0] == 0. && b[1] != 0.)
        .map(|(i, _)| i)
        .collect();
    let duty = branch[1..].iter().filter(|v| **v != 0.).count() as f64 / n as f64;
    let ratio = c.k * 1.65 / c.speed;
    if c.rate == 48000 && c.hz == 196. && (ratio - 1.8).abs() < 1e-12 {
        ensure(
            (0.95..=1.05).contains(&(onsets.len() as f64 / 196.)) && (duty - 0.127).abs() <= 0.05,
            "physical stick/slip",
        )?;
    }
    let force = samples(frames, 8);
    ensure(
        force
            .iter()
            .all(|v| v.abs() <= 2. * ratio * c.speed + 2e-10),
        "physical static force limit",
    )?;
    let x = samples(last, 5);
    let mu = mean(&x);
    let weighted: Vec<_> = x
        .iter()
        .zip(numerics::hann(n))
        .map(|(v, w)| (v - mu) * w)
        .collect();
    let p: Vec<_> = numerics::spectrum(&weighted)
        .iter()
        .map(|z| z.norm_sqr())
        .collect();
    row["band_1500_6000"] =
        (p[1500..=6000].iter().sum::<f64>() / p[1..].iter().sum::<f64>()).into();
    row["slip_onsets"] = onsets.len().into();
    row["duty"] = duty.into();
    row["force_rms"] = (force[start..].iter().map(|v| v * v).sum::<f64>() / n as f64)
        .sqrt()
        .into();
    row["mean_slip_interval_ms"] = json!(if onsets.len() > 1 {
        Some(
            (onsets[onsets.len() - 1] - onsets[0]) as f64 / (onsets.len() - 1) as f64 * 1000.
                / n as f64,
        )
    } else {
        None
    });
    row["normal_force"] = (2. * c.k).into();
    row["static_force_limit"] = (2. * ratio * c.speed).into();
    row["measured_period_ms"] = (1000. / value(&row["pickup"], "hz")).into();
    Ok(row)
}
fn sustained(c: &Case, frames: &[[u8; 96]], planner: &mut FftPlanner<f64>) -> Result<Value> {
    let n = c.rate as usize;
    let p = samples(frames, 5);
    let start = stddev(&p[4 * n / 5..n]);
    let end = stddev(&p[p.len() - n / 5..]);
    ensure(start > 0., "decay voiced reference")?;
    let ratio = end / start;
    let delta = db(ratio);
    let sixth = stddev(&p[5 * n..6 * n]);
    let voiced = spec(&p[n..2 * n], c.hz, false, planner)?;
    let last = if c.trajectory == "lift" && stddev(&p[p.len() - n..]) == 0. {
        Value::Null
    } else {
        spec(&p[p.len() - n..], c.hz, false, planner)?
    };
    ensure(
        value(&voiced, "cents").abs() <= 12.,
        "decay voiced frequency",
    )?;
    if c.trajectory == "lift" {
        ensure(
            delta <= -60.
                && [6, 10]
                    .iter()
                    .all(|col| samples(&frames[n..], *col).iter().all(|v| *v == 0.)),
            "passive lift force/slip or decay",
        )?;
    } else {
        ensure(
            delta.abs() <= 2. && value(&last, "cents").abs() <= 12.,
            "driven sustain",
        )?;
    }
    if c.hz == 82.41 {
        ensure(
            (sixth
                - if c.trajectory == "lift" {
                    0.00014420
                } else {
                    0.22813106
                })
            .abs()
                < 0.000002,
            "sixth second RMS",
        )?;
    }
    Ok(
        json!({"start_rms":start,"end_rms":end,"six_second_rms":sixth,"delta_db":delta,"unfloored_delta_db":(ratio>0.).then(||20.*ratio.log10()),"last_second":last,"first_second_after_lift_time":voiced}),
    )
}
fn long(c: &Case, frames: &[[u8; 96]], planner: &mut FftPlanner<f64>) -> Result<Value> {
    let n = c.rate as usize;
    let mut windows = vec![];
    let audio = samples(frames, 11);
    let p = samples(frames, 5);
    let coeff = (-TAU * 20. / n as f64).exp();
    let mut memory = 0.;
    let mut max_err = 0f64;
    for (&v, &measured) in p.iter().zip(&audio) {
        let hp = coeff.mul_add(v, memory);
        memory = (-coeff).mul_add(v, coeff * hp);
        max_err = max_err.max((0.85 * hp - measured).abs());
    }
    ensure(max_err < 1e-12, "guide full-stream independent M5")?;
    for (i, chunk) in frames.chunks(n).enumerate() {
        let mut s = spec(&p[i * n..(i + 1) * n], c.hz, false, planner)?;
        let bridge = spec(&samples(chunk, 2), c.hz, false, planner)?;
        let dc = mean(&audio[i * n..(i + 1) * n]);
        ensure(
            value(&s, "cents").abs() <= 12.
                && value(&bridge, "cents").abs() <= 12.
                && dc.abs() < 0.002,
            "physical long window",
        )?;
        for k in ["rms", "hz", "eta"] {
            s[format!("bridge_{k}")] = bridge[k].clone();
        }
        s["mean_audio"] = dc.into();
        windows.push(s);
    }
    let mut row = json!({"mean_audio":mean(&audio),"m5_error":max_err});
    for k in ["rms", "bridge_rms"] {
        let levels: Vec<_> = windows[1..].iter().map(|w| value(w, k)).collect();
        let d = db(levels.iter().copied().fold(0., f64::max)
            / levels.iter().copied().fold(f64::INFINITY, f64::min));
        ensure(d < 2., "physical 60-second drift")?;
        row[format!("{k}_drift_db")] = d.into();
    }
    row["windows"] = windows.into();
    Ok(row)
}
pub fn run(out: &Path) -> Result<Value> {
    let cases: BTreeMap<String, Case> =
        serde_json::from_str(include_str!("../../contracts/physical-cases.json"))?;
    let exe = replay::build(out, None)?;
    let mut planner = FftPlanner::new();
    let mut rows = BTreeMap::new();
    let mut prefixes = BTreeMap::new();
    for (name, c) in &cases {
        let controls = observation::controls(c);
        ensure(
            format!("{:x}", Sha256::digest(&controls)) == c.controls_sha256,
            "guide control words",
        )?;
        let result = replay::stream(&exe, &[c.rate.to_string()], &controls)?;
        let n = c.successful_ticks;
        let counts = String::from_utf8(result.stderr)?;
        ensure(
            counts.trim() == format!("{n} 1 0 1") && result.stdout.len() == n * 96,
            "guide lifecycle/count",
        )?;
        ensure(
            format!("{:x}", Sha256::digest(&result.stdout)) == c.raw_sha256,
            "guide raw words",
        )?;
        let frames = result.stdout.as_chunks::<96>().0;
        let mut peaks = [0f64; 12];
        for frame in &frames[..(c.rate * c.seconds) as usize] {
            for (p, w) in peaks.iter_mut().zip(frame.as_chunks::<8>().0) {
                let v = f64::from_le_bytes(*w);
                ensure(v.is_finite(), "guide nonfinite")?;
                *p = p.max(v.abs());
            }
        }
        ensure(peaks[..7].iter().all(|v| *v <= 4.), "guide wave bound")?;
        let mut row = match c.trajectory.as_str() {
            "steady" if c.seconds == 60 => long(c, frames, &mut planner)?,
            "steady" => stationary(c, frames, &mut planner)?,
            "lift" | "driven" => {
                let hash = Sha256::digest(&result.stdout[..c.rate as usize * 96]);
                if let Some(prev) = prefixes.insert(c.hz.to_bits(), hash) {
                    ensure(prev == hash, "lift/driven prefix")?;
                }
                sustained(c, frames, &mut planner)?
            }
            "bend" | "vibrato" => phase::grade(c, frames, out, name)?,
            _ => return Err("unknown physical trajectory".into()),
        };
        if let Some(expected) = &c.wav_sha256 {
            let audio: Vec<_> = samples(&frames[..(c.rate * c.seconds) as usize], 11)
                .into_iter()
                .map(|v| v as f32)
                .collect();
            let wav = observation::float_wav(c.rate, &audio);
            ensure(
                peaks[11] < 1. && format!("{:x}", Sha256::digest(&wav)) == *expected,
                "guide engineering WAV",
            )?;
            fs::write(out.join(format!("{name}.wav")), wav)?;
        }
        row["raw_sha256"] = c.raw_sha256.clone().into();
        row["controls_sha256"] = c.controls_sha256.clone().into();
        row["peaks"] = json!(peaks);
        rows.insert(name, row);
    }
    let case = |s: &str| &rows[&s.to_string()];
    let lo = case("steady_48000_440_R1.2_V0.1_3s");
    let hi = case("steady_48000_440_R2.4_V0.1_3s");
    ensure(
        (value(hi, "normal_force") / value(lo, "normal_force") - 2.).abs() <= 1e-6
            && value(hi, "band_1500_6000") - value(lo, "band_1500_6000") >= 0.01,
        "physical pressure response",
    )?;
    let fast = case("steady_48000_440_R1.8_V0.16_3s");
    let slow = case("steady_48000_440_R1.8_V0.06_3s");
    ensure(
        value(&fast["pickup"], "rms") / value(&slow["pickup"], "rms") >= 1.8,
        "physical speed response",
    )?;
    Ok(json!({"cases":rows,"producer":evidence::digest(&exe)?}))
}

pub fn differential(reference: &Path, root: &Path) -> Result<Value> {
    let path = reference.join("guide.json");
    ensure(
        evidence::digest(&path)?
            == "49d6e48753acc85491bffb88d320a910d4e0d3ead0b0df08613f31a05de1784a",
        "frozen guide reference identity",
    )?;
    let expected = evidence::strict(&fs::read(path)?)?;
    let actual = crate::publication::current_artifact(root, "guide", "evidence.json")?;
    let doc = evidence::strict(&fs::read(&actual)?)?;
    let cases = doc["results"]["cases"]
        .as_object()
        .ok_or("guide result cases")?;
    ensure(
        cases.len() == 152 && expected.as_object().is_some_and(|m| m.len() == 152),
        "guide case inventory",
    )?;
    fn compare(a: &Value, b: &Value, field: &str, max: &mut f64, words: &mut usize) -> Result<()> {
        match a {
            Value::Object(map) => {
                for (k, v) in map {
                    if k != "phase_reference_sha256" {
                        compare(
                            v,
                            b.get(k).ok_or_else(|| format!("missing {field}/{k}"))?,
                            &format!("{field}/{k}"),
                            max,
                            words,
                        )?;
                    }
                }
            }
            Value::Array(arr) => {
                let other = b.as_array().ok_or("guide field type")?;
                ensure(arr.len() == other.len(), "guide field length")?;
                for (x, y) in arr.iter().zip(other) {
                    compare(x, y, field, max, words)?;
                }
            }
            Value::Number(n) if n.is_f64() => {
                let x = n.as_f64().unwrap();
                let y = b.as_f64().ok_or("guide measurement type")?;
                let err = (x - y).abs() / 1f64.max(x.abs());
                ensure(err <= 1e-10, &format!("{field} reference {x} vs {y}"))?;
                *max = max.max(err);
                *words += 1;
            }
            _ => {
                ensure(a == b, &format!("guide decision/identity {field}"))?;
                *words += 1;
            }
        };
        Ok(())
    }
    let mut result = BTreeMap::new();
    for (name, case) in expected.as_object().unwrap() {
        let mut maximum = 0.;
        let mut words = 0;
        compare(case, &cases[name], name, &mut maximum, &mut words)?;
        let mut phase_error = 0f64;
        if let Some(hash) = case["phase_reference_sha256"].as_str() {
            let name = format!("{name}-phase.f64le");
            let old = reference.join(&name);
            ensure(evidence::digest(&old)? == hash, "guide phase reference SHA")?;
            let (a, b) = (
                fs::read(old)?,
                fs::read(actual.parent().unwrap().join(&name))?,
            );
            ensure(
                a.len() == b.len() && a.len() % 32 == 0,
                "guide phase frame inventory",
            )?;
            for (i, (x, y)) in a
                .as_chunks::<8>()
                .0
                .iter()
                .zip(b.as_chunks::<8>().0)
                .enumerate()
            {
                if i % 4 == 1 {
                    let error = (f64::from_le_bytes(*x) - f64::from_le_bytes(*y)).abs();
                    ensure(error <= 1e-9, "guide instantaneous phase cents")?;
                    phase_error = phase_error.max(error);
                } else {
                    ensure(x == y, "guide phase time/truth/lag target bits")?;
                }
                words += 1;
            }
        }
        result.insert(name,json!({"compared":words,"maximum_scaled_measurement_error":maximum,"maximum_instantaneous_error_cents":phase_error}));
    }
    Ok(json!({"guide_run":doc["run_id"],"cases":result}))
}
