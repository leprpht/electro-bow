//! Regrade malformed, coherently edited reports and sample payloads against fresh process-owned results.
use crate::{Result, ensure, evidence, publication};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
const CASE: &str = "48000/196/0.1/1.8/opposed";
fn grade(
    candidate: &Value,
    fresh: &Value,
    files: &BTreeMap<String, String>,
    expected: &BTreeMap<String, String>,
) -> Result<()> {
    ensure(
        files.keys().eq(expected.keys()),
        "sample artifact inventory",
    )?;
    for (k, v) in expected {
        ensure(files[k] == *v, &format!("waveform/clock replay {k}"))?;
    }
    let a = candidate.as_object().ok_or("report object")?;
    let b = fresh.as_object().ok_or("reference report object")?;
    ensure(a.keys().eq(b.keys()), "required reports")?;
    for (k, v) in b {
        ensure(&a[k] == v, &format!("remeasured {k}"))?;
    }
    Ok(())
}
fn set(data: &mut [u8], at: usize, x: f64) {
    data[8 * at..8 * at + 8].copy_from_slice(&x.to_le_bytes());
}
fn read(data: &[u8], at: usize) -> f64 {
    f64::from_le_bytes(data[8 * at..8 * at + 8].try_into().unwrap())
}
pub fn run(root: &Path) -> Result<Value> {
    let mut reports = BTreeMap::new();
    let mut paths = BTreeMap::new();
    let mut sources = BTreeMap::new();
    for s in [
        "i1-stationary",
        "i1-motion",
        "i1-stereo",
        "i1-sources",
        "i1-clock",
        "i1-events",
        "i1-path-mutants",
        "i1-transaction-mutants",
        "i1-layout",
        "i1-qr",
        "i1-capture",
        "i1-review82",
        "i1-listening",
        "recording-controls",
    ] {
        let receipt = publication::inspect(&root.join(s), s)?;
        ensure(
            receipt["status"] == "passed",
            "successful child evidence required",
        )?;
        let base = root
            .join(s)
            .join(receipt["run_id"].as_str().ok_or("run id")?);
        let payload = evidence::strict(&fs::read(base.join("evidence.json"))?)?;
        reports.insert(s, payload["results"].clone());
        sources.insert(s, receipt["run_id"].clone());
        for name in receipt["artifacts"]
            .as_object()
            .ok_or("artifact inventory")?
            .keys()
        {
            if name.ends_with(".f64le")
                || name.ends_with(".f32le")
                || name.ends_with(".zip")
                || name.ends_with(".wav")
            {
                paths.insert(format!("{s}/{name}"), base.join(name));
            }
        }
    }
    let fresh = json!(reports);
    let originals = paths
        .iter()
        .map(|(k, p)| Ok((k.clone(), evidence::digest(p)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    grade(&fresh, &fresh, &originals, &originals)?;
    let mut verdict = BTreeMap::new();
    for (name, owner) in [
        ("coherent residual", "i1-stationary"),
        ("empty harmonics", "i1-stationary"),
        ("fabricated sweep", "i1-stationary"),
        ("truncated timeline", "i1-stationary"),
        ("false controls", "i1-path-mutants"),
        ("omitted source", "i1-sources"),
        ("incorrect denominator", "i1-stationary"),
        ("omitted motion", "i1-motion"),
    ] {
        let mut copy = fresh.clone();
        let r = &mut copy[owner];
        match name {
            "coherent residual" => {
                let row = &mut r["stationary"][CASE]["depths"][1];
                row["factory_difference_over_off"] = (-240).into();
                row["factory_wet_difference_rms_FS"] = 0.into();
            }
            "empty harmonics" => {
                r["stationary"][CASE]["depths"][0]["harmonics"] =
                    json!({"peak_FS":[],"dBr":[],"phase_deg":[]});
            }
            "fabricated sweep" => {
                r["sweep"]["weak-H1/0.5"] =
                    json!({"peak":{"hz":-1,"dB":-200,"width_hz":null},"complex_error":0});
            }
            "truncated timeline" => {
                r["stationary"][CASE]["timeline"]
                    .as_array_mut()
                    .ok_or("timeline")?
                    .pop();
            }
            "false controls" => {
                *r = Value::Bool(false);
            }
            "omitted source" => {
                r["sources"].as_object_mut().unwrap().remove("techs-P1-60");
            }
            "incorrect denominator" => {
                r["stationary"][CASE]["depths"][0]["final_over_M5"] = 0.into()
            }
            "omitted motion" => {
                r["cases"].as_object_mut().unwrap().remove("vibrato-192000");
            }
            _ => unreachable!(),
        }
        let reason = grade(&copy, &fresh, &originals, &originals)
            .err()
            .ok_or_else(|| format!("surviving {name}"))?
            .to_string();
        ensure(
            reason == format!("remeasured {owner}"),
            "wrong first report predicate",
        )?;
        verdict.insert(name, json!({"first":reason,"exit":1}));
    }
    for (name, suite, file) in [
        (
            "unsafe independent gain",
            "i1-motion",
            "steady-evidence.f64le",
        ),
        ("bypassed slew", "i1-motion", "veto-evidence.f64le"),
        ("always neutral", "i1-motion", "vibrato-evidence.f64le"),
        ("incorrect paired wet", "i1-motion", "bend-evidence.f64le"),
        ("hidden normalization", "i1-motion", "held-evidence.f64le"),
        ("stale feature", "i1-stereo", "reference.f64le"),
        ("wrong physical epoch", "i1-stereo", "reference.f64le"),
        ("incorrect clock", "i1-stereo", "reference.f64le"),
    ] {
        let key = format!("{suite}/{file}");
        let mut data = fs::read(paths.get(&key).ok_or("expected sample artifact")?)?;
        let mut candidate = originals.clone();
        if suite == "i1-motion" {
            let (rows, tail) = data.as_chunks_mut::<{ 13 * 8 }>();
            ensure(tail.is_empty(), "motion evidence row width")?;
            for (t, r) in rows.iter_mut().enumerate() {
                for j in 0..4 {
                    let v = match name {
                        "unsafe independent gain" => read(r, j) + read(r, 4 + j),
                        "bypassed slew" if t >= 64800 => {
                            set(r, 12, 0.);
                            read(r, j)
                        }
                        "always neutral" => read(r, j),
                        "incorrect paired wet" if j == 1 => read(r, 8),
                        "hidden normalization" => read(r, 8 + j) * 0.9,
                        _ => continue,
                    };
                    set(r, 8 + j, v);
                }
            }
        } else if name == "incorrect clock" {
            data.rotate_right(22 * 8);
        } else if name == "wrong physical epoch" {
            for t in 800..6000 {
                set(&mut data, 22 * t, 5.);
            }
        } else {
            let stale = read(&data, 9000 * 22 + 8);
            for t in 480..8000 {
                set(&mut data, 22 * t + 8, stale);
            }
            let k = "i1-stereo/reference-coefficients.f64le";
            let mut coeff = fs::read(&paths[k])?;
            let sample = coeff[9000 * 192..9001 * 192].to_vec();
            for t in 480..8000 {
                coeff[192 * t..192 * (t + 1)].copy_from_slice(&sample);
            }
            candidate.insert(k.into(), format!("{:x}", Sha256::digest(&coeff)));
        }
        candidate.insert(key.clone(), format!("{:x}", Sha256::digest(data)));
        let reason = grade(&fresh, &fresh, &candidate, &originals)
            .err()
            .ok_or_else(|| format!("surviving {name}"))?
            .to_string();
        let expected = if name == "stale feature" {
            "i1-stereo/reference-coefficients.f64le"
        } else {
            &key
        };
        ensure(
            reason == format!("waveform/clock replay {expected}"),
            "wrong first sample predicate",
        )?;
        verdict.insert(name, json!({"first":reason,"exit":1}));
    }
    ensure(verdict.len() == 16, "I1 evidence negative inventory")?;
    Ok(json!({"controls":verdict,"sources":sources,"sample_artifacts":originals}))
}
