//! Preregistered calibration/withheld split. No class label or recording is inferred.
use super::{evaluate, path, read};
use crate::{
    Result,
    electrical::{carrier, scheduled_taps},
    ensure, evidence,
    identity::{capture, epoch::Kernel, stationary},
    numerics, replay,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
#[derive(Clone)]
struct Features {
    vector: [f64; 12],
    cues: [f64; 2],
    eligible: [bool; 2],
    orders: [bool; 12],
    bands: [f64; 4],
}
impl Features {
    fn new(h: [f64; 12]) -> Self {
        let peak = h.iter().copied().fold(0., f64::max);
        let floor = (0.015 * peak).max(0.00096);
        let orders = h.map(|v| v > floor);
        let levels = h.map(|v| db(v / peak).max(-50.));
        let m = levels.iter().sum::<f64>() / 12.;
        let norm = |lo: usize, hi: usize| h[lo..hi].iter().map(|v| v * v).sum::<f64>().sqrt();
        Self {
            vector: levels.map(|v| v - m),
            cues: [
                levels[5] - (levels[4] + levels[6]) / 2.,
                db(norm(4, 10) / norm(0, 4)),
            ],
            eligible: [
                orders[4..7].iter().all(|v| *v),
                orders[..10].iter().all(|v| *v),
            ],
            orders,
            bands: [(0, 2), (2, 4), (4, 8), (8, 12)].map(|(a, b)| db(norm(a, b) / norm(0, 12))),
        }
    }
    fn value(&self) -> Value {
        json!({"vector":self.vector,"cues":self.cues,"eligible":self.eligible,"orders":self.orders,"bands":self.bands})
    }
}
fn db(v: f64) -> f64 {
    20. * v.max(1e-12).log10()
}
fn diff(a: &[f64; 12], b: &[f64; 12]) -> f64 {
    (a.iter().zip(b).map(|(a, b)| (a - b).powi(2)).sum::<f64>() / 12.).sqrt()
}
type Records = BTreeMap<String, Features>;
fn distance(records: &Records, a: &str, b: &str) -> f64 {
    diff(&records[a].vector, &records[b].vector)
}
fn discrimination(rec: &Records) -> Value {
    let mut report = json!({});
    for (k, start) in [("calibration", 1), ("withheld", 3)] {
        let mut between = vec![];
        for i in start..=start + 1 {
            for j in start..=start + 1 {
                between.push(distance(rec, &format!("A{i}"), &format!("B{j}")));
            }
        }
        let within =
            ["A", "B"].map(|c| distance(rec, &format!("{c}{start}"), &format!("{c}{}", start + 1)));
        let margin = between.iter().copied().fold(f64::INFINITY, f64::min)
            - within.into_iter().fold(0., f64::max);
        report[k] = json!({"within":within,"between":between,"margin":margin});
    }
    let center = |c: &str| {
        std::array::from_fn(|i| {
            (rec[&format!("{c}1")].vector[i] + rec[&format!("{c}2")].vector[i]) / 2.
        })
    };
    for (c, other) in [("A", "B"), ("B", "A")] {
        for n in [3, 4] {
            let key = format!("{c}{n}");
            let v = &rec[&key].vector;
            report["matches"][&key] = (diff(v, &center(c)) < diff(v, &center(other))).into();
        }
    }
    for (a, va) in rec {
        for (b, vb) in
            rec.range::<String, _>((std::ops::Bound::Excluded(a), std::ops::Bound::Unbounded))
        {
            report["all_pairs"][format!("{a}/{b}")] = diff(&va.vector, &vb.vector).into();
        }
    }
    report
}
fn cue(rec: &Records, k: usize, indices: &[usize], threshold: f64) -> i32 {
    let mut smallest = f64::INFINITY;
    let mut largest = f64::NEG_INFINITY;
    for i in indices {
        for j in indices {
            let a = &rec[&format!("A{i}")];
            let b = &rec[&format!("B{j}")];
            if !a.eligible[k] || !b.eligible[k] {
                return 0;
            }
            let d = a.cues[k] - b.cues[k];
            smallest = smallest.min(d);
            largest = largest.max(d);
        }
    }
    if smallest >= threshold {
        1
    } else if largest <= -threshold {
        -1
    } else {
        0
    }
}
fn rejected(report: &mut Value, why: &str) -> Result<Value> {
    let mut pairs = BTreeMap::new();
    let names: Vec<_> = ["A", "B"]
        .iter()
        .flat_map(|c| (1..=4).map(move |i| format!("{c}{i}")))
        .collect();
    for (i, a) in names.iter().enumerate() {
        for b in &names[i + 1..] {
            pairs.insert(format!("{a}/{b}"), json!({"distance":null,"reason":why}));
        }
    }
    if report.get("di_pairs").is_none() {
        report["di_pairs"] = json!(pairs);
    }
    report["wet_pairs"] = json!(pairs);
    report["status"] = why.into();
    Ok(report.clone())
}
fn render(
    raw: &[[f64; 30]],
    q: &[[f64; 4]],
    pitch: f64,
    kernel: &Kernel,
    rate: u32,
    di: &[f64],
) -> Result<(Vec<[f64; 4]>, bool)> {
    let mut updates = capture::updates(di, rate as usize, pitch)?;
    let nonzero = updates.iter().any(|(_, v)| v.iter().any(|x| *x != 0.));
    for (tick, _) in &mut updates {
        *tick = (*tick as f64 * 48000. / f64::from(rate)).round_ties_even() as usize;
    }
    let mut updates = updates.iter().peekable();
    let mut target = [0.; 12];
    let mut z = [0.; 12];
    let mut epoch = kernel.epoch(48000.);
    let smoother = -(-1f64 / (0.015 * 48000.)).exp_m1();
    let mut wet = Vec::with_capacity(raw.len());
    for (start, chunk) in raw.chunks(8192).enumerate() {
        let frames: Vec<_> = chunk
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let n = start * 8192 + i;
                while updates.peek().is_some_and(|(t, _)| *t <= n) {
                    target = updates.next().unwrap().1;
                }
                let mut f = [0.; 20];
                f[0] = pitch;
                f[1] = row[11];
                f[2..6].copy_from_slice(&q[n]);
                f[18..].fill(1.);
                for j in 0..12 {
                    f[6 + j] = smoother.mul_add(target[j], z[j]);
                    z[j] = (1. - smoother) * f[6 + j];
                }
                f
            })
            .collect();
        let mut out = vec![[0.; 20]; frames.len()];
        epoch.run(&frames, &mut out, 0)?;
        for r in out {
            wet.push(r[4..8].try_into().unwrap());
        }
    }
    Ok((wet, nonzero))
}
pub(super) fn evaluate_cell(out: &Path, manifest: &Path) -> Result<Value> {
    let config = evidence::strict(&std::fs::read(manifest)?)?;
    let mut report = match evaluate(manifest) {
        Ok(v) => v,
        Err(e) => {
            return rejected(
                &mut json!({"manifest_sha256":evidence::digest(manifest)?,"failures":[e.to_string()]}),
                "ineligible recording manifest",
            );
        }
    };
    let pitch = config["hz"].as_f64().ok_or("recording nominal pitch")?;
    let mut inputs = Records::new();
    let mut takes = BTreeMap::new();
    let mut failures = report["failures"].as_array().unwrap().clone();
    let names: Vec<_> = report["takes"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    for name in &names {
        let take = &report["takes"][name];
        if take.get("capture").is_none() {
            continue;
        }
        let item = &take["metadata"];
        let (rate, x) = read(&path(manifest.parent().unwrap(), item, "original")?)?;
        let start = take["attack"].as_u64().unwrap() as usize;
        let hi = item["interval"][1].as_u64().unwrap() as usize;
        let samples = &x[start..hi];
        let segment = &samples[(0.08 * f64::from(rate)).round_ties_even() as usize
            ..(0.23 * f64::from(rate)).round_ties_even() as usize];
        let lo = pitch * 2f64.powf(-40. / 1200.);
        let stop = pitch * 2f64.powf(40. / 1200.) + 0.025;
        let mut best = (f64::NEG_INFINITY, lo);
        let step = (lo + 0.025) - lo;
        for k in 0..((stop - lo) / 0.025).ceil() as usize {
            let f = lo + k as f64 * step;
            let a = numerics::partials(segment, f64::from(rate), f, 1)[0].norm();
            if a > best.0 {
                best = (a, f);
            }
        }
        if (1200. * (best.1 / pitch).log2()).abs() > 15. {
            failures.push(format!("{name} DI pitch mismatch").into());
        }
        let h: Vec<_> = numerics::partials(segment, f64::from(rate), best.1, 12)
            .iter()
            .map(|z| z.norm())
            .collect();
        if h.len() != 12 || h.iter().copied().fold(0., f64::max) <= 1e-5 {
            failures.push(format!("{name} DI harmonic eligibility").into());
            continue;
        }
        inputs.insert(name.clone(), Features::new(h.try_into().unwrap()));
        takes.insert(name.clone(), (rate, samples.to_vec(), best.1));
    }
    for (i, a) in names.iter().enumerate() {
        for b in &names[i + 1..] {
            report["di_pairs"][format!("{a}/{b}")] =
                if inputs.contains_key(a) && inputs.contains_key(b) {
                    json!({"distance":distance(&inputs,a,b),"reason":null})
                } else {
                    json!({"distance":null,"reason":"ineligible take"})
                };
        }
    }
    for i in 1..=4 {
        for j in 1..=4 {
            let (a, b) = (format!("A{i}"), format!("B{j}"));
            if let (Some(x), Some(y)) = (takes.get(&a), takes.get(&b))
                && (1200. * (x.2 / y.2).log2()).abs() > 10.
            {
                failures.push(format!("{a}/{b} between-take pitch mismatch").into());
            }
        }
    }
    report["failures"] = json!(failures);
    if !failures.is_empty() {
        return rejected(&mut report, "ineligible controlled recordings");
    }
    let di = discrimination(&inputs);
    let signs = [0, 1].map(|k| {
        let sign = cue(&inputs, k, &[1, 2], 2.);
        if sign == cue(&inputs, k, &[1, 2, 3, 4], 2.) {
            sign
        } else {
            0
        }
    });
    let pass = |d: &Value, t: f64| {
        ["calibration", "withheld"]
            .iter()
            .all(|s| d[s]["margin"].as_f64().unwrap() >= t)
            && d["matches"]
                .as_object()
                .unwrap()
                .values()
                .all(|v| v == true)
    };
    report["registered_signs"] = json!(signs);
    report["inputs"] = json!(
        inputs
            .iter()
            .map(|(k, v)| (k, v.value()))
            .collect::<BTreeMap<_, _>>()
    );
    report["di"] = di.clone();
    if !pass(&di, 1.) {
        failures.push("DI split margin or withheld centroid".into());
    }
    if signs == [0, 0] {
        failures.push("no eligible stable registered DI cue".into());
    }
    if !failures.is_empty() {
        report["failures"] = json!(failures);
        return rejected(&mut report, "DI discrimination ineligible");
    }
    let exe = replay::build(out, None)?;
    let kernel = Kernel::build(out)?;
    let n = 144000;
    let ctl = [pitch, 0.1, 1.8 * 0.1 / 1.65]
        .map(f64::to_le_bytes)
        .as_flattened()
        .repeat(n);
    let raw = carrier::scheduled(&exe, 48000, n, &ctl)?;
    let velocity: Vec<_> = raw.iter().map(|r| r[21]).collect();
    let fourth = scheduled_taps(&exe, 48000., &velocity, |_| 0.5)?;
    let q: Vec<_> = raw
        .iter()
        .zip(fourth)
        .map(|(r, v)| [r[25], r[26], r[27], v[3]])
        .collect();
    for j in 0..3 {
        let row = stationary::grade(&raw, &q, 48000, pitch, j)?;
        if row["passes"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v == false)
        {
            report["upstream"] = row;
            return rejected(&mut report, "ineligible upstream carrier");
        }
    }
    let anchor = stationary::Spectrum::new(
        &raw[n - 48000..].iter().map(|r| r[11]).collect::<Vec<_>>(),
        48000,
    )
    .peak
    .ok_or("voiced recorded carrier")?;
    let mut wet = Records::new();
    for (name, (rate, samples, _)) in takes {
        let (y, captured) = render(&raw, &q, pitch, &kernel, rate, &samples)?;
        if !captured {
            failures.push(format!("{name} missing capture").into());
        }
        let mut depths = vec![];
        for j in 0..3 {
            let row = stationary::grade(&raw, &y, 48000, pitch, j)?;
            for (k, v) in row["passes"].as_object().unwrap() {
                if v == false {
                    failures.push(format!("{name}/{j}/{k}").into());
                }
            }
            depths.push(row);
        }
        report["wet_v17"][&name] = json!(depths);
        let col: Vec<_> = y[n - 48000..].iter().map(|r| r[0]).collect();
        let h: Vec<_> = numerics::partials(&col, 48000., anchor, 12)
            .iter()
            .map(|v| v.norm())
            .collect();
        wet.insert(
            name,
            Features::new(h.try_into().map_err(|_| "controlled wet orders")?),
        );
    }
    let score = discrimination(&wet);
    for (k, sign) in signs.into_iter().enumerate() {
        if sign != 0 && cue(&wet, k, &[3, 4], 0.5) != sign {
            failures.push("withheld wet signed cue".into());
            break;
        }
    }
    for s in ["calibration", "withheld"] {
        if score[s]["margin"].as_f64().unwrap() < 0.5 {
            failures.push(format!("{s} wet margin").into());
        }
    }
    for (take, v) in score["matches"].as_object().unwrap() {
        if v == false {
            failures.push(format!("{take} wet centroid").into());
        }
    }
    report["wet"] = json!(
        wet.iter()
            .map(|(k, v)| (k, v.value()))
            .collect::<BTreeMap<_, _>>()
    );
    report["score"] = score;
    report["status"] = if failures.is_empty() {
        "passed"
    } else {
        "held discrimination failed"
    }
    .into();
    report["failures"] = json!(failures);
    Ok(report)
}
pub fn run(out: &Path, manifest: &Path) -> Result<Value> {
    let result = evaluate_cell(out, manifest)?;
    evidence::atomic(&out.join("controlled.json"), &result)?;
    ensure(
        result["status"] == "passed",
        result["status"]
            .as_str()
            .unwrap_or("controlled recording failure"),
    )?;
    Ok(result)
}
