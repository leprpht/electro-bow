//! Evidence consumers bind mutable claims to raw samples captured before measurement.
use super::carrier;
use crate::{Result, ensure, evidence, identity::stationary, replay};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, f64::consts::TAU, path::Path};
fn digest(samples: impl Iterator<Item = f64>) -> String {
    let mut h = Sha256::new();
    for v in samples {
        h.update(v.to_le_bytes());
    }
    format!("{:x}", h.finalize())
}
#[derive(Clone, PartialEq)]
struct Clock {
    source: String,
    rate: u32,
    pitch: f64,
    epoch: u64,
    start: usize,
    end: usize,
    count: usize,
    window: [usize; 2],
    settings: String,
    m5: String,
    off: String,
}
struct Producer {
    clock: Clock,
    taps: BTreeMap<usize, String>,
    delivered: [String; 2],
}
#[derive(Clone)]
struct Claim {
    clock: Clock,
    depth: f64,
    samples: Vec<f64>,
}
fn input_schema(rows: &[[f64; 30]], taps: &BTreeMap<usize, String>) -> Result<()> {
    let actual: BTreeMap<_, _> = [3, 11, 21, 22, 24, 25, 26, 27]
        .map(|k| (k, digest(rows.iter().map(|r| r[k]))))
        .into();
    ensure(&actual == taps, "canonical Rust tap schema")
}
fn pair(
    truth: &Producer,
    clock: &Clock,
    off: Option<&Claim>,
    on: Option<&Claim>,
    factory: &[f64],
) -> Result<()> {
    let (off, on) = off.zip(on).ok_or("paired final wet traces required")?;
    ensure(
        clock == &truth.clock
            && off.clock == truth.clock
            && on.clock == truth.clock
            && off.depth == 0.
            && on.depth == 0.5
            && off.samples.len() == truth.clock.count
            && on.samples.len() == truth.clock.count
            && digest(factory.iter().copied()) == truth.taps[&26],
        "final wet alignment/settings/reference differ",
    )?;
    ensure(
        digest(off.samples.iter().copied()) == truth.delivered[0]
            && digest(on.samples.iter().copied()) == truth.delivered[1],
        "synchronized wet producer samples",
    )
}
pub fn run(dir: &Path) -> Result<Value> {
    let name = "steady_48000_196_R1.8_V0.1_3s";
    let c = carrier::cases()?.remove(name).ok_or("196 Hz fixture")?;
    let exe = replay::build(dir, None)?;
    let raw = carrier::render(&exe, &c)?;
    let n = raw.len();
    let mut h = Sha256::new();
    for r in &raw {
        h.update(r.map(f64::to_le_bytes).as_flattened());
    }
    let source = format!("{:x}", h.finalize());
    ensure(
        source == carrier::identities()[name][2],
        "frozen carrier digest",
    )?;
    let taps: BTreeMap<_, _> = [3, 11, 21, 22, 24, 25, 26, 27]
        .map(|k| (k, digest(raw.iter().map(|r| r[k]))))
        .into();
    input_schema(&raw, &taps)?;
    let clock = Clock {
        source,
        rate: 48000,
        pitch: 196.,
        epoch: 1,
        start: 0,
        end: n,
        count: n,
        window: [n - 48000, n],
        settings: "unity".into(),
        m5: taps[&11].clone(),
        off: taps[&25].clone(),
    };
    let off: Vec<_> = raw.iter().map(|r| r[25]).collect();
    let factory: Vec<_> = raw.iter().map(|r| r[26]).collect();
    let tone: Vec<_> = (0..n)
        .map(|i| (TAU * 6151. * i as f64 / 48000.).sin())
        .collect();
    let rms = |x: &[f64]| (x[n - 48000..].iter().map(|v| v * v).sum::<f64>() / 48000.).sqrt();
    let gain = 0.1 * rms(&off) / rms(&tone);
    let colored: Vec<_> = off.iter().zip(tone).map(|(x, v)| x + v * gain).collect();
    let producer = Producer {
        clock: clock.clone(),
        taps: taps.clone(),
        delivered: [digest(off.iter().copied()), digest(colored.iter().copied())],
    };
    let a = Claim {
        clock: clock.clone(),
        depth: 0.,
        samples: off,
    };
    let b = Claim {
        clock: clock.clone(),
        depth: 0.5,
        samples: colored,
    };
    pair(&producer, &clock, Some(&a), Some(&b), &factory)?;
    let mut verdict = BTreeMap::new();
    let mut reject = |name: &str, reason: &str, r: Result<()>| -> Result<()> {
        let err = r
            .err()
            .ok_or_else(|| format!("surviving {name}"))?
            .to_string();
        ensure(err == reason, &format!("{name}: {err}"))?;
        verdict.insert(name.to_owned(), json!({"status":1,"first":err}));
        Ok(())
    };
    for name in [
        "incorrect underlying factory bytes",
        "off only",
        "factory only",
        "wrong source",
        "incorrect count",
        "unaligned wet pair",
        "unmatched wet settings",
        "wrong epoch",
        "wrong reference",
        "wrong measurement window",
        "joint shift",
        "self-attested shift",
        "rolled wet valid tags",
    ] {
        let mut a = a.clone();
        let mut b = b.clone();
        let mut supplied = clock.clone();
        let mut factory = factory.clone();
        let expected = match name {
            "off only" | "factory only" => "paired final wet traces required",
            "rolled wet valid tags" => "synchronized wet producer samples",
            _ => "final wet alignment/settings/reference differ",
        };
        match name {
            "incorrect underlying factory bytes" => factory.rotate_right(1),
            "wrong source" => a.clock.source = "other recording".into(),
            "incorrect count" => a.clock.count -= 1,
            "unaligned wet pair" => a.clock.start = 1,
            "unmatched wet settings" => a.clock.settings = "other gain".into(),
            "wrong epoch" => a.clock.epoch = 2,
            "wrong reference" => a.clock.m5 = a.clock.m5.chars().rev().collect(),
            "wrong measurement window" => a.clock.window = [1, 48000],
            "joint shift" => {
                a.clock.start = 1;
                a.clock.end = 48001;
                a.clock.window = [1, 48001];
            }
            "self-attested shift" => {
                a.clock.start += 1;
                a.clock.end += 1;
                a.clock.window = [n - 48000 + 1, n + 1];
                supplied = a.clock.clone();
            }
            "rolled wet valid tags" => {
                a.samples.rotate_right(1);
                b.samples.rotate_right(1);
            }
            _ => {}
        }
        b.clock = a.clock.clone();
        reject(
            name,
            expected,
            pair(
                &producer,
                &supplied,
                (name != "factory only").then_some(&a),
                (name != "off only").then_some(&b),
                &factory,
            ),
        )?;
    }
    let mut spoof = taps;
    spoof.insert(11, spoof[&25].clone());
    reject(
        "substituted original M5",
        "canonical Rust tap schema",
        input_schema(&raw, &spoof),
    )?;
    let wet: Vec<_> = a
        .samples
        .iter()
        .zip(&b.samples)
        .map(|(x, y)| [*x, *y, *x, *y])
        .collect();
    let row = stationary::grade(&raw, &wet, 48000, 196., 1)?;
    ensure(
        (row["factory_difference_over_off"]
            .as_f64()
            .ok_or("factory residual")?
            + 20.)
            .abs()
            < 1e-9
            && row["local_q_difference_over_off"].as_f64().unwrap() < -28.,
        "colored wet control setup",
    )?;
    reject(
        "colored final wet",
        "final wet residual -20 dB",
        ensure(
            row["passes"]["factory_residual"] == true,
            "final wet residual -20 dB",
        ),
    )?;
    Ok(
        json!({"producer":evidence::digest(&exe)?,"source":clock.source,"taps":producer.taps,"delivered":producer.delivered,"controls":verdict,"case":name,"count":n,"window":clock.window}),
    )
}
