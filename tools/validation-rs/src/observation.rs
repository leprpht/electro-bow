//! Compare each committed observation against an unbounded chronological history.
pub mod measures;
use crate::{Result, ensure, evidence, process, replay};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, f64::consts::TAU, fs, path::Path, process::Command};

#[derive(Deserialize)]
pub struct Case {
    pub rate: u32,
    pub hz: f64,
    pub speed: f64,
    pub k: f64,
    pub seconds: u32,
    pub trajectory: String,
    pub successful_ticks: usize,
    pub controls_sha256: String,
    pub raw_sha256: String,
    pub wav_sha256: Option<String>,
}

fn frequency(c: &Case, n: usize) -> f64 {
    let t = n as f64 / f64::from(c.rate);
    let cents = match c.trajectory.as_str() {
        "bend" => 200. * (1. - (t - 1.2).abs() / 0.2).max(0.),
        "vibrato" => 35. * (TAU * 5. * t).sin(),
        _ => 0.,
    };
    c.hz * (cents / 1200.).exp2()
}
pub fn controls(c: &Case) -> Vec<u8> {
    let mut controls = Vec::with_capacity(c.successful_ticks * 24);
    for n in 0..c.successful_ticks {
        let k = if c.trajectory == "lift" && n >= c.rate as usize {
            0.
        } else {
            c.k
        };
        for v in [frequency(c, n), c.speed, k] {
            controls.extend(v.to_le_bytes());
        }
    }
    controls
}
fn read(history: &[f64], delay: f64, launch_period: f64, speed: f64) -> f64 {
    let m = delay.floor() as isize;
    let u = delay - m as f64;
    let at = |t: isize| {
        if t >= 0 {
            history[t as usize]
        } else {
            let phase = t as f64 / launch_period + 1. - 0.127;
            -(speed / 0.127) * (phase - phase.floor() - 0.5)
        }
    };
    let newest = history.len() as isize - 1 - m;
    (1. - u) * at(newest) + u * at(newest - 1)
}
// Canonical scipy.io.wavfile IEEE-float mono container with fact chunk.
pub fn float_wav(rate: u32, samples: &[f32]) -> Vec<u8> {
    let frames = samples.len() as u32;
    let data = 4 * frames;
    let mut v = Vec::with_capacity(58 + data as usize);
    v.extend(b"RIFF");
    v.extend((50 + data).to_le_bytes());
    v.extend(b"WAVEfmt ");
    v.extend(18u32.to_le_bytes());
    v.extend(3u16.to_le_bytes());
    v.extend(1u16.to_le_bytes());
    v.extend(rate.to_le_bytes());
    v.extend((4 * rate).to_le_bytes());
    v.extend(4u16.to_le_bytes());
    v.extend(32u16.to_le_bytes());
    v.extend(0u16.to_le_bytes());
    v.extend(b"fact");
    v.extend(4u32.to_le_bytes());
    v.extend(frames.to_le_bytes());
    v.extend(b"data");
    v.extend(data.to_le_bytes());
    for sample in samples {
        v.extend(sample.to_le_bytes());
    }
    v
}
fn grade(
    name: &str,
    c: &Case,
    exe: &Path,
    out: &Path,
    retained: &BTreeMap<String, String>,
) -> Result<Value> {
    let count = c.successful_ticks;
    let rate = f64::from(c.rate);
    let controls = controls(c);
    ensure(
        format!("{:x}", Sha256::digest(&controls)) == c.controls_sha256,
        &format!("{name} control bytes"),
    )?;
    let actual = replay::stream(exe, &[c.rate.to_string(), "--o1".into()], &controls)?;
    let counts: Vec<usize> = std::str::from_utf8(&actual.stderr)?
        .split_whitespace()
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()?;
    ensure(
        counts == [count, 1, 0, 1] && actual.stdout.len() == count * 22 * 8,
        "O1 lifecycle/trace count",
    )?;
    let mut physical = Sha256::new();
    let mut hist = Vec::with_capacity(count);
    let mut audio = Vec::new();
    let mut errors = [0.0_f64; 10];
    let mut peak = 0.0_f64;
    for (n, frame) in actual.stdout.as_chunks::<176>().0.iter().enumerate() {
        physical.update(&frame[..96]);
        let value = |i: usize| f64::from_le_bytes(frame[8 * i..8 * i + 8].try_into().unwrap());
        hist.push(value(3));
        if c.wav_sha256.is_some() && n < (c.seconds * c.rate) as usize {
            audio.push(value(11) as f32);
        }
        let p = rate / frequency(c, n);
        let ray = |d| read(&hist, d, rate / c.hz, c.speed);
        let compare = |i: usize, x: f64, errors: &mut [f64; 10]| -> Result<()> {
            let error = (value(12 + i) - x).abs();
            if error > 1e-9 || !error.is_finite() {
                return Err(
                    format!("{name} tick {n} O1 field {i}: {} vs {x}", value(12 + i)).into(),
                );
            }
            errors[i] = errors[i].max(error);
            Ok(())
        };
        let mut aperture = 0.;
        for (i, s) in [0.162, 0.174, 0.186].into_iter().enumerate() {
            let direct = ray((s - 0.127) * p / 2.);
            let reflected = ray((2. - s - 0.127) * p / 2.);
            let total = direct - reflected;
            for (j, x) in [direct, reflected, total].into_iter().enumerate() {
                compare(3 * i + j, x, &mut errors)?;
            }
            aperture += [1. / 6., 4. / 6., 1. / 6.][i] * total;
        }
        compare(9, aperture, &mut errors)?;
        peak = peak.max(value(21).abs());
        if retained.contains_key(name) {
            ensure(
                (ray((1. - 0.127) * p) + value(0)).abs() <= 1e-12,
                "bow end alignment",
            )?;
        }
    }
    ensure(
        format!("{:x}", physical.finalize()) == c.raw_sha256,
        &format!("{name} physical/M5 hash"),
    )?;
    if let Some(expected) = retained.get(name) {
        ensure(
            format!("{:x}", Sha256::digest(&actual.stdout)) == *expected,
            "retained O1 hash",
        )?;
        fs::write(out.join(format!("{name}.f64le")), &actual.stdout)?;
    }
    if let Some(expected) = &c.wav_sha256 {
        let bytes = float_wav(c.rate, &audio);
        ensure(
            format!("{:x}", Sha256::digest(&bytes)) == *expected,
            "canonical M5 WAV",
        )?;
        fs::write(out.join(format!("{name}.wav")), bytes)?;
    }
    Ok(
        json!({"case":name,"ticks":count,"max_errors_m_s":errors,"peak_m_s":peak,"controls_sha256":c.controls_sha256,"physical_sha256":c.raw_sha256,"wav_sha256":c.wav_sha256,"counters":counts}),
    )
}
pub fn run(out: &Path) -> Result<Value> {
    let cases: BTreeMap<String, Case> =
        serde_json::from_str(include_str!("../contracts/physical-cases.json"))?;
    let retained: BTreeMap<_, _> = include_str!("../../validation/fixtures/o1_sha256.tsv")
        .lines()
        .map(|line| {
            let (digest, f) = line.split_once('\t').unwrap();
            (
                f.strip_suffix(".f64le").unwrap().to_string(),
                digest.to_string(),
            )
        })
        .collect();
    ensure(
        cases.len() == 152 && retained.len() == 15,
        "guide/O1 manifest",
    )?;
    let exe = replay::build(out, None)?;
    let stress = process::check(Command::new(&exe).arg("--stress"))?;
    let mut rows = Vec::new();
    for (name, c) in &cases {
        rows.push(grade(name, c, &exe, out, &retained)?);
    }
    Ok(
        json!({"cases":rows,"measurements":measures::retained(out,&cases,&retained)?,"stress":String::from_utf8(stress.stdout)?,"producer_sha256":evidence::digest(&exe)?}),
    )
}
