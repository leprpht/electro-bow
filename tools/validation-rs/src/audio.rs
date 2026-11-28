use crate::{Result, ensure, evidence};
use hound::{SampleFormat, WavReader};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Deserialize)]
struct Fixture {
    path: String,
    sha256: String,
    decoded: String,
    rate: u32,
    frames: u32,
    channels: u16,
    dtype: String,
}

pub fn fixtures() -> Result<Value> {
    let expected: Vec<Fixture> = serde_json::from_value(evidence::strict(include_bytes!(
        "../reference/acoustic.json"
    ))?)?;
    let sums = fs::read_to_string("tools/validation/fixtures/acoustic/SHA256SUMS")?;
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    ensure(
        expected.len() == 25 && sums.lines().count() == expected.len(),
        "canonical acoustic inventory",
    )?;
    for (f, line) in expected.iter().zip(sums.lines()) {
        ensure(
            seen.insert(&f.path) && line == format!("{}  {}", f.sha256, f.path),
            "acoustic manifest row",
        )?;
        let path = Path::new(&f.path);
        ensure(
            evidence::digest(path)? == f.sha256,
            &format!("container SHA-256 {}", f.path),
        )?;
        let mut reader = WavReader::open(path)?;
        let spec = reader.spec();
        ensure(
            (spec.sample_rate, spec.channels, reader.duration()) == (f.rate, f.channels, f.frames),
            "WAV metadata",
        )?;
        let mut hash = Sha256::new();
        let mut record = |sample: f64| -> Result<()> {
            ensure(sample.is_finite(), "nonfinite WAV")?;
            hash.update((if sample == 0.0 { 0.0 } else { sample }).to_le_bytes());
            Ok(())
        };
        let dtype = match spec.sample_format {
            SampleFormat::Float => {
                for sample in reader.samples::<f32>() {
                    record(f64::from(sample?))?;
                }
                "float32"
            }
            SampleFormat::Int => {
                let scale = 2f64.powi(i32::from(spec.bits_per_sample) - 1);
                for sample in reader.samples::<i32>() {
                    record(f64::from(sample?) / scale)?;
                }
                match spec.bits_per_sample {
                    1..=8 => "uint8",
                    9..=16 => "int16",
                    17..=32 => "int32",
                    _ => return Err("unsupported PCM".into()),
                }
            }
        };
        let decoded = format!("{:x}", hash.finalize());
        ensure(
            dtype == f.dtype && decoded == f.decoded,
            &format!("decoded samples {}", f.path),
        )?;
        rows.push(json!({"path":f.path,"frames":f.frames,"rate":f.rate,"channels":f.channels,"dtype":dtype,"decoded":decoded,"sha256":f.sha256}));
    }
    Ok(rows.into())
}
