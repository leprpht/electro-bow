//! Reproduce the earlier fixed-bin research study independently of the revised moving monitor.
use super::{Bank, capture, legacy, motion};
use crate::{
    Result,
    electrical::{acoustic, carrier},
    ensure, evidence, guide, numerics, observation, publication, recordings, replay,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
const RATE: usize = 48000;
const ALTERNATING: [f64; 2] = [-0.4376586748096509, 0.7782794100389229];
mod sources;
mod stationary;
type Channels = [f64; 4];
struct Measured {
    wet: Vec<Channels>,
    final_coeff: [f64; 12],
    unsafe_off: Vec<f64>,
    scale: Vec<f64>,
    log: Vec<Value>,
}
fn render(
    raw: &[[f64; 30]],
    pitch: &[f64],
    updates: Option<&capture::Updates>,
    unavailable: bool,
    rate: u32,
    constant: [f64; 12],
) -> Result<Measured> {
    let mut bank = Bank::new(rate.into())?;
    let mut frames = Vec::with_capacity(raw.len());
    let mut corrections = Vec::with_capacity(raw.len());
    let mut events = updates.into_iter().flatten().peekable();
    let mut target = [0.; 12];
    let mut z = [0.; 12];
    let smoother = -(-1. / (0.015 * f64::from(rate))).exp_m1();
    for (i, (r, &f)) in raw.iter().zip(pitch).enumerate() {
        let mut row = [0.; 20];
        row[0] = f;
        row[1] = r[11];
        row[2..5].copy_from_slice(&r[25..28]);
        row[5] = r[26];
        while events.peek().is_some_and(|(tick, _)| *tick <= i) {
            target = events.next().unwrap().1;
        }
        for j in 0..12 {
            row[6 + j] = if updates.is_some() {
                let y = smoother.mul_add(target[j], z[j]);
                z[j] = (1. - smoother) * y;
                y
            } else {
                constant[j]
            };
        }
        row[18] = f64::from(!(unavailable && (64800..74400).contains(&i)));
        row[19] = 1.;
        let mut input = [0.; 17];
        input[0] = f;
        input[1..].copy_from_slice(&row[2..18]);
        let mut delta = [0.; 20];
        delta[..4].copy_from_slice(&bank.apply(&input)?[..4]);
        frames.push(row);
        corrections.push(delta);
    }
    let final_coeff = frames.last().ok_or("empty study")?[6..18].try_into()?;
    let unsafe_off = frames
        .iter()
        .zip(&corrections)
        .map(|(r, d)| r[2] + d[0])
        .collect();
    let (scale, log) = legacy::monitor(&frames, &corrections, rate.into())?;
    let wet = frames
        .iter()
        .zip(corrections)
        .zip(&scale)
        .map(|((r, d), &s)| {
            std::array::from_fn(|j| {
                if s == 0. {
                    r[2 + j]
                } else {
                    r[2 + j] + s * d[j]
                }
            })
        })
        .collect();
    Ok(Measured {
        wet,
        scale,
        log,
        final_coeff,
        unsafe_off,
    })
}
fn policy(data: &Measured) -> Value {
    let n = data.log.len();
    let count =
        |field: &str, expected: bool| data.log.iter().filter(|r| r[field] == expected).count();
    json!({"windows":n,"veto_fraction":count("veto",true) as f64/n as f64,"constraint_fraction":data.log.iter().filter(|r|r["scale"].as_f64().unwrap()<1.).count() as f64/n as f64,"neutral_sample_fraction":data.scale.iter().filter(|&&v|v==0.).count() as f64/data.scale.len() as f64,"upstream_or_uncertified":count("neutral_ok",false),"realized_veto_failures":count("realized_ok",false),"settled_scale":data.scale.last()})
}
fn rms(x: &[f64]) -> f64 {
    let p: Vec<_> = x.iter().map(|v| v * v).collect();
    (p.chunks(8192).map(numerics::sum_numpy).sum::<f64>() / x.len() as f64).sqrt()
}
type Clip = (String, Vec<f64>, std::ops::Range<usize>);
fn dynamics(exe: &Path, clips: &mut Vec<Clip>) -> Result<Value> {
    let cases = carrier::cases()?;
    let mut rows = BTreeMap::new();
    for name in [
        "bend",
        "vibrato",
        "retune",
        "gain-veto",
        "lift",
        "long-hold",
        "bow-controls",
    ] {
        let key = match name {
            "lift" => "lift_48000_82.41_R1.8_V0.1_7s",
            "bend" => "bend_48000_196_R1.8_V0.1_3s",
            "vibrato" => "vibrato_48000_196_R1.8_V0.1_3s",
            _ => "steady_48000_196_R1.8_V0.1_3s",
        };
        let c = &cases[key];
        let (raw, pitch, sha) = if matches!(name, "retune" | "long-hold" | "bow-controls") {
            let (mut pitch, _, mut controls) = motion::schedule(
                if name == "retune" {
                    "fast-retune"
                } else {
                    name
                },
                RATE as u32,
            );
            if name == "long-hold" {
                pitch = pitch.repeat(4);
                controls = controls.repeat(4);
            }
            if name != "bow-controls" {
                for row in controls.as_chunks_mut::<24>().0 {
                    row[16..24].copy_from_slice(&(0.18_f64 / 1.65).to_le_bytes());
                }
            }
            let raw = carrier::scheduled(exe, RATE as u32, pitch.len(), &controls)?;
            let mut h = Sha256::new();
            for r in &raw {
                h.update(r.map(f64::to_le_bytes).as_flattened());
            }
            (raw, pitch, Some(format!("{:x}", h.finalize())))
        } else {
            let raw = carrier::render(exe, c)?;
            let pitch = (0..raw.len())
                .map(|i| observation::frequency(c, i))
                .collect();
            (raw, pitch, None)
        };
        let veto = name == "gain-veto";
        let events = vec![
            (0, std::array::from_fn(|j| ALTERNATING[j % 2])),
            (RATE, std::array::from_fn(|j| ALTERNATING[(j + 1) % 2])),
            (2 * RATE, std::array::from_fn(|j| ALTERNATING[j % 2])),
        ];
        let data = render(
            &raw,
            &pitch,
            veto.then_some(&events),
            veto,
            RATE as u32,
            std::array::from_fn(|j| ALTERNATING[j % 2]),
        )?;
        let off: Vec<_> = data.wet.iter().map(|r| r[0]).collect();
        let diff: Vec<_> = off.iter().zip(&raw).map(|(v, r)| v - r[25]).collect();
        let peak = data
            .wet
            .iter()
            .flat_map(|r| &r[..3])
            .map(|v| v.abs())
            .fold(0., f64::max);
        let mut row = json!({"policy":policy(&data),"peak":peak,"delta_rms":rms(&diff),"timeline":data.log,"sha256":sha});
        if matches!(name, "bend" | "vibrato") {
            row["tracking"] = guide::phase::auxiliary(c, &off)?;
        }
        if name == "lift" {
            let before = rms(&off[RATE / 2..RATE]);
            let after = rms(&off[6 * RATE..7 * RATE]);
            ensure(after < before, "legacy lift decay")?;
            row["before_after"] = json!([before, after]);
            row["released_peak_after_zero"] = json!(
                off.iter()
                    .enumerate()
                    .skip(RATE + RATE / 10)
                    .map(|(i, v)| v
                        * (1. - (i - RATE + 1) as f64 / (RATE / 10) as f64).clamp(0., 1.))
                    .map(f64::abs)
                    .fold(0., f64::max)
            );
        }
        if veto {
            let recovered = data.scale[RATE / 2..RATE].iter().any(|v| *v > 0.)
                && data.scale[69600..74400].contains(&0.)
                && data.scale[2 * RATE..].iter().any(|v| *v > 0.);
            ensure(recovered, "legacy veto recovery")?;
            row["veto_recovered"] = recovered.into();
        }
        if matches!(name, "bend" | "gain-veto") {
            clips.push((name.into(), off[..2 * RATE].to_vec(), RATE / 2..RATE));
        }
        rows.insert(name, row);
    }
    Ok(json!(rows))
}
pub fn run(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let mut clips = Vec::new();
    let real = sources::run(&exe, &mut clips)?;
    let grid = stationary::run(&exe)?;
    let inversion = sources::inversion(&exe)?;
    let dynamic = dynamics(&exe, &mut clips)?;
    let mut entries = Vec::new();
    let mut gains = Vec::new();
    for (name, x, anchor) in clips {
        let gain = 0.1 / rms(&x[anchor]);
        gains.push(format!("  \"{name}\": {}", serde_json::to_string(&gain)?));
        for (dir, g) in [("engineering", 1.), ("matched", gain)] {
            let alias = match (dir, name.as_str()) {
                ("matched", "techs-P1-60-held") => "held-B",
                ("matched", "techs-P2-60-held") => "held-A",
                _ => &name,
            };
            let bytes = observation::float_wav(
                RATE as u32,
                &x.iter().map(|v| (g * v) as f32).collect::<Vec<_>>(),
            );
            entries.push((format!("{dir}/{alias}.wav"), bytes));
        }
    }
    entries.push((
        "listening-gains.json".into(),
        format!("{{\n{}\n}}", gains.join(",\n")).into_bytes(),
    ));
    entries.push((
        "ATTRIBUTION.md".into(),
        fs::read("tools/validation/fixtures/acoustic/ATTRIBUTION.md")?,
    ));
    entries.push(("LISTEN.txt".into(),b"Compare the two G3 DI attacks to the held tones at fixed playback gain. Which source matches each held tone, and by which cue? Compare the matched neutral tone. Hear the bend and the monitor fallback. Record route, timestamp, cue and indistinguishable outcomes. Engineering files retain unity gain. No listening verdict recorded.\n".to_vec()));
    let mut members = BTreeMap::new();
    for (name, data) in &entries {
        members.insert(name, format!("{:x}", Sha256::digest(data)));
    }
    acoustic::listening::canonical_archive(&entries, &out.join("listening.zip"))?;
    let zip = evidence::digest(&out.join("listening.zip"))?;
    Ok(
        json!({"dynamic":dynamic,"real":real,"grid":grid,"inversion":inversion,"archive":zip,"members":members}),
    )
}
pub fn differential(reference: &Path, root: &Path) -> Result<Value> {
    let old = evidence::strict(&fs::read(
        reference.join("checkout/build/i1/i1-results.json"),
    )?)?;
    let path = publication::current_artifact(root, "i1-study", "evidence.json")?;
    let measured = evidence::strict(&fs::read(path)?)?;
    let result = &measured["results"];
    let mut stats = (0, 0., 0.);
    acoustic::compare(&old["dynamic"], &result["dynamic"], "dynamic", &mut stats)?;
    for key in ["real", "inversion"] {
        acoustic::compare(&old[key], &result[key], key, &mut stats)?;
    }
    for key in [
        "stationary",
        "complex",
        "expression",
        "corners",
        "measured_sweep",
    ] {
        acoustic::compare(&old[key], &result["grid"][key], key, &mut stats)?;
    }
    let historical = reference.join("checkout/build/i1/listening.zip");
    let fresh = publication::current_artifact(root, "i1-study", "listening.zip")?;
    let mut old_zip = zip::ZipArchive::new(fs::File::open(&historical)?)?;
    let mut new_zip = zip::ZipArchive::new(fs::File::open(&fresh)?)?;
    ensure(
        old_zip.len() == 17 && new_zip.len() == old_zip.len(),
        "legacy audition inventory",
    )?;
    let mut gain_distances = BTreeMap::new();
    use std::io::Read;
    for i in 0..old_zip.len() {
        let mut a = old_zip.by_index(i)?;
        let mut b = new_zip.by_index(i)?;
        ensure(a.name() == b.name(), "legacy audition entry order")?;
        let mut old_bytes = Vec::new();
        a.read_to_end(&mut old_bytes)?;
        let mut new_bytes = Vec::new();
        b.read_to_end(&mut new_bytes)?;
        if a.name() == "listening-gains.json" {
            let expected: BTreeMap<String, f64> =
                serde_json::from_value(evidence::strict(&old_bytes)?)?;
            let actual: BTreeMap<String, f64> =
                serde_json::from_value(evidence::strict(&new_bytes)?)?;
            ensure(
                expected.len() == 7 && expected.keys().eq(actual.keys()),
                "audition gain coverage",
            )?;
            for (k, v) in expected {
                let x = actual[&k];
                ensure(
                    x.is_finite() && x > 0. && (v - x).abs() <= 2e-11,
                    "legacy measured matching gain",
                )?;
                gain_distances.insert(k, v.to_bits().abs_diff(x.to_bits()));
            }
        } else {
            ensure(
                old_bytes == new_bytes,
                &format!("legacy audition {}", a.name()),
            )?;
        }
    }
    Ok(
        json!({"dynamic_cases":7,"fields":stats.0,"largest_scaled_error":stats.1,
        "entry_zip_sha256":evidence::digest(&historical)?,"rust_zip_sha256":result["archive"],
        "bitwise_wavs":14,"bitwise_attribution_and_instructions":true,"gain_ulp_distances":gain_distances}),
    )
}
