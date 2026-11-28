//! Offline four-path independent correction bank. State belongs to one launch epoch.
pub mod capture;
pub mod deadlines;
pub mod epoch;
pub mod layout;
pub mod motion;
pub mod paths;
pub mod qr;
pub mod stationary;
use crate::{Result, ensure, evidence};
use rustfft::num_complex::Complex64;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    f64::consts::TAU,
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

pub struct Bank {
    rate: f64,
    pitch: f64,
    radius: f64,
    poles: [Complex64; 12],
    taper: [f64; 12],
    first: [[Complex64; 12]; 4],
    second: [[Complex64; 12]; 4],
}
impl Bank {
    pub fn new(rate: f64) -> Result<Self> {
        ensure(
            rate.is_finite() && (32000.0..=192000.0).contains(&rate),
            "unsupported bank sample rate",
        )?;
        Ok(Self {
            rate,
            pitch: 0.,
            radius: 0.,
            poles: [Complex64::ZERO; 12],
            taper: [0.; 12],
            first: [[Complex64::ZERO; 12]; 4],
            second: [[Complex64::ZERO; 12]; 4],
        })
    }
    pub fn apply(&mut self, frame: &[f64; 17]) -> Result<[f64; 6]> {
        let f = frame[0];
        ensure(
            frame.iter().all(|v| v.is_finite()) && f > 0.,
            "nonfinite bank input or invalid pitch",
        )?;
        if f != self.pitch {
            self.pitch = f;
            self.radius = (-TAU * 0.22 * f / self.rate).exp();
            let analysis = self.rate / (self.rate / 16000.).floor();
            for k in 0..12 {
                let order = (k + 1) as f64;
                self.poles[k] = Complex64::from_polar(self.radius, TAU * order * f / self.rate);
                self.taper[k] = ((0.28 * analysis - order * f) / (0.03 * analysis)).clamp(0., 1.);
            }
        }
        let mut output = [0.0_f64; 6];
        for j in 0..4 {
            let x = frame[j + 1];
            output[5] = output[5].max(x.abs());
            for k in 0..12 {
                let z = self.poles[k] * self.first[j][k] + (1. - self.radius) * x;
                let w = self.poles[k] * self.second[j][k] + (1. - self.radius) * z;
                self.first[j][k] = z;
                self.second[j][k] = w;
                output[j] += frame[5 + k] * self.taper[k] * 2. * w.re;
                output[4] = output[4].max(z.norm()).max(w.norm());
            }
        }
        ensure(
            output.iter().all(|v| v.is_finite()),
            "nonfinite bank output",
        )?;
        Ok(output)
    }
}
fn next_frame<const N: usize>(r: &mut impl Read) -> Result<Option<[f64; N]>> {
    let mut bytes = [0u8; 8];
    if r.read(&mut bytes[..1])? == 0 {
        return Ok(None);
    }
    r.read_exact(&mut bytes[1..])?;
    let mut frame = [0.; N];
    frame[0] = f64::from_le_bytes(bytes);
    for v in &mut frame[1..] {
        r.read_exact(&mut bytes)?;
        *v = f64::from_le_bytes(bytes);
    }
    Ok(Some(frame))
}
pub fn stream(rate: f64) -> Result<()> {
    let mut input = BufReader::new(std::io::stdin().lock());
    let mut out = BufWriter::new(std::io::stdout().lock());
    let mut bank = Bank::new(rate)?;
    while let Some(frame) = next_frame(&mut input)? {
        for x in bank.apply(&frame)? {
            out.write_all(&x.to_le_bytes())?;
        }
    }
    out.flush()?;
    Ok(())
}
#[derive(Deserialize)]
struct Witness {
    cell_rms: Vec<f64>,
    active: usize,
    longest_gap_ms: usize,
    reference: f64,
    passed: bool,
}
#[derive(Deserialize)]
struct Case {
    case: String,
    frames: usize,
    rate: u32,
    input: String,
    output: String,
    difference: Option<String>,
    witness: Option<Witness>,
    delivery_witness: Option<Witness>,
}
fn witness(x: &[[f64; 2]], fs: u32, column: usize, old: &Witness) -> Result<Value> {
    let edge = |i: usize| ((0.5 + i as f64 * 0.02) * f64::from(fs)).round_ties_even() as usize;
    let mut cells = Vec::new();
    let (mut active, mut gap, mut longest) = (0, 0, 0);
    ensure(
        old.cell_rms.len() == 100 && edge(100) <= x.len(),
        "motion window inventory",
    )?;
    for i in 0..100 {
        let window = &x[edge(i)..edge(i + 1)];
        let rms = (window.iter().map(|v| v[column] * v[column]).sum::<f64>() / window.len() as f64)
            .sqrt();
        ensure((rms - old.cell_rms[i]).abs() <= 2e-11, "motion cell RMS")?;
        if rms >= 0.5 * old.reference {
            active += 1;
            gap = 0;
        } else {
            gap += 1;
            longest = longest.max(gap);
        }
        cells.push(rms);
    }
    let passed = old.reference > 1e-4 && active >= 75 && longest <= 3;
    ensure(
        active == old.active && longest * 20 == old.longest_gap_ms && passed == old.passed,
        "motion witness disposition",
    )?;
    Ok(
        json!({"rms":cells,"active":active,"longest_gap_ms":longest*20,"passed":passed,"stationary_reference":old.reference}),
    )
}

pub fn compare(root: &Path) -> Result<Value> {
    let dir = root.join("bank");
    let manifest = dir.join("manifest.json");
    ensure(
        evidence::digest(&manifest)?
            == "673b5673d3f9e75f5786a4ab39307f2a88f218d1b27829798a7bb3add253bc9b",
        "frozen C++ bank manifest",
    )?;
    let cases: Vec<Case> = serde_json::from_value(evidence::strict(&fs::read(&manifest)?)?)?;
    let mut expected = std::collections::BTreeSet::new();
    for fs in [32000, 44100, 48000, 88200, 96000, 176400, 192000] {
        for k in ["steady", "bend", "vibrato"] {
            expected.insert(format!("{k}-{fs}"));
        }
    }
    for k in [
        "fast-retune",
        "max-retune",
        "bow-controls",
        "veto",
        "lift",
        "held",
    ] {
        expected.insert(format!("{k}-48000"));
    }
    let mut rows = Vec::new();
    for c in cases {
        ensure(
            expected.remove(&c.case) && c.case.ends_with(&format!("-{}", c.rate)),
            "I1 motion case inventory",
        )?;
        let frames = c.rate as usize * if c.case.starts_with("lift-") { 7 } else { 3 };
        ensure(c.frames == frames, "registered bank frame count")?;
        let input = dir.join(format!("{}.in", c.case));
        let output = dir.join(format!("{}.out", c.case));
        ensure(
            evidence::digest(&input)? == c.input && evidence::digest(&output)? == c.output,
            "frozen bank stream digest",
        )?;
        let mut data = BufReader::new(fs::File::open(input)?);
        let mut old = BufReader::new(fs::File::open(output)?);
        let mut bank = Bank::new(f64::from(c.rate))?;
        let mut errors = [0.0_f64; 6];
        let mut peak = 0.0_f64;
        for n in 0..frames {
            let y = bank.apply(&next_frame(&mut data)?.ok_or("truncated bank input")?)?;
            let reference = next_frame::<6>(&mut old)?.ok_or("truncated C++ comparison")?;
            for j in 0..6 {
                let e = (y[j] - reference[j]).abs();
                if !(0.0..=2e-11).contains(&e) {
                    return Err(format!(
                        "{} frame {n} field {j}: {} vs {}",
                        c.case, y[j], reference[j]
                    )
                    .into());
                }
                errors[j] = errors[j].max(e);
            }
            peak = peak.max(y[5]);
            ensure(y[4] <= peak + 1e-12, "bank numerical state")?;
        }
        ensure(
            next_frame::<17>(&mut data)?.is_none() && next_frame::<6>(&mut old)?.is_none(),
            "trailing bank frames",
        )?;
        let mut row = json!({"case":c.case,"frames":frames,"max_errors":errors,"input":c.input,"frozen_output":c.output});
        if c.case.starts_with("vibrato-") {
            let diff = dir.join(format!("{}.difference", c.case));
            ensure(
                Some(evidence::digest(&diff)?) == c.difference,
                "wet difference bytes",
            )?;
            let mut r = BufReader::new(fs::File::open(diff)?);
            let mut samples = Vec::with_capacity(frames);
            while let Some(pair) = next_frame::<2>(&mut r)? {
                samples.push(pair);
            }
            ensure(samples.len() == frames, "wet difference sample count")?;
            row["off_witness"] = witness(
                &samples,
                c.rate,
                0,
                c.witness.as_ref().ok_or("missing off witness")?,
            )?;
            row["delivery_witness"] = witness(
                &samples,
                c.rate,
                1,
                c.delivery_witness
                    .as_ref()
                    .ok_or("missing delivery witness")?,
            )?;
        }
        rows.push(row);
    }
    ensure(expected.is_empty(), "missing I1 bank case")?;
    Ok(json!({"cases":rows,"reference_manifest":evidence::digest(&manifest)?}))
}
