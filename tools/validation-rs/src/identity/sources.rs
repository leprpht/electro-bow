//! The six attributed G3 source excerpts. Controlled identity is graded by the separate repeatability suite.
use super::{capture, epoch::Kernel, stationary};
use crate::{
    Result,
    electrical::{acoustic, carrier, scheduled_taps},
    ensure, evidence, numerics,
    recordings::{self, scoring},
    replay,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
pub fn run(out: &Path) -> Result<Value> {
    let original: Value = serde_json::from_str(include_str!("../../contracts/sources.json"))?;
    let exe = replay::build(out, None)?;
    let kernel = Kernel::build(out)?;
    let case = "steady_48000_196_R1.8_V0.1_3s";
    let c = carrier::cases()?.remove(case).ok_or("G3 carrier")?;
    let raw = carrier::render(&exe, &c)?;
    let n = raw.len();
    let fourth = scheduled_taps(
        &exe,
        48000.,
        &raw.iter().map(|r| r[21]).collect::<Vec<_>>(),
        |_| 0.5,
    )?;
    let q = raw
        .iter()
        .zip(fourth)
        .map(|(r, f)| [r[25], r[26], r[27], f[3]])
        .collect::<Vec<_>>();
    let f = stationary::Spectrum::new(
        &raw[n - 48000..].iter().map(|r| r[11]).collect::<Vec<_>>(),
        48000,
    )
    .peak
    .ok_or("M5 anchor")?;
    let mut audition = Vec::new();
    let mut features = BTreeMap::new();
    let mut reports = BTreeMap::new();
    let mut pairs = BTreeMap::new();
    for (name, old) in original["sources"].as_object().unwrap() {
        let file = Path::new("tools/validation/fixtures/acoustic").join(format!("{name}.wav"));
        let (rate, x) = recordings::read(&file)?;
        let attack = x.iter().position(|v| v.abs() > 0.01).ok_or("DI attack")?;
        ensure(
            rate == 48000 && old["attack"] == attack,
            "source attack and native rate",
        )?;
        let x = &x[attack..];
        let computed = capture::updates(x, rate as usize, 196.)?;
        let frozen: capture::Updates = serde_json::from_value(old["schedule"].clone())?;
        ensure(frozen.len() == computed.len(), "source update count")?;
        let mut largest = 0f64;
        for ((t, g), (expected, v)) in computed.iter().zip(&frozen) {
            ensure(t == expected, "source publication tick")?;
            for (a, b) in g.iter().zip(v) {
                largest = largest.max((a - b).abs());
            }
        }
        ensure(largest <= 2e-11, "source DI publication coefficients")?;
        let y = scoring::render_updates(&raw, &q, 196., &kernel, &frozen)?;
        let independent = scoring::render_updates(&raw, &q, 196., &kernel, &computed)?;
        let mut h = Sha256::new();
        let mut error = 0f64;
        for (a, b) in y.iter().zip(independent) {
            h.update(a.map(f64::to_le_bytes).as_flattened());
            for (v, w) in a.iter().zip(b) {
                error = error.max((v - w).abs());
            }
        }
        let sha = format!("{:x}", h.finalize());
        ensure(
            old["wet"] == sha && error <= 2e-11,
            &format!("{name} emitted wet compare"),
        )?;
        let mut depths = vec![];
        for j in 0..3 {
            let row = stationary::grade(&raw, &y, 48000, 196., j)?;
            ensure(
                row["passes"]
                    .as_object()
                    .unwrap()
                    .values()
                    .all(|v| v != false),
                "recorded source V17",
            )?;
            depths.push(row);
        }
        let held = y[n - 48000..].iter().map(|v| v[0]).collect::<Vec<_>>();
        let harmonics = numerics::partials(&held, 48000., f, 12);
        let ftr = scoring::Features::new(
            harmonics
                .iter()
                .map(|v| v.norm())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        );
        let mut errors = (0, 0., 0.);
        acoustic::compare(&old["held"], &ftr.value(), name, &mut errors)?;
        for (other, peer) in &features {
            let peer: &scoring::Features = peer;
            let dist = (peer
                .vector
                .iter()
                .zip(ftr.vector)
                .map(|(v, w)| (v - w).powi(2))
                .sum::<f64>()
                / 12.)
                .sqrt();
            let k = format!("{other}/{name}");
            let reverse = format!("{name}/{other}");
            let (key, sign) = if original["pairs"].get(&k).is_some() {
                (k, 1.)
            } else {
                (reverse, -1.)
            };
            let actual = json!({"distance":dist,"signed":[sign*(peer.cues[0]-ftr.cues[0]),sign*(peer.cues[1]-ftr.cues[1])]});
            acoustic::compare(&original["pairs"][&key], &actual, &key, &mut errors)?;
            pairs.insert(key, actual);
        }
        features.insert(name.clone(), ftr);
        if name.ends_with("-60") {
            audition.push(super::artifacts::array(
                &format!("{name}-held"),
                held.len(),
                None,
                held.iter().copied(),
            )?);
            let di = x[..19200]
                .iter()
                .map(|v| f64::from(*v as f32))
                .collect::<Vec<_>>();
            for (tag, clip) in [("held", held), ("DI", di)] {
                fs::write(
                    out.join(format!("{name}-{tag}.f64le")),
                    clip.into_iter()
                        .flat_map(f64::to_le_bytes)
                        .collect::<Vec<_>>(),
                )?;
            }
        }
        reports.insert(name,json!({"source_sha256":evidence::digest(&file)?,"attack":attack,"publications":computed.len(),"wet_sha256":sha,"max_feature_error":largest,"max_wet_error":error,"depths":depths}));
    }
    crate::electrical::acoustic::listening::canonical_npz(
        &audition,
        &out.join("revised-held.npz"),
    )?;
    ensure(reports.len() == 6 && pairs.len() == 15, "source inventory")?;
    Ok(json!({"sources":reports,"pairs":pairs}))
}
