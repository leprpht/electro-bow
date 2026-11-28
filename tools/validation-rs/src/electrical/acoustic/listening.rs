use crate::{Result, ensure, evidence, numerics::sum_numpy, observation::float_wav};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{Cursor, Read, Write},
    path::Path,
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};
type Sources = BTreeMap<(String, String), Vec<f64>>;
const LISTEN: &str = "Hear the physical low-E stroke, then matched A/B at low, middle and high pitch. Which resembles bowed electric guitar? Which buzzes less? Mark any whistle, resonance or objectionable harmonic with time. Is the 440 Hz detail pair distinguishable and useful? Is the sustained note and lift musically usable? Neither and indistinguishable are valid. Record listener, date, route and failed excerpt before opening the separate listening-key.json. Engineering WAVs preserve common gain. No effects.\n";

// Keep the reviewed Python ZipInfo header encoding. zlib supplies the real compressed stream.
fn canonical_archive(entries: &[(String, Vec<u8>)], path: &Path) -> Result<()> {
    let mut zipped = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(6));
    for (name, data) in entries {
        zipped.start_file(name, options)?;
        zipped.write_all(data)?;
    }
    let mut archive = ZipArchive::new(zipped.finish()?)?;
    let mut data = Vec::new();
    let mut central = Vec::new();
    for i in 0..archive.len() {
        let mut member = archive.by_index_raw(i)?;
        let name = member.name().as_bytes().to_vec();
        let offset = u32::try_from(data.len())?;
        let crc = member.crc32();
        let cs = u32::try_from(member.compressed_size())?;
        let sz = u32::try_from(member.size())?;
        let len = u16::try_from(name.len())?;
        data.extend(0x04034b50u32.to_le_bytes());
        central.extend(0x02014b50u32.to_le_bytes());
        central.extend(0x0314u16.to_le_bytes());
        for b in [&mut data, &mut central] {
            for v in [20u16, 0, 8, 0, 33] {
                b.extend(v.to_le_bytes());
            }
            for v in [crc, cs, sz] {
                b.extend(v.to_le_bytes());
            }
            b.extend(len.to_le_bytes());
            b.extend(0u16.to_le_bytes());
        }
        data.extend(&name);
        member.read_to_end(&mut data)?;
        for v in [0u16, 0, 0] {
            central.extend(v.to_le_bytes());
        }
        central.extend((0o600u32 << 16).to_le_bytes());
        central.extend(offset.to_le_bytes());
        central.extend(&name);
    }
    let offset = u32::try_from(data.len())?;
    let size = u32::try_from(central.len())?;
    let count = u16::try_from(entries.len())?;
    data.extend(central);
    data.extend(0x06054b50u32.to_le_bytes());
    for v in [0u16, 0, count, count] {
        data.extend(v.to_le_bytes());
    }
    data.extend(size.to_le_bytes());
    data.extend(offset.to_le_bytes());
    data.extend(0u16.to_le_bytes());
    fs::write(path, data)?;
    Ok(())
}
pub fn write(dir: &Path, sources: &Sources) -> Result<Value> {
    let mut entries = vec![];
    let mut key = vec![];
    let mut gains_json = vec![];
    let mut records = BTreeMap::new();
    // Frozen seed-603 blinded schedule, in original archive order. No audio measurements are stored here.
    for (pair, labels) in [
        ("82.41Hz", &["O2", "M5"][..]),
        ("196Hz", &["O2", "M5"]),
        ("440Hz", &["M5", "O2"]),
        ("detail", &["off", "factory"]),
        ("lift", &["O2"]),
    ] {
        let mut gains = vec![];
        let mut head = 0f64;
        for &label in labels {
            let src = &sources[&(pair.into(), label.into())];
            let power: Vec<_> = src[9600..38400].iter().map(|v| v * v).collect();
            let gain =
                0.1 / (power.chunks(8192).map(sum_numpy).sum::<f64>() / power.len() as f64).sqrt();
            gains.push(gain);
            head = head.max(gain * src.iter().map(|v| v.abs()).fold(0., f64::max));
        }
        let shared = (0.9 / head).min(1.);
        for (i, (&label, gain)) in labels.iter().zip(gains).enumerate() {
            let gain = gain * shared;
            let anon = if labels.len() == 1 {
                pair.into()
            } else {
                format!("{pair}-{}", char::from(b'A' + i as u8))
            };
            let src = &sources[&(pair.into(), label.into())];
            let g = serde_json::to_string(&gain)?;
            key.push(format!(
                "  \"{anon}\": {{\n    \"source\": \"{label}\",\n    \"listening_gain\": {g}\n  }}"
            ));
            gains_json.push(format!("  \"{anon}\": {g}"));
            records.insert(anon.clone(), json!({"source":label,"listening_gain":gain}));
            for (file, scale) in [
                (format!("engineering/{pair}-{label}.wav"), 1.),
                (format!("matched/{anon}.wav"), gain),
            ] {
                let samples: Vec<_> = src.iter().map(|v| (v * scale) as f32).collect();
                let data = float_wav(48000, &samples);
                let path = dir.join(&file);
                fs::create_dir_all(path.parent().unwrap())?;
                fs::write(path, &data)?;
                entries.push((file, data));
            }
        }
    }
    fs::write(
        dir.join("listening-key.json"),
        format!("{{\n{}\n}}\n", key.join(",\n")),
    )?;
    for f in ["vaisvil-low-e-stroke.wav", "ATTRIBUTION.md"] {
        entries.push((
            format!("reference/{f}"),
            fs::read(Path::new("tools/validation/fixtures/acoustic").join(f))?,
        ));
    }
    entries.push((
        "gains.json".into(),
        format!("{{\n{}\n}}", gains_json.join(",\n")).into_bytes(),
    ));
    entries.push(("LISTEN.txt".into(), LISTEN.as_bytes().to_vec()));
    canonical_archive(&entries, &dir.join("listening.zip"))?;
    for line in include_str!("../../../../validation/fixtures/stage07_audio_sha256.tsv").lines() {
        let (sha, file) = line.split_once("  ").ok_or("audio manifest")?;
        if !file.starts_with("audio/") {
            let h = evidence::digest(&dir.join(file))?;
            ensure(
                h == sha,
                &format!("listening artifact {file}: {h} expected {sha}"),
            )?;
        }
    }
    Ok(json!(records))
}
