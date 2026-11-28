//! Controlled recording preflight. Hash decoded source, interval and attack before any wet processing.
use crate::{Result, ensure, evidence, observation};
use hound::{SampleFormat, WavReader};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
fn hash(x: &[f64]) -> String {
    let mut h = Sha256::new();
    for &v in x {
        h.update((if v == 0. { 0. } else { v }).to_le_bytes());
    }
    format!("{:x}", h.finalize())
}
fn read(path: &Path) -> Result<(u32, Vec<f64>)> {
    let mut wav = WavReader::open(path)?;
    let spec = wav.spec();
    ensure(
        spec.channels == 1 && spec.sample_rate >= 16000,
        "invalid mono recording or unsupported DI decimator rate",
    )?;
    let data = match spec.sample_format {
        SampleFormat::Float => wav
            .samples::<f32>()
            .map(|v| v.map(f64::from))
            .collect::<std::result::Result<Vec<_>, _>>()?,
        SampleFormat::Int => {
            let scale = 2f64.powi(i32::from(spec.bits_per_sample) - 1);
            wav.samples::<i32>()
                .map(|v| v.map(|w| f64::from(w) / scale))
                .collect::<std::result::Result<Vec<_>, _>>()?
        }
    };
    ensure(
        data.iter().all(|v| v.is_finite()),
        "nonfinite decoded recording",
    )?;
    Ok((spec.sample_rate, data))
}
fn path(dir: &Path, v: &Value, key: &str) -> Result<PathBuf> {
    Ok(dir
        .join(
            v[key]
                .as_str()
                .ok_or_else(|| format!("missing path {key}"))?,
        )
        .canonicalize()?)
}
fn interval(v: &Value, n: usize) -> Result<(usize, usize)> {
    let a = v.as_array().ok_or("invalid sample interval")?;
    ensure(a.len() == 2, "invalid sample interval")?;
    let lo = usize::try_from(a[0].as_u64().ok_or("invalid sample interval")?)?;
    let hi = usize::try_from(a[1].as_u64().ok_or("invalid sample interval")?)?;
    ensure(lo < hi && hi <= n, "invalid sample interval")?;
    Ok((lo, hi))
}
fn labels(c: &Value) -> Result<()> {
    ensure(
        c["manifest_revision"] == 2 && c["protocol_revision"] == "I1-repeatability-8.2",
        "recording protocol/manifest revision",
    )?;
    ensure(
        c["hz"]
            .as_f64()
            .is_some_and(|v| (65. ..=1320.).contains(&v)),
        "unsupported pitch",
    )?;
    let classes = c["classes"].as_object().ok_or("class labels")?;
    let varies = match c["comparison"].as_str() {
        Some("pickup") => &["pickup"][..],
        Some("string") => &["string", "fret"],
        Some("guitar") => &["guitar", "pickup", "string", "fret"],
        _ => return Err("class labels".into()),
    };
    ensure(
        classes.len() == 2 && classes.contains_key("A") && classes.contains_key("B"),
        "class labels",
    )?;
    for k in [
        "guitar",
        "pickup",
        "string",
        "fret",
        "chain",
        "gain",
        "gesture",
        "performer",
    ] {
        let a = &classes["A"][k];
        let b = &classes["B"][k];
        ensure(
            [a, b]
                .iter()
                .all(|v| v.is_number() || v.as_str().is_some_and(|s| !s.trim().is_empty())),
            &format!("missing label {k}"),
        )?;
        ensure(varies.contains(&k) || a == b, &format!("uncontrolled {k}"))?;
    }
    let k = c["comparison"].as_str().unwrap();
    ensure(classes["A"][k] != classes["B"][k], "identical class")
}
pub fn evaluate(manifest: &Path) -> Result<Value> {
    let dir = manifest.parent().ok_or("manifest parent")?;
    let config = evidence::strict(&fs::read(manifest)?)?;
    labels(&config)?;
    let shared = match config["recording_mode"].as_str() {
        Some("shared-session") => true,
        Some("separate-originals") => false,
        _ => return Err("recording mode".into()),
    };
    let session = if shared {
        let desc = &config["session"];
        let audio = path(dir, desc, "original")?;
        let log = path(dir, desc, "attack_log")?;
        ensure(
            evidence::digest(&audio)? == desc["sha256"]
                && evidence::digest(&log)? == desc["attack_log_sha256"],
            "session audio or attack log identity",
        )?;
        let log = evidence::strict(&fs::read(log)?)?;
        ensure(
            log.as_array().is_some_and(|v| v.len() == 8),
            "attack log inventory",
        )?;
        Some((read(&audio)?, log))
    } else {
        None
    };
    let mut originals = BTreeMap::new();
    let mut segments = BTreeMap::new();
    let mut ranges: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut ordered = vec![];
    let mut failures = vec![];
    let mut takes = BTreeMap::new();
    for label in ["A", "B"] {
        let class = &config["classes"][label];
        let items = class["takes"].as_array().ok_or("missing takes")?;
        ensure(items.len() == 4, "four independent attacks per setting")?;
        for (i, item) in items.iter().enumerate() {
            let name = format!("{label}{}", i + 1);
            let mut errors = vec![];
            let mut report = json!({"metadata":item});
            let result = (|| -> Result<()> {
                let file = path(dir, item, "original")?;
                let (rate, x) = read(&file)?;
                let original = evidence::digest(&file)?;
                let whole = hash(&x);
                let (lo, hi) = interval(&item["interval"], x.len())?;
                let y = &x[lo..hi];
                let start = y
                    .iter()
                    .position(|v| v.abs() > 0.01)
                    .ok_or("absent attack")?;
                let count = (0.30 * f64::from(rate)).round_ties_even() as usize;
                ensure(
                    y.len() - start >= count && y.iter().all(|v| v.abs() < 1.),
                    "clipped or incomplete take",
                )?;
                let segment = hash(y);
                let capture = hash(&y[start..start + count]);
                report = json!({"metadata":item,"original":original,"decoded":whole,"segment":segment,"capture":capture,"fs":rate,"attack":lo+start});
                if originals
                    .insert(whole.clone(), file.clone())
                    .is_some_and(|p| p != file)
                {
                    errors.push("duplicate decoded recording".to_owned());
                }
                if let Some(prev) = segments.insert(capture, name.clone()) {
                    errors.push(format!("duplicate decoded capture {prev}"));
                }
                let prior = ranges.entry(whole.clone()).or_default();
                if prior.iter().any(|(a, b)| *a < hi && lo < *b) {
                    errors.push("overlapping recording segments".into());
                }
                prior.push((lo, hi));
                let order = 2 * i + 1 + usize::from(label == "B");
                for (k, v) in [
                    ("original_sha256", json!(original)),
                    ("decoded_sha256", json!(whole)),
                    ("segment_sha256", json!(segment)),
                    ("rate", json!(rate)),
                    ("order", json!(order)),
                ] {
                    if item[k] != v {
                        errors.push(format!("incorrect {k}"));
                    }
                }
                let uid = item["take_id"].as_str().unwrap_or("");
                if uid.trim().is_empty() || !ids.insert(uid.to_owned()) {
                    errors.push("missing or duplicate take identity".into());
                }
                if !item["attack_record"]
                    .as_str()
                    .is_some_and(|s| !s.trim().is_empty())
                {
                    errors.push("missing physical attack documentation".into());
                }
                if let Some(((sr, session), log)) = &session {
                    let (begin, end) = interval(&item["session_interval"], session.len())?;
                    ensure(
                        *sr == rate && hash(&session[begin..end]) == segment,
                        "session sample coordinates",
                    )?;
                    let attack = begin + start;
                    ensure(
                        log[order - 1]
                            == json!({"order":order,"take_id":uid,"attack":attack,"interval":[begin,end],"pickup":class["pickup"],"gain":class["gain"],"gesture":class["gesture"]}),
                        "session attack log entry",
                    )?;
                    ordered.push((order, begin, end, attack));
                }
                Ok(())
            })();
            if let Err(e) = result {
                errors.push(e.to_string());
            }
            failures.extend(errors.iter().map(|e| format!("{name}: {e}")));
            report["failures"] = json!(errors);
            takes.insert(name, report);
        }
    }
    if let Some(((rate, _), _)) = session {
        ordered.sort();
        let min = (0.3 * f64::from(rate)).round_ties_even() as usize;
        for w in ordered.windows(2) {
            if w[1].1 < w[0].2 || w[1].3 < w[0].3 + min {
                failures.push("shared session chronological independent attack intervals".into());
            }
        }
    } else if originals.len() != 8 {
        failures.push("separate originals require eight original recordings".into());
    }
    Ok(
        json!({"manifest_sha256":evidence::digest(manifest)?,"takes":takes,"failures":failures,"eligible_for_di_analysis":failures.is_empty()}),
    )
}
pub fn preflight(dir: &Path, manifest: &Path) -> Result<Value> {
    let result = evaluate(manifest)?;
    evidence::atomic(&dir.join("recordings.json"), &result)?;
    ensure(
        result["eligible_for_di_analysis"] == true,
        "ineligible controlled recordings, see recordings.json",
    )?;
    Ok(result)
}
pub fn controls(dir: &Path) -> Result<Value> {
    let mut rows = Vec::new();
    for scenario in [
        "separate",
        "shared",
        "duplicate containers",
        "duplicate captures",
        "overlapping attacks",
    ] {
        let dest = dir.join(scenario);
        fs::create_dir(&dest)?;
        let rate = 16000;
        let mut classes = json!({});
        let mut session = vec![];
        let mut log = vec![];
        for i in 0..8 {
            let label = if i % 2 == 0 { "A" } else { "B" };
            let name = format!("{label}{}", i / 2 + 1);
            let base = if matches!(scenario, "duplicate containers" | "duplicate captures") {
                i % 2
            } else {
                i
            };
            let mut x: Vec<_> = (0..5000)
                .map(|t| ((0.1 + base as f64 / 100.) * (t as f64 * 0.01).cos()) as f32)
                .collect();
            if scenario == "duplicate captures" {
                x[4999] = i as f32 * 0.01;
            }
            let begin = session.len();
            session.extend(&x);
            let end = session.len();
            let filename = format!("{name}.wav");
            let mut wav = observation::float_wav(rate, &x);
            wav.extend((i as u16).to_le_bytes());
            fs::write(dest.join(&filename), wav)?;
            let (_, values) = read(&dest.join(&filename))?;
            let sha = hash(&values);
            let order = i + 1;
            if i < 2 {
                classes[label] = json!({"guitar":"synthetic","pickup":label,"string":"G","fret":0,"chain":"offline","gain":1,"gesture":"test","performer":"generator","takes":[]});
            }
            let mut interval = [begin, end];
            if scenario == "overlapping attacks" && i == 7 {
                interval = [0, 5000];
            }
            let take = json!({"original":filename,"interval":[0,x.len()],"order":order,"take_id":name,"rate":rate,"original_sha256":evidence::digest(&dest.join(&filename))?,"decoded_sha256":sha,"segment_sha256":sha,"attack_record":"synthetic preflight test","session_interval":interval});
            classes[label]["takes"].as_array_mut().unwrap().push(take);
            log.push(json!({"order":order,"take_id":name,"attack":interval[0],"interval":interval,"pickup":label,"gain":1,"gesture":"test"}));
        }
        let shared = matches!(scenario, "shared" | "overlapping attacks");
        let mut config = json!({"classes":classes,"hz":196,"comparison":"pickup","protocol_revision":"I1-repeatability-8.2","manifest_revision":2,"recording_mode":if shared{"shared-session"}else{"separate-originals"}});
        if shared {
            if scenario == "overlapping attacks" {
                // Give the last take the actual first interval and valid decoded/audio/log identities.
                let original = &config["classes"]["A"]["takes"][0];
                let mut last = config["classes"]["B"]["takes"][3].clone();
                for key in [
                    "original",
                    "original_sha256",
                    "decoded_sha256",
                    "segment_sha256",
                ] {
                    last[key] = original[key].clone();
                }
                config["classes"]["B"]["takes"][3] = last;
            }
            fs::write(
                dest.join("session.wav"),
                observation::float_wav(rate, &session),
            )?;
            evidence::atomic(&dest.join("attacks.json"), &json!(log))?;
            config["session"] = json!({"original":"session.wav","sha256":evidence::digest(&dest.join("session.wav"))?,"attack_log":"attacks.json","attack_log_sha256":evidence::digest(&dest.join("attacks.json"))?});
        }
        let manifest = dest.join("manifest.json");
        evidence::atomic(&manifest, &config)?;
        let verdict = evaluate(&manifest)?;
        let errors = verdict["failures"].as_array().unwrap();
        let marker = match scenario {
            "duplicate containers" => "duplicate decoded recording",
            "duplicate captures" => "duplicate decoded capture",
            "overlapping attacks" => "shared session chronological independent attack intervals",
            _ => "",
        };
        ensure(
            if marker.is_empty() {
                errors.is_empty()
            } else {
                errors.iter().any(|v| v.as_str().unwrap().contains(marker))
            },
            &format!("recording control {scenario}: {errors:?}"),
        )?;
        rows.push(json!({"scenario":scenario,"actual":verdict,"synthetic":true}));
    }
    Ok(json!({"controls":rows}))
}
