//! Descriptive measurements of the fifteen retained O1 streams.
use super::Case;
use crate::{Result, electrical::acoustic, ensure, evidence, guide, numerics, publication};
use rustfft::{FftPlanner, num_complex::Complex64};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};

fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
}
fn eligible(x: &[f64], z: &[Complex64]) -> Vec<bool> {
    let mean = x.iter().sum::<f64>() / x.len() as f64;
    let voiced =
        (x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / x.len() as f64).sqrt() > 1e-12;
    z.iter().map(|a| voiced && a.norm() > 1e-12).collect()
}
fn column(raw: &[u8], k: usize) -> Vec<f64> {
    raw.as_chunks::<176>()
        .0
        .iter()
        .map(|r| f64::from_le_bytes(r[k * 8..k * 8 + 8].try_into().unwrap()))
        .collect()
}
fn measure(c: &Case, f: f64, raw: &[u8]) -> Result<Value> {
    let rate = c.rate as usize;
    let end = rate * c.seconds as usize;
    let all = column(raw, 21);
    let x = &all[..end];
    let tail = end - rate..end;
    let signals = [
        x[tail.clone()].to_vec(),
        column(raw, 17)[tail.clone()].to_vec(),
        column(raw, 3)[tail].to_vec(),
    ];
    let partials = signals
        .each_ref()
        .map(|s| numerics::partials(s, rate as f64, f, 12));
    let valid: [_; 3] = std::array::from_fn(|i| eligible(&signals[i], &partials[i]));
    let ratios = |top: usize, bottom: usize, fund: bool| {
        (0..partials[top].len())
            .map(|i| {
                let j = if fund { 0 } else { i };
                (valid[top][i] && valid[bottom][j]).then(|| {
                    20. * (partials[top][i] / partials[bottom][j])
                        .norm()
                        .max(1e-12)
                        .log10()
                })
            })
            .collect::<Vec<_>>()
    };
    let h = &partials[0];
    let mut r = json!({"rms_m_s":rms(x),"steady_rms_m_s":rms(&signals[0]),"peak_m_s":x.iter().map(|v|v.abs()).fold(0.,f64::max),"projection_Hz":f,"harmonic_peak_m_s":h.iter().map(|a|a.norm()).collect::<Vec<_>>(),"harmonic_phase_deg":h.iter().zip(&valid[0]).map(|(z,v)|v.then(||z.arg().to_degrees())).collect::<Vec<_>>(),"envelope_dBr":ratios(0,0,true),"center_over_N_dB":ratios(1,2,false),"aperture_over_center_dB":ratios(0,1,false)});
    if matches!(c.trajectory.as_str(), "bend" | "vibrato") {
        r["auxiliary_motion"] = guide::phase::auxiliary(c, &all)?;
    }
    if c.trajectory == "lift" {
        r["rms_50ms_at_0.95_1_1.10_2s"] = json!(
            [rate - rate / 20, rate, rate + rate / 10, 2 * rate].map(|a| rms(&x[a..a + rate / 20]))
        );
    }
    if c.hz == 1318.51 {
        let end = 32000 * c.seconds as usize;
        let y = acoustic::resample::window(x, rate, end - 32000..end);
        let m = y.iter().sum::<f64>() / y.len() as f64;
        let samples = y
            .iter()
            .zip(numerics::hann(y.len()))
            .map(|(v, w)| (v - m) * w)
            .collect::<Vec<_>>();
        let p = numerics::spectrum(&samples)
            .iter()
            .map(|v| v.norm_sqr())
            .collect::<Vec<_>>();
        let total = p.iter().sum::<f64>();
        r["common_32k_centroid_Hz"] =
            json!(p.iter().enumerate().map(|(i, v)| i as f64 * v).sum::<f64>() / total);
        r["common_32k_above_6k_fraction"] = json!(p[6000..].iter().sum::<f64>() / total);
    }
    Ok(r)
}
pub fn retained(
    out: &Path,
    cases: &BTreeMap<String, Case>,
    keys: &BTreeMap<String, String>,
) -> Result<Value> {
    let mut reports = BTreeMap::new();
    let mut planner = FftPlanner::new();
    for key in keys.keys() {
        let c = &cases[key];
        let (_, suffix) = key.split_once('_').ok_or("case label")?;
        let (controls, _) = suffix.rsplit_once('_').ok_or("case duration")?;
        let nominal = format!("steady_{controls}_3s");
        let nominal_raw = fs::read(out.join(format!("{nominal}.f64le")))?;
        let pickup = column(&nominal_raw, 5);
        let rate = c.rate as usize;
        let s = guide::spec(&pickup[pickup.len() - rate..], c.hz, false, &mut planner)?;
        let f = s["hz"].as_f64().ok_or("nominal pickup frequency")?;
        let raw = fs::read(out.join(format!("{key}.f64le")))?;
        reports.insert(key, measure(c, f, &raw)?);
    }
    Ok(json!(reports))
}

pub fn differential(reference: &Path, root: &Path) -> Result<Value> {
    let old = evidence::strict(&fs::read(
        reference.join("checkout/build/stage-05/manifest.json"),
    )?)?;
    let new = evidence::strict(&fs::read(publication::current_artifact(
        root,
        "o1-replay",
        "evidence.json",
    )?)?)?;
    let mut rows = BTreeMap::new();
    for (name, case) in old["cases"].as_object().ok_or("O1 case inventory")? {
        if let Some(report) = case.get("measurements") {
            let mut stats = (0, 0., 0.);
            acoustic::compare(
                report,
                &new["results"]["measurements"][name],
                name,
                &mut stats,
            )?;
            rows.insert(name,json!({"fields":stats.0,"maximum_scaled_error":stats.1,"maximum_phase_error_degrees":stats.2}));
        }
    }
    ensure(rows.len() == 15, "fifteen retained O1 measurement sets")?;
    Ok(json!({"producer":new["run_id"],"cases":rows}))
}

pub fn silent_control() -> Result<String> {
    let c = Case {
        rate: 48000,
        hz: 196.,
        speed: 0.1,
        k: 0.,
        seconds: 1,
        trajectory: "steady".into(),
        successful_ticks: 48000,
        controls_sha256: String::new(),
        raw_sha256: String::new(),
        wav_sha256: None,
    };
    let mut raw = vec![0u8; 48000 * 176];
    let check = |row: &Value| -> Result<()> {
        for key in [
            "harmonic_phase_deg",
            "envelope_dBr",
            "center_over_N_dB",
            "aperture_over_center_dB",
        ] {
            ensure(
                row[key]
                    .as_array()
                    .is_some_and(|a| a.len() == 12 && a.iter().all(Value::is_null)),
                &format!("ineligible {key}"),
            )?;
        }
        Ok(())
    };
    for gain in [0., 1e-22] {
        for (i, frame) in raw.as_chunks_mut::<176>().0.iter_mut().enumerate() {
            let x = gain * (std::f64::consts::TAU * 196. * i as f64 / 48000.).sin();
            frame[168..176].copy_from_slice(&x.to_le_bytes());
        }
        check(&measure(&c, 196., &raw)?)?;
    }
    let mut corrupted = measure(&c, 196., &raw)?;
    corrupted["harmonic_phase_deg"] = json!(vec![0.; 12]);
    Ok(check(&corrupted)
        .err()
        .ok_or("fabricated phase survived")?
        .to_string())
}
