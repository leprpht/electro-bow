use crate::{Result, ensure, evidence, process::check};
use libloading::Library;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    ffi::c_void,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const BUNDLE: &str = "build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3";
const NEUTRAL: Controls = Controls([0., 0., 0.5, 0.5, 0.5, 0.5]);
const WET: Controls = Controls([50., 0.5, 0.5, 0.5, 0.5, 0.5]);
#[repr(C)]
#[derive(Clone, Copy)]
struct Controls([f64; 6]);

fn pluck(rate: usize) -> Vec<f32> {
    let mut body = Vec::with_capacity(2 * rate);
    for i in 0..2 * rate {
        let t = i as f64 / rate as f64;
        let angle = 2. * std::f64::consts::PI * 110. * t;
        let sum = (1..=8)
            .map(f64::from)
            .filter(|k| 110. * k < 0.45 * rate as f64)
            .map(|k| (k * angle + 0.13 * k * k).sin() / k)
            .sum::<f64>();
        body.push((1. - (-t / 0.001).exp()) * (-t / 0.28).exp() * sum);
    }
    let peak = body.iter().copied().map(f64::abs).fold(0., f64::max);
    let mut x = vec![0.; rate / 10];
    x.extend(body.into_iter().map(|v| (v * 0.35 / peak) as f32));
    x.resize(x.len() + rate / 2, 0.);
    x.push(0.25);
    x.resize(x.len() + (1.3 * rate as f64).round_ties_even() as usize, 0.);
    x
}
type Create = unsafe extern "C" fn(f64, i32, i32) -> *mut c_void;
type Destroy = unsafe extern "C" fn(*mut c_void);
type Process =
    unsafe extern "C" fn(*mut c_void, *const *const f32, *const *mut f32, i32, Controls) -> i32;
fn abi(
    lib: &Library,
    rate: usize,
    x: &[f32],
    channels: usize,
    controls: Controls,
    inplace: bool,
) -> Result<Vec<f32>> {
    // All symbols are loaded from the same packaged image and the handle is destroyed on every exit.
    unsafe {
        let create = lib.get::<Create>(b"eb_instrument_create")?;
        let destroy = lib.get::<Destroy>(b"eb_instrument_destroy")?;
        let process = lib.get::<Process>(b"eb_instrument_process")?;
        let h = create(rate as f64, 127, channels as i32);
        ensure(!h.is_null(), "ABI create")?;
        let result = (|| -> Result<Vec<f32>> {
            let mut output = Vec::with_capacity(x.len());
            for frames in x.chunks(channels * 127) {
                let n = frames.len() / channels;
                let mut input = [[0f32; 127]; 2];
                let mut dest = [[0f32; 127]; 2];
                for (idx, &v) in frames.iter().enumerate() {
                    input[idx % channels][idx / channels] = v;
                }
                let src = input.each_ref().map(|s| s.as_ptr());
                let dst = if inplace {
                    input.each_mut().map(|s| s.as_mut_ptr())
                } else {
                    dest.each_mut().map(|s| s.as_mut_ptr())
                };
                ensure(
                    process(h, std::ptr::null(), std::ptr::null(), 0, controls) == 1,
                    "ABI zero block",
                )?;
                ensure(
                    process(h, src.as_ptr(), dst.as_ptr(), n as i32, controls) == 1,
                    "ABI block",
                )?;
                let out = if inplace { &input } else { &dest };
                for i in 0..n {
                    for ch in &out[..channels] {
                        output.push(ch[i]);
                    }
                }
            }
            Ok(output)
        })();
        destroy(h);
        result
    }
}
fn bytes(x: &[f32]) -> Vec<u8> {
    x.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn max_difference(x: &[f32], y: &[f32]) -> Result<f32> {
    ensure(
        x.len() == y.len() && x.iter().chain(y).all(|v| v.is_finite()),
        "package sample count or finite values",
    )?;
    Ok(x.iter()
        .zip(y)
        .map(|(a, b)| (a - b).abs())
        .fold(0., f32::max))
}
struct Host {
    exe: PathBuf,
    scratch: PathBuf,
}
impl Host {
    fn render(
        &self,
        rate: usize,
        channels: usize,
        block: usize,
        controls: Controls,
        x: &[f32],
        state: Option<(&str, &Path)>,
    ) -> Result<Vec<f32>> {
        let mut input = tempfile::tempfile_in(&self.scratch)?;
        input.write_all(&bytes(x))?;
        std::io::Seek::rewind(&mut input)?;
        let mut cmd = Command::new(&self.exe);
        cmd.arg(BUNDLE)
            .args([rate, channels, block].map(|n| n.to_string()))
            .args(controls.0.map(|v| v.to_string()))
            .stdin(Stdio::from(input));
        if let Some((mode, path)) = state {
            cmd.arg(mode).arg(path);
        }
        let data = check(&mut cmd)?.stdout;
        ensure(
            data.len() == x.len() * 4,
            "VST3 returned wrong sample count",
        )?;
        Ok(data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect())
    }
}
pub fn run(dir: &Path) -> Result<Value> {
    ensure(
        cfg!(target_os = "macos"),
        "packaged ABI location requires macOS",
    )?;
    let build = Path::new("build/validation-migration/native-host");
    check(
        Command::new("cmake")
            .args(["-S", "tools/validation-rs/native", "-B"])
            .arg(build)
            .arg(format!(
                "-DJUCE_PATH={}",
                fs::canonicalize(std::env::var_os("JUCE_PATH").unwrap_or_else(|| "JUCE".into()))?
                    .display()
            ))
            .arg("-DCMAKE_BUILD_TYPE=Release"),
    )?;
    check(Command::new("cmake").arg("--build").arg(build).args([
        "--config",
        "Release",
        "--parallel",
        "6",
    ]))?;
    let host = Host {
        exe: build.join("validation-host_artefacts/Release/validation-host"),
        scratch: dir.into(),
    };
    let binary = Path::new(BUNDLE).join("Contents/MacOS/ElectroBow");
    let lib = unsafe { Library::new(&binary)? };
    let mut rows = vec![];
    for rate in [
        1000, 2000, 4000, 8000, 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000, 384000,
    ] {
        let p = pluck(rate);
        for channels in [1, 2] {
            let x = if channels == 1 {
                p.clone()
            } else {
                p.iter().flat_map(|v| [*v, -*v]).collect()
            };
            let dst = abi(&lib, rate, &x, channels, NEUTRAL, false)?;
            let alias = abi(&lib, rate, &x, channels, NEUTRAL, true)?;
            let packaged = host.render(rate, channels, 127, NEUTRAL, &x, None)?;
            let error = [
                max_difference(&dst, &x)?,
                max_difference(&alias, &x)?,
                max_difference(&packaged, &x)?,
            ];
            ensure(error.iter().all(|e| *e <= 2e-7), "neutral ABI/VST3 parity")?;
            rows.push(json!({"rate":rate,"channels":channels,"frames":p.len(),"errors":error}));
        }
    }
    let x = pluck(48000);
    let dst = abi(&lib, 48000, &x, 1, WET, false)?;
    let state = dir.canonicalize()?.join("state.bin");
    let wet = host.render(48000, 1, 127, WET, &x, Some(("save", &state)))?;
    let restored = host.render(48000, 1, 73, NEUTRAL, &x, Some(("load", &state)))?;
    let error = max_difference(&dst, &wet)?;
    let effect = max_difference(&x, &wet)?;
    ensure(error <= 1e-6 && effect > 0.01, "wet ABI/package response")?;
    ensure(
        max_difference(&wet, &restored)? == 0.,
        "package restored preset/partition",
    )?;
    let mut hashes = vec![];
    for (name, samples, expected) in [
        (
            "input",
            &x,
            "5206d315d476ce363fb99c0e90884b68f599a3a3e4626e33bfad5c7322588709",
        ),
        (
            "abi",
            &dst,
            "c4f241531583ac391ec9419a8c21fc7b5f27465e78d93dfcb0aaa64e12fae6de",
        ),
        (
            "vst3",
            &wet,
            "c7251cd11c37048e8cbd2a645897bcd14e0b87dfec1f650c2797bd5cb5658a94",
        ),
    ] {
        ensure(
            samples[samples.len() - 14400..].iter().all(|v| *v == 0.),
            "package tail",
        )?;
        let hash = format!("{:x}", Sha256::digest(bytes(samples)));
        ensure(
            hash == expected,
            &format!("fixed package {name} payload {hash}, expected {expected}"),
        )?;
        let mut wav = hound::WavWriter::create(
            dir.join(format!("fixed-{name}.wav")),
            hound::WavSpec {
                channels: 1,
                sample_rate: 48000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )?;
        for &sample in samples {
            wav.write_sample(sample)?;
        }
        wav.finalize()?;
        hashes.push(json!({"name":name,"payload_sha256":hash}));
    }
    Ok(
        json!({"neutral":rows,"fixed":hashes,"wet_error":error,"effect":effect,"state_partition_max_error":0,"host":evidence::digest(&host.exe)?,"packaged_image":evidence::digest(&binary)?}),
    )
}
