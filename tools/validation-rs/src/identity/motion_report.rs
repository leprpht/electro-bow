//! Full moving row measurements and explicit upstream eligibility under deliberate corruption.
use super::{epoch::Kernel, legacy, stationary};
use crate::{
    Result, electrical::acoustic, ensure, evidence, guide, observation::Case, publication,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, f64::consts::TAU, fs, path::Path};
fn rms(x: impl Iterator<Item = f64>, n: usize) -> f64 {
    (x.map(|v| v * v).sum::<f64>() / n as f64).sqrt()
}
pub(super) fn prerequisite(raw: &[[f64; 30]], rate: u32, epoch: bool) -> Result<Value> {
    let q = raw
        .iter()
        .map(|r| [r[25], r[26], r[27], r[26]])
        .collect::<Vec<_>>();
    let mut failures = Vec::new();
    let mut rows = Vec::new();
    if !epoch {
        failures.push("invalid epoch".into());
    }
    for j in 0..3 {
        let row = stationary::grade(raw, &q, rate, 196., j)?;
        for k in [
            "bridge_anchor",
            "level",
            "coloration",
            "fundamental",
            "high_note_eta",
            "factory_eta",
            "factory_residual",
        ] {
            if row["passes"][k] == false {
                failures.push(format!("{j}/{k}"));
            }
        }
        rows.push(row);
    }
    Ok(json!({"passed":failures.is_empty(),"failures":failures,"rows":rows}))
}
pub(super) fn defects(raw: &[[f64; 30]], kernel: &Kernel) -> Result<Value> {
    let n = raw.len();
    let rate = 48000.;
    let powers = raw[n - 48000..]
        .iter()
        .map(|r| r[25] * r[25])
        .collect::<Vec<_>>();
    let scale = 2f64.sqrt()
        * (powers
            .chunks(8192)
            .map(crate::numerics::sum_numpy)
            .sum::<f64>()
            / 48000.)
            .sqrt();
    let alt = std::array::from_fn::<_, 12, _>(|j| [-0.4376586748096509, 0.7782794100389229][j % 2]);
    let mut reports = BTreeMap::new();
    for key in [
        "199Hz",
        "dominant-H2",
        "missing-H1",
        "weak-H1",
        "Nyquist",
        "invalid-anchor",
        "ineligible",
        "invalid-epoch",
    ] {
        let mut changed = raw.to_vec();
        let mut frames = Vec::with_capacity(n);
        for (i, r) in changed.iter_mut().enumerate() {
            let h1 = (TAU * 196. * i as f64 / rate).cos();
            let h2 = (TAU * 392. * i as f64 / rate).cos();
            let wave = match key {
                "199Hz" => Some((TAU * 199. * i as f64 / rate).cos()),
                "dominant-H2" => Some(0.5 * h1 + 0.8 * h2),
                "missing-H1" => Some(h2),
                "weak-H1" => Some(0.02 * h1 + h2),
                "Nyquist" => Some(0.5 * h1 + if i % 2 == 0 { 0.8 } else { -0.8 }),
                "invalid-anchor" => {
                    r[11] = scale * h2;
                    None
                }
                "ineligible" => {
                    r[11] = 0.;
                    None
                }
                _ => None,
            };
            if let Some(v) = wave {
                r[25..28].fill(scale * v);
            }
            let mut f = [0.; 20];
            f[..6].copy_from_slice(&[196., r[11], r[25], r[26], r[27], r[26]]);
            f[6..18].copy_from_slice(&alt);
            f[18] = 1.;
            frames.push(f);
        }
        let mut row = prerequisite(&changed, 48000, key != "invalid-epoch")?;
        ensure(row["passed"] == false, "escaped upstream prerequisite")?;
        let mut output = vec![[0.; 20]; n];
        kernel.epoch(rate).run(&frames, &mut output, 0)?;
        ensure(
            output.iter().any(|r| r[..4].iter().any(|v| *v != 0.))
                && frames.iter().zip(&output).all(|(f, y)| {
                    y[8] == 0. && (0..4).all(|j| f[2 + j].to_bits() == y[4 + j].to_bits())
                }),
            "failed prerequisite colored",
        )?;
        row["exact_neutral"] = true.into();
        if key == "199Hz" {
            for f in &mut frames {
                f[6..18].fill(0.);
                f[6] = (std::f64::consts::LN_10 * 2. / 20.).exp_m1();
            }
            kernel.epoch(rate).run(&frames, &mut output, 0)?;
            let old = legacy::scales(&frames, &output, rate)?;
            row["old_scale"] = old[n - 1].into();
            row["old_wet_rms"] = rms(
                (0..n).map(|i| (frames[i][2] + old[i] * output[i][0]) - frames[i][2]),
                n,
            )
            .into();
            row["revised_scale"] = output[n - 1][8].into();
            row["revised_wet_rms"] = rms((0..n).map(|i| output[i][4] - frames[i][2]), n).into();
            ensure(
                old[n - 1] > 0. && row["revised_wet_rms"] == 0.,
                "detuned upstream escape",
            )?;
        }
        reports.insert(key, row);
    }
    Ok(json!(reports))
}
pub(super) fn measures(
    kind: &str,
    c: &Case,
    frames: &[[f64; 20]],
    output: &[[f64; 20]],
    legacy: &[f64],
) -> Result<Value> {
    let n = frames.len();
    let fs = c.rate as usize;
    let mut gap = 0;
    let mut longest = 0;
    for y in output {
        if y[8] == 0. {
            gap += 1;
            longest = longest.max(gap);
        } else {
            gap = 0;
        }
    }
    let mut row = json!({"samples":n,"delta_rms":std::array::from_fn::<_,4,_>(|j|rms((0..n).map(|i|output[i][4+j]-frames[i][2+j]),n)),"old_rms":rms((0..n).map(|i|(frames[i][2]+legacy[i]*output[i][0])-frames[i][2]),n),
        "active_samples":(0..n).filter(|&i|output[i][4]!=frames[i][2]).count() as f64/n as f64,"max_step":output.windows(2).flat_map(|w|(4..8).map(move |j|(w[1][j]-w[0][j]).abs())).fold(0.,f64::max),"peak":output.iter().flat_map(|r|&r[4..8]).map(|v|v.abs()).fold(0.,f64::max),"paired_ratio":rms(output.iter().map(|r|r[5]-r[4]),n)/rms(output.iter().map(|r|r[4]),n),"longest_neutral_samples":longest});
    if matches!(kind, "vibrato" | "bend") {
        row["tracking"] =
            guide::phase::auxiliary(c, &output.iter().map(|r| r[4]).collect::<Vec<_>>())?;
        row["neutral_tracking"] =
            guide::phase::auxiliary(c, &frames.iter().map(|r| r[2]).collect::<Vec<_>>())?;
        if kind == "vibrato" {
            row["delivery_tracking"] =
                guide::phase::auxiliary(c, &output.iter().map(|r| r[7]).collect::<Vec<_>>())?;
        }
    }
    if kind == "veto" {
        row["recovery_samples"] = output[74400..]
            .iter()
            .position(|r| r[8] > 0.)
            .ok_or("recovery samples")?
            .into();
    }
    if kind == "lift" {
        row["lift_rms"] = json!([
            rms(output[fs / 2..fs].iter().map(|r| r[7]), fs / 2),
            rms(output[n - fs..].iter().map(|r| r[7]), fs)
        ]);
        row["after_release_peak"] = (fs + fs / 10..n)
            .map(|i| output[i][7] * (1. - (i - fs + 1) as f64 / (fs / 10) as f64).clamp(0., 1.))
            .map(f64::abs)
            .fold(0., f64::max)
            .into();
    }
    Ok(row)
}
pub fn compare(reference: &Path, root: &Path) -> Result<Value> {
    let old = evidence::strict(&fs::read(reference.join("checkout/build/i1/motion.json"))?)?;
    let fresh = evidence::strict(&fs::read(publication::current_artifact(
        root,
        "i1-motion",
        "evidence.json",
    )?)?)?;
    let mut stats = (0, 0., 0.);
    let actual = &fresh["results"]["cases"];
    for (name, expected) in old["rows"].as_object().ok_or("historical motion rows")? {
        let r = &actual[name];
        let mut row = r["measurements"].clone();
        if let Some(v) = r.get("historical_stationary_calibration") {
            row["reference"] = v.clone();
        }
        for (a, b) in [
            ("off", "witness"),
            ("delivery", "delivery_witness"),
            ("old_off", "old_witness"),
        ] {
            if let Some(v) = r.get(a) {
                row[b] = json!({"active":v["active"],"passed":v["passed"],"reference":v["stationary_reference"],"longest_gap_ms":v["longest_gap_ms"],"cell_rms":v["rms"]});
            }
        }
        acoustic::compare(expected, &row, name, &mut stats)?;
    }
    Ok(
        json!({"cases":27,"fields":stats.0,"max_scaled_error":stats.1,"max_phase_error_deg":stats.2}),
    )
}
