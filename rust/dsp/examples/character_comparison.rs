//! Reproducible synthetic guitar-like fixture, not part of the plugin DSP.
//! Usage: cargo run -p electrobow-dsp --release --example character_comparison -- build/bow-audio
use electrobow_dsp::*;
use std::{fs, io::Write, path::Path};
fn wav(path: &Path, samples: &[f32]) {
    let mut file = fs::File::create(path).unwrap();
    let bytes = (samples.len() * 2) as u32;
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + bytes).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    for v in [1u16, 1] {
        file.write_all(&v.to_le_bytes()).unwrap();
    }
    for v in [48000u32, 96000] {
        file.write_all(&v.to_le_bytes()).unwrap();
    }
    for v in [2u16, 16] {
        file.write_all(&v.to_le_bytes()).unwrap();
    }
    file.write_all(b"data").unwrap();
    file.write_all(&bytes.to_le_bytes()).unwrap();
    for &x in samples {
        file.write_all(&((x.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
            .unwrap();
    }
}
fn main() {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "build/bow-audio".into());
    let path = Path::new(&directory);
    fs::create_dir_all(path).unwrap();
    let input: Vec<f32> = (0..48000 * 8)
        .map(|i| {
            let t = i as f64 / 48000.0;
            let period = if t < 4.0 { 1.0 } else { 0.075 };
            let age = t % period;
            let mut x = 0.0;
            for f in [146.832, 220.0, 293.665] {
                for h in 1..=10 {
                    x += 0.045 / h as f64
                        * (-age * (2.0 + h as f64 * 0.8)).exp()
                        * (std::f64::consts::TAU * f * h as f64 * t).sin();
                }
            }
            let dynamics = if t < 2.0 { 0.35 } else { 1.0 };
            (x * dynamics) as f32
        })
        .collect();
    wav(&path.join("00-input-synthetic.wav"), &input);
    for (name, amount) in [
        ("01-smooth.wav", 0.0),
        ("02-textured.wav", 0.5),
        ("03-rough.wav", 1.0),
    ] {
        let bank = eb_instrument_create(48000.0, 256, 1);
        assert!(!bank.is_null());
        let mut output = vec![0.0; input.len()];
        for (source, destination) in input.chunks(256).zip(output.chunks_mut(256)) {
            // SAFETY: exclusive live handle, disjoint arrays, valid chunk sizes.
            unsafe {
                assert_eq!(
                    eb_instrument_process(
                        bank,
                        [source.as_ptr()].as_ptr(),
                        [destination.as_mut_ptr()].as_ptr(),
                        source.len() as i32,
                        InstrumentControls {
                            attack_ms: 250.0,
                            character: amount,
                            noise: 0.5,
                            saturation: 0.5,
                            spectral_movement: 0.5,
                            out: 0.5,
                        }
                    ),
                    1
                );
            }
        }
        // SAFETY: unique ownership; all borrowed buffers are no longer used.
        unsafe {
            eb_instrument_destroy(bank);
        }
        let rms = (output.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>() / output.len() as f64)
            .sqrt();
        println!(
            "{name}: rms={rms:.8}, peak={:.8}",
            output.iter().map(|x| x.abs()).fold(0.0, f32::max)
        );
        wav(&path.join(name), &output);
    }
}
