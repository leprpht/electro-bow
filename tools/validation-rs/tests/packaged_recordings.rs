//! Optional cross-host replay of the four historical recorded guitar auditions.
//! Run after the public package suite.
#![cfg(target_os = "macos")]
use hound::{SampleFormat, WavReader};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::{Seek, Write},
    path::Path,
    process::{Command, Stdio},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn excerpt(root: &Path, name: &str, seconds: usize, align: bool) -> Result<(usize, Vec<f32>)> {
    let mut source = WavReader::open(
        root.join("ThirdParty/q/test/audio_files")
            .join(format!("{name}.wav")),
    )?;
    let spec = source.spec();
    assert_eq!(spec.channels, 1);
    let rate = spec.sample_rate as usize;
    let x = match spec.sample_format {
        SampleFormat::Float => source
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()?,
        SampleFormat::Int => {
            let scale = 2f64.powi(i32::from(spec.bits_per_sample) - 1);
            source
                .samples::<i32>()
                .map(|v| v.map(|s| (f64::from(s) / scale) as f32))
                .collect::<std::result::Result<Vec<_>, _>>()?
        }
    };
    let start = if align {
        x.iter()
            .position(|v| v.abs() > 0.025)
            .ok_or("missing onset")?
            .saturating_sub((0.03 * rate as f64) as usize)
    } else {
        0
    };
    Ok((rate, x[start..x.len().min(start + seconds * rate)].to_vec()))
}
fn strum(root: &Path) -> Result<(usize, Vec<f32>)> {
    let mut rate = 0;
    let mut chord = Vec::new();
    for (i, n) in ["3a-D", "2b-A-12th", "3b-D-12th", "-2c-F#-24th"]
        .into_iter()
        .enumerate()
    {
        let (fs, x) = excerpt(root, n, 3, true)?;
        let spacing = (fs as f64 * 0.023).round_ties_even() as usize;
        if i == 0 {
            rate = fs;
            chord.resize(rate * 3 + 3 * spacing, 0.);
        }
        assert_eq!(rate, fs);
        for (v, w) in chord[spacing * i..].iter_mut().zip(x) {
            *v += 0.35f32 * w;
        }
    }
    Ok((rate, chord))
}
fn render(
    host: &Path,
    bundle: &Path,
    rate: usize,
    block: usize,
    controls: [f32; 6],
    data: &[f32],
    state: (&str, &Path),
) -> Result<Vec<f32>> {
    let mut input = tempfile::tempfile()?;
    input.write_all(
        &data
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect::<Vec<_>>(),
    )?;
    input.rewind()?;
    let cmd = Command::new(host)
        .arg(bundle)
        .args([rate, 1, block].map(|v| v.to_string()))
        .args(controls.map(|v| v.to_string()))
        .arg(state.0)
        .arg(state.1)
        .stdin(Stdio::from(input))
        .output()?;
    assert!(
        cmd.status.success(),
        "{}",
        String::from_utf8_lossy(&cmd.stderr)
    );
    assert_eq!(cmd.stdout.len(), 4 * data.len());
    Ok(cmd
        .stdout
        .as_chunks::<4>()
        .0
        .iter()
        .map(|w| f32::from_le_bytes(*w))
        .collect())
}
fn wav24(path: &Path, rate: usize, data: &[f32]) -> Result<()> {
    let mut bytes = Vec::with_capacity(44 + data.len() * 3 + 1);
    let length = u32::try_from(data.len() * 3)?;
    bytes.extend(b"RIFF");
    bytes.extend((length + 36 + (length % 2)).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend((rate as u32).to_le_bytes());
    bytes.extend((3 * rate as u32).to_le_bytes());
    bytes.extend(3u16.to_le_bytes());
    bytes.extend(24u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(length.to_le_bytes());
    for &v in data {
        assert!(v.is_finite());
        let word = (f64::from(v) * 2147483648.).round_ties_even() as i32;
        bytes.extend(&word.to_le_bytes()[1..]);
    }
    if length % 2 == 1 {
        bytes.push(0);
    }
    fs::write(path, bytes)?;
    Ok(())
}
#[test]
#[ignore = "requires a built macOS VST3, the validation host"]
fn recorded_audition_and_preset_recall() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let host = root.join(
        "build/validation-migration/native-host/validation-host_artefacts/Release/validation-host",
    );
    let bundle = root.join("build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3");
    let reference: BTreeMap<String, String> =
        serde_json::from_str(include_str!("fixtures/packaged-audition.json"))?;
    let base = root.join("build/validation-migration/packaged-audition");
    fs::create_dir_all(&base)?;
    let out = tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(base)?
        .keep();
    let cases = [
        (
            "low-E-swell",
            excerpt(&root, "1a-Low-E", 4, true)?,
            [240., 0.68, 0.5, 0.5, 0.85, 0.5],
        ),
        (
            "staccato",
            excerpt(&root, "GStaccato", 5, false)?,
            [35., 0.72, 0.3, 0.65, 0.5, 0.5],
        ),
        (
            "layered-D-chord",
            strum(&root)?,
            [115., 0.8, 0.5, 0.7, 0.85, 0.5],
        ),
        (
            "riff",
            excerpt(&root, "riff_0_0", 5, true)?,
            [22., 0.85, 0.55, 0.5, 1., 0.5],
        ),
    ];
    let mut report = BTreeMap::<String, Value>::new();
    for (label, (rate, mut dry), controls) in cases {
        let fade = dry.len().min((rate as f64 * 0.015) as usize);
        let start = dry.len() - fade;
        let step = -1. / (fade - 1) as f64;
        for (i, v) in dry[start..].iter_mut().enumerate() {
            *v = (f64::from(*v) * (1. + i as f64 * step)) as f32;
        }
        dry.resize(dry.len() + rate, 0.);
        let state = out.join(format!("{label}.state"));
        let wet = render(&host, &bundle, rate, 127, controls, &dry, ("save", &state))?;
        assert!(wet.iter().all(|v| v.is_finite() && v.abs() <= 0.980001));
        let mut recalls = Vec::new();
        for block in [17, 1024] {
            let recalled = render(&host, &bundle, rate, block, [0.; 6], &dry, ("load", &state))?;
            let error = wet
                .iter()
                .zip(recalled)
                .map(|(s, r)| (s - r).abs())
                .fold(0f32, f32::max);
            assert!(error <= 1e-6, "{label} block {block}: {error}");
            recalls.push(error);
        }
        for (suffix, x) in [("dry", &dry), ("wet", &wet)] {
            let filename = format!("{label}-{suffix}.wav");
            let file = out.join(&filename);
            wav24(&file, rate, x)?;
            let actual = format!("{:x}", Sha256::digest(fs::read(&file)?));
            let expected = reference.get(&filename).ok_or("unexpected audition case")?;
            assert_eq!(
                expected, &actual,
                "canonical pedalboard/soundfile PCM24 file {filename}"
            );
            report.insert(filename,json!({"reference_sha256":expected,"actual_sha256":actual,"frames":x.len(),"recalled_max_errors_FS":recalls}));
        }
    }
    assert_eq!(report.len(), reference.len());
    let digest =
        |path: &Path| -> Result<String> { Ok(format!("{:x}", Sha256::digest(fs::read(path)?))) };
    let receipt = json!({"files":report,"host_sha256":digest(&host)?,"package_sha256":digest(&bundle.join("Contents/MacOS/ElectroBow"))?,"test_sha256":digest(&std::env::current_exe()?)?});
    fs::write(
        out.join("comparison.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!("{}", out.display());
    Ok(())
}
