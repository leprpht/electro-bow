//! Grade the original Stage 08 adversarial claims against fresh Rust measurements.
use super::{integrity, legacy, stationary, sweeps};
use crate::{Result, ensure, evidence, publication};
use rustfft::num_complex::Complex64;
use serde_json::{Value, json};
use std::{collections::BTreeMap, f64::consts::TAU, fs, io::Read, path::Path};
fn number(v: &Value) -> Result<f64> {
    v.as_f64().ok_or("missing numeric study measurement".into())
}
fn grade(rows: &[Value], neutral: &[Value], fs: f64, hz: f64) -> Result<()> {
    ensure(rows.len() == 3 && neutral.len() == 3, "measurement depths")?;
    let ratio = |x: f64, y: f64| 20. * (x / y).max(1e-12).log10();
    let m5 =
        number(&neutral[0]["rms_FS"])? / 10f64.powf(number(&neutral[0]["final_over_M5"])? / 20.);
    for (j, r) in rows.iter().enumerate() {
        let level = ratio(number(&r["rms_FS"])?, m5);
        let color = ratio(number(&r["rms_FS"])?, number(&neutral[j]["rms_FS"])?);
        ensure(
            (level - number(&r["final_over_M5"])?).abs() < 1e-9,
            "original M5 denominator",
        )?;
        ensure(
            (color - number(&r["corrected_over_neutral"])?).abs() < 1e-9,
            "same-depth reference",
        )?;
        ensure(
            stationary::near(r["measured_f0"].as_f64(), hz) && r["failure_reason"].is_null(),
            "ineligible M5",
        )?;
        ensure(level.abs() <= 3. && color.abs() <= 3., "level")?;
        ensure(
            stationary::near(r["strongest_hz"].as_f64(), hz),
            "fundamental",
        )?;
        let h = &r["harmonics"];
        let count = (1..=12)
            .filter(|k| f64::from(*k) * r["measured_f0"].as_f64().unwrap() < fs / 2.)
            .count();
        ensure(
            ["peak_FS", "phase_deg", "dBr"]
                .into_iter()
                .all(|k| h[k].as_array().is_some_and(|v| v.len() == count)),
            "harmonic inventory",
        )?;
        for i in 0..count {
            let a = number(&h["peak_FS"][i])?;
            let first = number(&h["peak_FS"][0])?;
            let eligible = a > 1e-12 && first > 1e-12;
            ensure(
                h["phase_deg"][i].is_number() == eligible && h["dBr"][i].is_number() == eligible,
                "harmonic eligibility",
            )?;
            if eligible {
                ensure(
                    number(&h["phase_deg"][i])?.abs() <= 180.
                        && (number(&h["dBr"][i])? - ratio(a, first)).abs() < 1e-9,
                    "relative harmonic",
                )?;
            }
        }
        let eta = number(&r["spectral"]["eta"])?;
        ensure(
            (0. ..=1.).contains(&eta)
                && (eta + number(&r["spectral"]["residual"])? - 1.).abs() < 1e-12,
            "spectral energy",
        )?;
        if j == 1 || (j == 0 && fs == 32000. && hz == 1318.51) {
            ensure(eta >= 0.85, "harmonic fraction")?;
        }
        if j == 1 {
            let residual = ratio(
                number(&r["factory_wet_difference_rms_FS"])?,
                number(&rows[0]["rms_FS"])?,
            );
            ensure(
                (residual - number(&r["factory_difference_over_off"])?).abs() < 1e-9
                    && residual <= -28.,
                "final-wet residual",
            )?;
        } else {
            ensure(
                r["factory_difference_over_off"].is_null()
                    && r["factory_wet_difference_rms_FS"].is_null(),
                "ineligible factory residual",
            )?;
        }
        ensure(
            r["passes"]
                .as_object()
                .is_some_and(|m| m.values().all(|v| v.is_null() || v == true)),
            "recorded V17 predicate",
        )?;
    }
    Ok(())
}
pub fn run(root: &Path) -> Result<Value> {
    let read = |suite: &str| -> Result<Value> {
        Ok(evidence::strict(&fs::read(publication::current_artifact(
            root,
            suite,
            "evidence.json",
        )?)?)?["results"]
            .take())
    };
    let study = read("i1-study")?;
    let stresses = read("o2-acoustic")?;
    let mut grid: BTreeMap<String, Value> =
        serde_json::from_value(study["grid"]["stationary"].clone())?;
    let contract: stationary::Contract =
        serde_json::from_str(include_str!("../../contracts/stationary.json"))?;
    let inventory = |rows: &BTreeMap<String, Value>| {
        ensure(
            rows.len() == contract.cases.len()
                && contract.cases.iter().all(|c| rows.contains_key(&c.case)),
            "I1 case inventory",
        )
    };
    inventory(&grid)?;
    for c in &contract.cases {
        let r = &grid[&c.case];
        let neutral = r["neutral_depths"].as_array().ok_or("neutral rows")?;
        for label in ["depths", "neutral_depths"] {
            grade(
                r[label].as_array().ok_or("depths")?,
                neutral,
                c.rate.into(),
                c.hz,
            )?;
        }
    }
    let mut rejected = BTreeMap::new();
    let mut reject = |name: &str, result: Result<()>, reason: &str| -> Result<()> {
        let message = result
            .err()
            .ok_or_else(|| format!("escaped {name}"))?
            .to_string();
        ensure(
            message.contains(reason),
            &format!("{name} rejected at {message}, expected {reason}"),
        )?;
        rejected.insert(name.to_string(), message);
        Ok(())
    };
    let reference = &grid["48000/196/0.1/1.8/opposed"];
    let neutral = reference["neutral_depths"]
        .as_array()
        .ok_or("neutral depths")?;
    for (name, reason) in [
        ("inversion", "fundamental"),
        ("upper", "level"),
        ("lower", "level"),
    ] {
        let mut bad = neutral.clone();
        bad[0] = stresses["v17"][format!("stress/196/{name}")].clone();
        reject(
            &format!("independent clipping {name}"),
            grade(&bad, neutral, 48000., 196.),
            reason,
        )?;
    }
    for (name, index, key, value, reason) in [
        (
            "wrong M5 denominator",
            0,
            "final_over_M5",
            0.,
            "original M5",
        ),
        (
            "wrong residual",
            1,
            "factory_difference_over_off",
            -240.,
            "final-wet",
        ),
        ("ineligible phase", 0, "harmonics", 0., "eligibility"),
    ] {
        let mut bad = reference["depths"]
            .as_array()
            .ok_or("measured depths")?
            .clone();
        if key == "harmonics" {
            bad[index][key]["phase_deg"][0] = Value::Null;
        } else {
            bad[index][key] = value.into();
        }
        reject(name, grade(&bad, neutral, 48000., 196.), reason)?;
    }
    for r in grid.values_mut() {
        r["neutral"] = r["neutral_depths"].clone();
    }
    sweeps::expression(&grid)?;
    let mut flat = grid.clone();
    flat.get_mut("48000/440/0.1/2.4/opposed").unwrap()["depths"] =
        grid["48000/440/0.1/1.2/opposed"]["depths"].clone();
    reject(
        "lost pressure variation",
        sweeps::expression(&flat).map(drop),
        "pressure contrast",
    )?;
    for name in ["bright", "dark"] {
        let r = grid.get_mut(&format!("48000/196/0.1/1.8/{name}")).unwrap();
        r["depths"] = r["neutral"].clone();
    }
    reject(
        "neutral-only identity",
        sweeps::expression(&grid).map(drop),
        "source transfer",
    )?;
    grid.pop_first();
    reject("missing required case", inventory(&grid), "inventory")?;
    let mut archive = zip::ZipArchive::new(fs::File::open(publication::current_artifact(
        root,
        "i1-study",
        "listening.zip",
    )?)?)?;
    let mut word = Vec::new();
    archive
        .by_name("listening-gains.json")?
        .read_to_end(&mut word)?;
    let gain = number(&evidence::strict(&word)?["neutral-held"])?;
    let mut wav = |group: &str| -> Result<Vec<f64>> {
        let mut data = Vec::new();
        archive
            .by_name(&format!("{group}/neutral-held.wav"))?
            .read_to_end(&mut data)?;
        Ok(hound::WavReader::new(&data[..])?
            .samples::<f32>()
            .map(|x| x.map(f64::from))
            .collect::<std::result::Result<_, _>>()?)
    };
    let engineering = wav("engineering")?;
    let matched = wav("matched")?;
    let normalize = |g: f64| {
        ensure(
            engineering.len() == matched.len()
                && engineering
                    .iter()
                    .zip(&matched)
                    .all(|(a, b)| (a * g - b).abs() < 1e-7),
            "hidden listening normalization",
        )
    };
    normalize(gain)?;
    reject(
        "hidden normalization",
        normalize(2. * gain),
        "normalization",
    )?;
    let bound = |peak: f64, input: f64| {
        ensure(
            peak.is_finite() && peak <= input + 1e-12,
            "contractive bank bound",
        )
    };
    bound(1., 1.)?;
    reject("unbounded bank state", bound(1.01, 1.), "contractive")?;
    reject(
        "unsafe smoothing",
        integrity::trajectory(&[0., 1.], 240),
        "trajectory",
    )?;
    let phase = (0..1960)
        .map(|i| Complex64::from_polar(1., -TAU * 8. * i as f64 / 1960.))
        .collect::<Vec<_>>();
    let base = phase.iter().map(|z| z.re).collect::<Vec<_>>();
    let changed = base
        .iter()
        .enumerate()
        .map(|(i, x)| 0.5 * x + if i % 2 == 0 { 0.6 } else { -0.6 })
        .collect::<Vec<_>>();
    ensure(
        legacy::allowed(&base, &phase, 0.5, 0.5, false),
        "healthy harmonic",
    )?;
    reject(
        "Nyquist veto",
        ensure(
            legacy::allowed(&changed, &phase, 0.5, 0.5, false),
            "realized projection/energy gate",
        ),
        "projection",
    )?;
    ensure(rejected.len() == 13, "historical study negatives")?;
    Ok(json!({"rejected":rejected,"stationary_case_count":contract.cases.len()}))
}
