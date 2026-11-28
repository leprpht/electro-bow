//! Public scorer inputs come directly from the producer's full 30-column stream.
use super::carrier;
use crate::{
    Result, ensure, evidence,
    identity::{Bank, stationary},
    numerics,
    observation::Case,
    replay,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    f64::consts::{SQRT_2, TAU},
    path::Path,
};
pub(crate) mod listening;
pub(crate) mod resample;
const VECTORS: [&str; 4] = ["neutral", "upper", "lower", "inversion"];
fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
}
fn column(r: &[[f64; 30]], j: usize) -> Vec<f64> {
    r.iter().map(|s| s[j]).collect()
}
fn harmonics(x: &[f64], fs: u32, f: f64) -> Value {
    let h = numerics::partials(x, fs as f64, f, 12);
    let m = x.iter().sum::<f64>() / x.len() as f64;
    let voiced = (x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / x.len() as f64).sqrt() > 1e-12
        && x.len() as f64 * f >= 2. * fs as f64;
    let eligible = |i: usize| voiced && h[0].norm() > 1e-12 && h[i].norm() > 1e-12;
    json!({"peak_FS":h.iter().map(|z|z.norm()).collect::<Vec<_>>(),"dBr":(0..h.len()).map(|i|eligible(i).then(||20.*(h[i].norm()/h[0].norm()).max(1e-12).log10())).collect::<Vec<_>>(),"phase_deg":h.iter().enumerate().map(|(i,z)|eligible(i).then(||(*z*rustfft::num_complex::Complex64::from_polar(1.,-((i+1) as f64)*h[0].arg())).arg().to_degrees())).collect::<Vec<_>>()})
}
fn corrected(raw: &[[f64; 30]], c: &Case, vector: usize) -> Result<Vec<[f64; 4]>> {
    let mut bank = Bank::new(c.rate as f64)?;
    let mut frame = [0.; 17];
    frame[0] = c.hz;
    for (k, v) in frame[5..].iter_mut().enumerate() {
        let db = match (vector, k) {
            (1, _) | (3, 1) => 5.,
            (2, _) | (3, 0) => -5.,
            _ => 0.,
        };
        *v = 10f64.powf(db / 20.) - 1.;
    }
    raw.iter()
        .map(|r| {
            frame[1..5].copy_from_slice(&[r[25], r[26], r[27], r[26]]);
            let corr = bank.apply(&frame)?;
            Ok(std::array::from_fn(|k| frame[1 + k] + corr[k]))
        })
        .collect()
}
pub(crate) fn pluck(f: f64) -> Vec<f64> {
    let mut x: Vec<_> = (0..144000)
        .map(|i| {
            let t = i as f64 / 48000.;
            let angle = TAU * f * t;
            -(-t / 0.001).exp_m1()
                * (-t / 0.28).exp()
                * (1..=8)
                    .map(f64::from)
                    .map(|k| (k * angle + 0.13 * k * k).sin() / k)
                    .sum::<f64>()
        })
        .collect();
    let peak = x.iter().map(|v| v.abs()).fold(0., f64::max);
    for v in &mut x {
        *v = 0.35 * *v / peak;
    }
    x
}
pub fn run(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let cases = carrier::cases()?;
    let ids = carrier::identities();
    let mut v17 = BTreeMap::new();
    let mut corners = vec![];
    let mut high = BTreeMap::new();
    let mut onsets = BTreeMap::new();
    let mut clips = BTreeMap::new();
    let mut witness = Value::Null;
    let mut sources = BTreeMap::new();
    let corner_refs: Vec<_> = include_str!("../../../validation/fixtures/o2_corner_peaks.tsv")
        .lines()
        .skip(1)
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .collect();
    let direct = pluck(440.);
    for (name, c) in cases {
        let fs = c.rate;
        let n = fs as usize;
        let f = c.hz;
        let ratio = (c.k * 1.65 / c.speed * 10.).round() / 10.;
        let corner =
            fs == 48000 && f == 440. && c.speed != 0.1 && ratio != 1.8 && c.trajectory == "steady";
        let steady = c.trajectory == "steady"
            && c.seconds == 3
            && ((fs == 48000 && matches!(f, 82.41 | 196. | 440.) && c.speed == 0.1)
                || (f == 1318.51 && ratio == 1.8)
                || (fs == 48000 && f == 440. && c.speed == 0.16 && ratio == 1.8));
        let lift = c.trajectory == "lift" && matches!(f, 82.41 | 440.);
        if !(corner || steady || lift) {
            continue;
        }
        let raw = carrier::render(&exe, &c)?;
        let mut digest = Sha256::new();
        for frame in &raw {
            digest.update(frame.map(f64::to_le_bytes).as_flattened());
        }
        let hash = format!("{:x}", digest.finalize());
        ensure(ids[&name.as_str()][2] == hash, "acoustic source identity")?;
        sources.insert(name.clone(), hash.clone());
        let q: Vec<_> = raw.iter().map(|s| [s[25], s[26], s[27], s[26]]).collect();
        if steady || corner {
            let detail: Vec<_> = raw[raw.len() - n..].iter().map(|r| r[26] - r[24]).collect();
            ensure(
                rms(&detail) > 1e-12
                    && raw
                        .iter()
                        .all(|r| ((r[27] - r[24]) - 2. * (r[26] - r[24])).abs() <= 1e-9),
                "absent or nonlinear carrier detail",
            )?;
        }
        if steady {
            for (j, d) in [0., 0.5, 1.].into_iter().enumerate() {
                let row = stationary::grade(&raw, &q, fs, f, j)?;
                ensure(
                    !row["passes"]
                        .as_object()
                        .unwrap()
                        .values()
                        .any(|v| v == false),
                    "neutral V17",
                )?;
                v17.insert(format!("{fs}/{f}/{}/{ratio}/{d}", c.speed), row);
            }
            if fs == 48000 && f < 1000. && ratio == 1.8 && c.speed == 0.1 {
                for (v, id) in VECTORS.iter().enumerate().skip(1) {
                    let y = corrected(&raw, &c, v)?;
                    let row = stationary::grade(&raw, &y, fs, f, 0)?;
                    for (k, val) in row["passes"].as_object().unwrap() {
                        ensure(
                            (val == false)
                                == (if v == 3 {
                                    k == "fundamental"
                                } else {
                                    matches!(k.as_str(), "level" | "coloration")
                                }),
                            "historical unsafe V17 disposition",
                        )?;
                    }
                    v17.insert(format!("stress/{f}/{id}"), row);
                }
                let pair = format!("{f}Hz");
                clips.insert((pair.clone(), "M5".to_string()), column(&raw[..n], 11));
                clips.insert(
                    (pair, "O2".to_string()),
                    raw[..n].iter().map(|r| SQRT_2 * r[25]).collect(),
                );
                if f == 440. {
                    for (j, id) in [(25, "off"), (26, "factory")] {
                        clips.insert(
                            ("detail".into(), id.into()),
                            raw[..n].iter().map(|r| SQRT_2 * r[j]).collect(),
                        );
                    }
                }
            }
            if c.speed == 0.16 {
                let p = raw.iter().map(|s| s[25].abs()).fold(0., f64::max);
                let vals = [
                    SQRT_2 * p,
                    p,
                    p * 1.4 * 2.,
                    20. * (p * 1.4 * 2. / 0.98).log10(),
                ];
                ensure(
                    vals.into_iter()
                        .zip([1.005419, 0.710938, 1.990628, 6.155])
                        .enumerate()
                        .all(|(i, (a, b))| (a - b).abs() < if i == 3 { 0.0005 } else { 1e-6 }),
                    "Review 5 gain witness",
                )?;
                witness = json!(vals);
            }
            if f == 1318.51 {
                let anchor = stationary::Spectrum::new(&column(&raw[raw.len() - n..], 11), fs).peak;
                for (j, id, gain) in [
                    (21, "raw", 1.),
                    (22, "blocked", 1.),
                    (24, "loaded", SQRT_2 / 0.85),
                    (26, "factory", SQRT_2 / 0.85),
                    (27, "full", SQRT_2 / 0.85),
                ] {
                    let row = if stationary::near(anchor, f) {
                        let f0 = anchor.unwrap();
                        let x: Vec<_> = raw.iter().map(|r| gain * r[j]).collect();
                        let y = resample::common_window(&x, n);
                        let spec = stationary::Spectrum::new(&y, 32000);
                        let mut metrics = spec.band_metrics(32000, f0, 14000.);
                        let h = harmonics(&y, 32000, f0);
                        let count = (14000. / f0).floor() as usize;
                        metrics["amplitudes_m_s"] =
                            json!(&h["peak_FS"].as_array().unwrap()[..count]);
                        metrics["phase_deg"] = json!(&h["phase_deg"].as_array().unwrap()[..count]);
                        metrics["f0"] = f0.into();
                        metrics["rms_m_s"] = rms(&y).into();
                        metrics
                    } else {
                        json!({"failure_reason":"ineligible high-note bridge anchor","f0":anchor,"eta":null})
                    };
                    high.insert(format!("{fs}/{id}"), row);
                }
            }
        }
        if corner {
            for (v, id) in VECTORS.iter().enumerate() {
                let wet = if v == 0 {
                    q.clone()
                } else {
                    corrected(&raw, &c, v)?
                };
                for (j, d) in [0., 0.5, 1.].into_iter().enumerate() {
                    let expected = corner_refs
                        .iter()
                        .find(|r| {
                            r[0].parse() == Ok(c.speed)
                                && r[1].parse() == Ok(ratio)
                                && r[2].parse() == Ok(d)
                                && r[3] == *id
                        })
                        .ok_or("corner reference")?[4]
                        .parse::<f64>()?;
                    for gain in [0.08, 1.4] {
                        for character in [0., 1.] {
                            for out_gain in [0., 2.] {
                                let peak = direct
                                    .iter()
                                    .zip(&wet)
                                    .map(|(x, y)| {
                                        (out_gain
                                            * ((1. - character) * x + character * gain * y[j]))
                                            .abs()
                                    })
                                    .fold(0., f64::max);
                                let pre = out_gain
                                    * if character == 0. {
                                        0.35
                                    } else {
                                        gain * expected
                                    };
                                ensure((peak - pre).abs() <= 1e-9, "Stage 06 corner peak")?;
                                corners.push(json!({"V":c.speed,"R":ratio,"noise":d,"vector":id,"G":gain,"character":character,"internal_Out":out_gain,"peak":peak,"demand_dB":(20.*(peak/0.98).max(1e-12).log10()).max(0.),"trace":hash}));
                            }
                        }
                    }
                }
            }
        }
        if lift {
            let anchor = stationary::Spectrum::new(&column(&raw[..n], 11), fs).peak;
            for (j, d) in [0., 0.5, 1.].into_iter().enumerate() {
                for (start, end) in [
                    (0, 1),
                    (0, 480),
                    (0, 2400),
                    (24000, 48000),
                    (46800, 48000),
                    (48000, 49200),
                    (49200, 50400),
                    (52800, 55200),
                    (96000, 98400),
                    (288000, 336000),
                ] {
                    let section = &raw[start..end];
                    let y = column(section, j + 25);
                    let mean = y.iter().sum::<f64>() / y.len() as f64;
                    let ac: Vec<_> = y.iter().map(|v| v - mean).collect();
                    let harmonic = (y.len() >= 3 && stationary::near(anchor, f))
                        .then(|| harmonics(&y, fs, anchor.unwrap()));
                    let detail: Vec<_> = section.iter().map(|r| r[j + 25] - r[25]).collect();
                    let load: Vec<_> = section
                        .iter()
                        .map(|r| r[24] - 0.85 * r[22] / SQRT_2)
                        .collect();
                    onsets.insert(format!("{f}/{d}/{start}-{end}"),json!({"interval":[start,end],"rms":rms(&y),"ac_rms":rms(&ac),"peak":y.iter().map(|v|v.abs()).fold(0.,f64::max),"mean":mean,"harmonic_FS":harmonic,"detail_difference_rms":rms(&detail),"load_minus_input_rms":rms(&load)}));
                }
                if f == 82.41 && j == 1 {
                    clips.insert(
                        ("lift".into(), "O2".into()),
                        raw[..2 * n].iter().map(|r| SQRT_2 * r[26]).collect(),
                    );
                }
            }
        }
    }
    for (key, row) in high.clone() {
        let reference = &high[&format!("192000/{}", key.split_once('/').unwrap().1)];
        let diffs =
            if row.get("failure_reason").is_none() && reference.get("failure_reason").is_none() {
                let deltas: Vec<_> = row["amplitudes_m_s"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .zip(reference["amplitudes_m_s"].as_array().unwrap())
                    .map(|(a, b)| {
                        let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
                        (a > 1e-12 && b > 1e-12).then(|| 20. * (a / b).max(1e-12).log10())
                    })
                    .collect();
                [
                    json!(
                        row["centroid_hz"].as_f64().unwrap()
                            - reference["centroid_hz"].as_f64().unwrap()
                    ),
                    json!(deltas),
                    json!(
                        row["high_fraction"].as_f64().unwrap()
                            - reference["high_fraction"].as_f64().unwrap()
                    ),
                ]
            } else {
                [Value::Null, Value::Null, Value::Null]
            };
        for (k, v) in [
            "centroid_minus_ref_hz",
            "H1_to_H10_delta_dB",
            "high_fraction_minus_ref",
        ]
        .into_iter()
        .zip(diffs)
        {
            high.get_mut(&key).unwrap()[k] = v;
        }
    }
    ensure(
        v17.len() == 60
            && corners.len() == 384
            && high.len() == 35
            && onsets.len() == 60
            && clips.len() == 9,
        "O2 acoustic row inventories",
    )?;
    // Publish gain corners in the same speed/pressure/vector/depth order as the differential contract.
    corners.sort_by_key(|r| {
        (
            r["V"].as_f64().unwrap().to_bits(),
            r["R"].as_f64().unwrap().to_bits(),
            VECTORS.iter().position(|v| r["vector"] == *v).unwrap(),
            r["noise"].as_f64().unwrap().to_bits(),
            r["G"].as_f64().unwrap().to_bits(),
            r["character"].as_f64().unwrap().to_bits(),
            r["internal_Out"].as_f64().unwrap().to_bits(),
        )
    });
    let listening = listening::write(out, &clips)?;
    let mut nominal = vec![0f32; 4800];
    nominal.extend(pluck(196.).into_iter().map(|v| v as f32));
    nominal.resize(nominal.len() + 62400, 0.);
    let bytes: Vec<_> = nominal.iter().flat_map(|v| v.to_le_bytes()).collect();
    Ok(
        json!({"v17":v17,"sources":sources,"corners":corners,"high_band":high,"onset_release":onsets,"witness":witness,"listening":listening,"nominal_sha256":format!("{:x}",Sha256::digest(bytes)),"executable":evidence::digest(&exe)?}),
    )
}

pub fn differential(reference: &Path, root: &Path) -> Result<Value> {
    let path = reference.join("acoustic-result.json");
    ensure(
        evidence::digest(&path)?
            == "72f49cbf7b13939920fee56afadcd72f6b61f7f69edb0122e93b20c4df9bcdcd",
        "frozen acoustic differential",
    )?;
    let old = evidence::strict(&std::fs::read(path)?)?;
    let p = crate::publication::current_artifact(root, "o2-acoustic", "evidence.json")?;
    let new = evidence::strict(&std::fs::read(p)?)?;
    let mut rows = BTreeMap::new();
    for (k, v) in old.as_object().unwrap() {
        let mut stats = (0, 0., 0.);
        compare(v, &new["results"][k], k, &mut stats)?;
        rows.insert(k,json!({"fields":stats.0,"largest_scaled_error":stats.1,"largest_phase_error_deg":stats.2}));
    }
    Ok(json!({"producer_run":new["run_id"],"fields":rows}))
}

pub(crate) fn compare(
    a: &Value,
    b: &Value,
    path: &str,
    stats: &mut (usize, f64, f64),
) -> Result<()> {
    match a {
        Value::Object(obj) => {
            ensure(
                b.as_object().is_some_and(|m| m.len() == obj.len()),
                "acoustic field inventory",
            )?;
            for (k, x) in obj {
                compare(
                    x,
                    b.get(k).ok_or_else(|| format!("absent {path}/{k}"))?,
                    &format!("{path}/{k}"),
                    stats,
                )?;
            }
        }
        Value::Array(arr) => {
            let vals = b.as_array().ok_or("acoustic differential array")?;
            ensure(arr.len() == vals.len(), "acoustic array length")?;
            for (x, y) in arr.iter().zip(vals) {
                compare(x, y, path, stats)?;
            }
        }
        Value::Number(n) if !n.is_f64() && b.is_f64() => {
            ensure(n.as_f64() == b.as_f64(), &format!("numeric control {path}"))?;
            stats.0 += 1;
        }
        Value::Number(n) if n.is_f64() => {
            let x = n.as_f64().unwrap();
            let y = b.as_f64().ok_or("non-numeric measurement")?;
            let e = (x - y).abs();
            let scaled = e / 1f64.max(x.abs());
            if path.ends_with("phase_deg") {
                ensure(e <= 1e-6, &format!("phase degrees {path}"))?;
                stats.2 = stats.2.max(e);
            } else {
                ensure(
                    scaled <= 1e-9,
                    &format!("acoustic measurement {path} {x} vs {y}"),
                )?;
                stats.1 = stats.1.max(scaled);
            }
            stats.0 += 1;
        }
        _ => {
            ensure(a == b, &format!("acoustic decision or identity {path}"))?;
            stats.0 += 1;
        }
    }
    Ok(())
}
