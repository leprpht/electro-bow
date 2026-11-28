mod stimulus;
use crate::{Result, ensure, process};
use nalgebra::{DMatrix, DVector};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    f64::consts::{PI, TAU},
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

#[derive(Deserialize)]
struct Case {
    group: String,
    fs: u32,
    f: String,
    shape: String,
    peak: String,
    stimulus_start: usize,
    score_from: usize,
    score_until: usize,
    sha256: String,
}

fn fir(dec: usize) -> Vec<f64> {
    let n = 32 * dec;
    let mut values: Vec<_> = (0..=n)
        .map(|i| {
            let phase = TAU * (i as f64 - n as f64 / 2.) / n as f64;
            let blackman = 0.42 + 0.5 * phase.cos() + 0.08 * (2. * phase).cos();
            let s = PI * (0.68 * (i as f64 - 16. * dec as f64) / dec as f64);
            blackman * if s == 0. { 1. } else { s.sin() / s }
        })
        .collect();
    let sum = stimulus::sum_numpy(&values);
    for v in &mut values {
        *v /= sum;
    }
    values
}
struct Windows {
    host: Vec<usize>,
    above: Vec<bool>,
    differences: Vec<f64>,
    w: usize,
    k: usize,
    h: usize,
    fa: f64,
    dec: usize,
}
fn analysis(x: &[f64], c: &Case) -> Result<Windows> {
    let fs = f64::from(c.fs);
    let dec = (c.fs / 8000) as usize;
    let fa = fs / dec as f64;
    let w = (fa / 65.).ceil() as usize;
    let k = w + 1;
    let h = (0.005 * fa + 0.5).floor() as usize;
    let hp = (-TAU * 30. / fs).exp();
    let alpha = -(-1. / (0.004 * fs)).exp_m1();
    let taps = fir(dec);
    let mut raw = Vec::with_capacity(x.len());
    let mut fast = Vec::with_capacity(x.len());
    let (mut state, mut energy) = (0., 0.);
    for &v in x {
        let y = hp.mul_add(v, state);
        state = hp.mul_add(y, -hp * v);
        let e = alpha.mul_add(y * y, energy);
        energy = (1. - alpha) * e;
        raw.push(y);
        fast.push(e.sqrt());
    }
    let mut z = Vec::with_capacity(x.len() / dec);
    for end in (dec - 1..x.len()).step_by(dec) {
        let length = (end + 1).min(taps.len());
        z.push(
            taps[..length]
                .iter()
                .zip(raw[end + 1 - length..=end].iter().rev())
                .map(|(a, b)| a * b)
                .sum::<f64>(),
        );
    }
    let level = match c.group.as_str() {
        "DI" => 0.004,
        "unpitched" => 0.,
        _ => 0.001,
    };
    let mut host = vec![];
    let mut above = vec![];
    let mut differences = vec![];
    for end in (h - 1..z.len()).step_by(h) {
        let tick = (end + 1) * dec - 1;
        if end + 1 < w + k || tick < c.score_from || tick >= c.score_until {
            continue;
        }
        let t = end + 1 - w;
        let window = &z[t..=end];
        host.push(tick);
        above.push(fast[tick] >= level);
        for lag in 0..=k {
            let sum = window
                .iter()
                .zip(&z[t - lag..=end - lag])
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>();
            differences.push(sum);
        }
    }
    if c.group == "counter" || (c.group == "unpitched" && c.shape == "noise-30") {
        let mut padded = vec![0.; w + k];
        padded.extend(&z);
        let mut sums = vec![0.; k + 1];
        let mut cursor = 1;
        for i in 0..z.len() {
            let end = i + w + k;
            for (lag, v) in sums.iter_mut().enumerate() {
                *v += (padded[end] - padded[end - lag]).powi(2)
                    - (padded[end - w] - padded[end - w - lag]).powi(2);
            }
            if (i + 1) % h == 0 {
                for (lag, v) in sums.iter_mut().enumerate() {
                    let direct = (end + 1 - w..=end)
                        .map(|t| (padded[t] - padded[t - lag]).powi(2))
                        .sum::<f64>();
                    ensure((direct - *v).abs() < 1e-8, "direct A2 recurrence/rebase")?;
                    if lag == cursor {
                        *v = direct;
                    }
                }
                cursor = 1 + cursor % k;
            }
        }
        for (j, &filtered) in z.iter().enumerate() {
            let end = (j + 1) * dec - 1;
            let begin = (end + 1).saturating_sub(taps.len());
            let chronological = (begin..=end).map(|i| raw[i] * taps[end - i]).sum::<f64>();
            ensure(
                (filtered - chronological).abs() <= 1e-13,
                "direct FIR parity",
            )?;
        }
    }
    Ok(Windows {
        host,
        above,
        differences,
        w,
        k,
        h,
        fa,
        dec,
    })
}
fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let p = (sorted.len() - 1) as f64 * fraction;
    let i = p.floor() as usize;
    sorted[i] + p.fract() * (sorted[(i + 1).min(sorted.len() - 1)] - sorted[i])
}
fn movement(row: &mut Value, truth: &[f64], times: &[usize], freq: &[f64], c: &Case) -> Result<()> {
    let fs = f64::from(c.fs);
    let n = freq.len() as f64;
    let measured: DVector<_> =
        DVector::from_iterator(freq.len(), freq.iter().map(|f| 1200. * f.log2()));
    let mu = measured.mean();
    let mut best = (f64::NEG_INFINITY, 0, 0.);
    for ms in 0..=45 {
        let offset = (f64::from(ms) * fs / 1000.).round_ties_even() as usize;
        let aligned: DVector<_> = DVector::from_iterator(
            times.len(),
            times
                .iter()
                .map(|t| 1200. * truth[t.saturating_sub(offset)].log2()),
        );
        let err = (&aligned - &measured).norm() / n.sqrt();
        let a = aligned.add_scalar(-aligned.mean());
        let m = measured.add_scalar(-mu);
        let corr = a.dot(&m) / (a.norm() * m.norm());
        if corr > best.0 {
            best = (corr, ms, err);
        }
    }
    row["lag_ms"] = best.1.into();
    row["aligned_rms"] = best.2.into();
    if c.shape.starts_with("vib") {
        let mut fit = (f64::INFINITY, 0., 0.);
        for i in 0..=500 {
            let rate = 3. + f64::from(i) * ((3.0f64 + 0.01) - 3.);
            let b = DMatrix::from_fn(times.len(), 3, |j, k| {
                let phase = TAU * (times[j] as f64 / fs) * rate;
                match k {
                    0 => 1.,
                    1 => phase.sin(),
                    _ => phase.cos(),
                }
            });
            let (q, r) = b.clone().qr().unpack();
            let a = r
                .solve_upper_triangular(&(q.transpose() * &measured))
                .ok_or("singular vibrato fit")?;
            let error = (b * &a - &measured).norm();
            if error < fit.0 {
                fit = (error, rate, a[1].hypot(a[2]));
            }
        }
        row["modulation_hz"] = fit.1.into();
        row["depth_cents"] = fit.2.into();
    }
    Ok(())
}
fn summarize(
    c: &Case,
    w: &Windows,
    rank: &[[f64; 9]],
    truth: &[f64],
    method: usize,
    mode: usize,
) -> Result<Value> {
    let fs = f64::from(c.fs);
    let mut times = vec![];
    let mut freq = vec![];
    for (i, result) in rank.iter().enumerate() {
        let f = result[3 * method + mode] * w.fa;
        if w.above[i] && (0.995 * 65.0..=1.005 * 1320.).contains(&f) {
            freq.push(f.clamp(65., 1320.));
            times.push(w.host[i] - c.stimulus_start);
        }
    }
    let eligible = w.above.iter().filter(|&&ok| ok).count();
    let mut row = json!({"group":c.group,"fs":c.fs,"f":c.f,"shape":c.shape,"peak":c.peak,"method":(["integer","fitted","deepest"][method]),"mode":(["start","follow"][mode]),"sha256":c.sha256,"decimation":w.dec,"analysis_hz":w.fa,"W":w.w,"K":w.k,"H":w.h,"stimulus_start":c.stimulus_start,"score_from":c.score_from,"score_until":c.score_until,"window":w.host.len(),"eligible":eligible,"accepted":freq.len(),"rejected":eligible-freq.len()});
    if !freq.is_empty() {
        let mut sorted = freq.clone();
        sorted.sort_by(f64::total_cmp);
        row["median_hz"] = percentile(&sorted, 0.5).into();
        row["first_ms"] = (times[0] as f64 * 1000. / fs).into();
        if !truth.is_empty() || !c.f.is_empty() {
            let fixed = c.f.parse::<f64>().ok();
            let mut e: Vec<_> = freq
                .iter()
                .zip(&times)
                .map(|(f, t)| (1200. * (f / fixed.unwrap_or_else(|| truth[*t])).log2()).abs())
                .collect();
            row["rms"] = (e.iter().map(|x| x * x).sum::<f64>() / e.len() as f64)
                .sqrt()
                .into();
            e.sort_by(f64::total_cmp);
            row["median"] = percentile(&e, 0.5).into();
            row["p95"] = percentile(&e, 0.95).into();
            row["octave"] = e.iter().filter(|&&v| v >= 600.).count().into();
            if !truth.is_empty() {
                movement(&mut row, truth, &times, &freq, c)?;
            }
        }
    }
    if mode == 0 {
        let clean = || {
            freq.len() == eligible
                && !freq.is_empty()
                && row["median"].as_f64().is_some_and(|n| n <= 12.)
                && row["p95"].as_f64().is_some_and(|n| n <= 25.)
                && row["octave"]
                    .as_u64()
                    .is_some_and(|n| (n as f64) < 0.01 * eligible as f64)
        };
        if method == 1 && (c.group == "named" || c.group == "sweep" && c.shape == "six") {
            ensure(clean(), "named/six clean pitch")?;
        }
        if c.group == "counter" && method < 2 {
            let n = if c.fs == 48000 { 200 } else { 199 };
            ensure(
                freq.len() == n
                    && eligible == n
                    && row["octave"] == if method == 0 { n } else { 0 },
                "historical octave counterexample",
            )?;
            if method == 1 {
                ensure(clean(), "fitted counterexample")?;
            }
        }
        if method == 1 && c.group == "harmonics" && c.shape == "second" && c.peak == "0.35" {
            ensure(
                eligible == 200 && freq.len() == 200 && row["octave"] == 200,
                "dominant second counterexample",
            )?;
        }
        if method == 1 && c.group == "unpitched" {
            ensure(freq.is_empty(), "unpitched rejection")?;
        }
    }
    Ok(row)
}
pub fn run(out: &Path) -> Result<Value> {
    let binary = out.join(format!("pitch-candidates{}", std::env::consts::EXE_SUFFIX));
    process::check(
        Command::new("rustc")
            .args([
                "--edition",
                "2024",
                "-O",
                "tools/validation/pitch_candidates.rs",
                "-o",
            ])
            .arg(&binary),
    )?;
    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut writer = BufWriter::new(child.stdin.take().ok_or("pitch adapter stdin")?);
    let mut reader = BufReader::new(child.stdout.take().ok_or("pitch adapter stdout")?);
    let mut cases = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(include_bytes!("../../contracts/pitch.tsv").as_slice());
    let mut totals = BTreeMap::new();
    let mut output = BufWriter::new(fs::File::create(out.join("pitch.jsonl"))?);
    let mut manifest = Vec::new();
    let mut rows = 0;
    let result = (|| -> Result<()> {
        for c in cases.deserialize() {
            let c: Case = c?;
            let (x, truth) = stimulus::generate(&c)?;
            let mut hash = Sha256::new();
            for v in &x {
                hash.update(v.to_le_bytes());
            }
            let digest = format!("{:x}", hash.finalize());
            ensure(
                digest == c.sha256,
                &format!(
                    "pitch input {} {} {} {} {} got {digest}",
                    c.group, c.fs, c.f, c.shape, c.peak
                ),
            )?;
            manifest.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{digest}\n",
                c.group, c.fs, c.f, c.shape, c.peak
            ));
            let w = analysis(&x, &c)?;
            for header in [w.host.len(), (w.fa / 1320.).floor().max(2.) as usize, w.k] {
                writer.write_all(&u32::try_from(header)?.to_le_bytes())?;
            }
            for v in &w.differences {
                writer.write_all(&v.to_le_bytes())?;
            }
            writer.flush()?;
            let mut rank = Vec::with_capacity(w.host.len());
            for _ in &w.host {
                let mut bytes = [0u8; 72];
                reader.read_exact(&mut bytes)?;
                rank.push(std::array::from_fn(|i| {
                    f64::from_le_bytes(bytes.as_chunks::<8>().0[i])
                }));
            }
            for method in 0..3 {
                for mode in 0..2 {
                    let row = summarize(&c, &w, &rank, &truth, method, mode)?;
                    serde_json::to_writer(&mut output, &row)?;
                    writeln!(output)?;
                    rows += 1;
                }
            }
            *totals.entry(c.group).or_insert(0) += 1;
        }
        Ok(())
    })();
    drop(writer);
    drop(reader);
    let status = child.wait()?;
    result?;
    ensure(status.success(), "pitch adapter exit")?;
    output.flush()?;
    manifest.sort();
    let fingerprint = format!("{:x}", Sha256::digest(manifest.concat().as_bytes()));
    ensure(
        rows == 40350
            && fingerprint == "08c856c64164aaf2e7b38f7ff01f8d476a990adc9320a6704cddca2f27acf798",
        "pitch fixture inventory",
    )?;
    Ok(
        json!({"fixtures":totals,"rows":rows,"inputs_sha256":fingerprint,"rows_sha256":crate::evidence::digest(&out.join("pitch.jsonl"))?}),
    )
}

pub fn differential(out: &Path, root: &Path) -> Result<Value> {
    use std::io::BufRead;
    let execution = run(out)?;
    let path = root.join("checkout/build/spec-signoff/pitch.csv");
    ensure(
        crate::evidence::digest(&path)?
            == "a3a80badc95855f9fcaa92180ca61a9c2b375d3d4e7b1a1ff086e819fe3bc242",
        "frozen pitch CSV digest",
    )?;
    let mut old = csv::Reader::from_path(path)?;
    let header = old.headers()?.clone();
    let mut rows = BufReader::new(fs::File::open(out.join("pitch.jsonl"))?).lines();
    let mut max_errors: BTreeMap<String, f64> = BTreeMap::new();
    let mut count = 0;
    for record in old.records() {
        let record = record?;
        count += 1;
        let actual =
            crate::evidence::strict(rows.next().ok_or("missing pitch result")??.as_bytes())?;
        for (key, expected) in header.iter().zip(&record) {
            let measured = &actual[key];
            if let Some(text) = measured.as_str() {
                ensure(expected == text, key)?;
            } else if expected.is_empty() {
                ensure(measured.is_null(), key)?;
            } else {
                let error = (expected.parse::<f64>()?
                    - measured.as_f64().ok_or("missing pitch number")?)
                .abs();
                let limit = match key {
                    "median_hz" | "median" | "p95" | "rms" | "aligned_rms" | "depth_cents" => 1e-10,
                    _ => 0.,
                };
                ensure(
                    error <= limit,
                    &format!("pitch row {count}/{key}: {measured} vs {expected}"),
                )?;
                let max = max_errors.entry(key.to_string()).or_default();
                *max = max.max(error);
            }
        }
    }
    ensure(
        rows.next().is_none() && count == 40350,
        "differential pitch row inventory",
    )?;
    Ok(json!({"execution":execution,"compared_rows":count,"max_abs_error":max_errors}))
}
