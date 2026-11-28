use super::Case;
use crate::{Result, ensure};
use hound::{SampleFormat, WavReader};
use std::{f64::consts::TAU, path::Path};

use crate::numerics::sum_numpy;

fn noise(n: usize, level: f64) -> Vec<f64> {
    let mut v = 1u32;
    let mut x: Vec<_> = (0..n)
        .map(|_| {
            v ^= v << 13;
            v ^= v >> 17;
            v ^= v << 5;
            2. * f64::from(v) / f64::from(u32::MAX) - 1.
        })
        .collect();
    let power: Vec<_> = x.iter().map(|v| v * v).collect();
    // NumPy iterates contiguous reduction operands in 8,192-element buffers.
    let power_sum = power.chunks(8192).map(sum_numpy).sum::<f64>();
    let gain = level / (power_sum / n as f64).sqrt();
    for v in &mut x {
        *v *= gain;
    }
    x
}
pub fn generate(c: &Case) -> Result<(Vec<f64>, Vec<f64>)> {
    let fs = f64::from(c.fs);
    let mut truth = vec![];
    let mut signal = vec![0.; c.stimulus_start];
    if c.group == "DI" {
        let mut reader = WavReader::open(
            Path::new("ThirdParty/q/test/audio_files").join(format!("{}.wav", c.shape)),
        )?;
        let spec = reader.spec();
        ensure(
            spec.channels == 1 && spec.sample_rate == c.fs,
            "DI metadata",
        )?;
        match spec.sample_format {
            SampleFormat::Int => {
                let norm = 2f64.powi(i32::from(spec.bits_per_sample) - 1);
                for v in reader.samples::<i32>() {
                    signal.push(f64::from(v?) / norm);
                }
            }
            SampleFormat::Float => {
                for v in reader.samples::<f32>() {
                    signal.push(f64::from(v?));
                }
            }
        }
        return Ok((signal, truth));
    }
    if c.group == "unpitched" {
        let n = 10 * c.fs as usize;
        let x = match c.shape.as_str() {
            "noise-65" | "noise-30" => noise(n, 10f64.powf(c.shape[5..].parse::<f64>()? / 20.)),
            "hum50" | "hum60" => {
                let hz = c.shape[3..].parse::<f64>()?;
                (0..n)
                    .map(|i| {
                        (TAU * hz * i as f64 / fs).sin() * 2f64.sqrt() * 10f64.powf(-50. / 20.)
                    })
                    .collect()
            }
            "silence" => vec![0.; n],
            "2ms-click" => {
                let mut x = vec![0.; c.fs as usize];
                x[..96].copy_from_slice(&noise(96, 0.2));
                x
            }
            _ => return Err("unknown rejection fixture".into()),
        };
        signal.extend(x);
        return Ok((signal, truth));
    }
    let n = match c.group.as_str() {
        "sweep" => (fs * 0.12).round_ties_even() as usize,
        "trajectory" => 3 * c.fs as usize,
        _ => 2 * c.fs as usize,
    };
    let f = c.f.parse::<f64>().unwrap_or(0.);
    let mut phase = Vec::with_capacity(n);
    if c.group == "trajectory" {
        let mut sum = 0.;
        for i in 0..n {
            let t = i as f64 / fs;
            let hz = if c.shape == "bend" {
                196. * 2f64.powf((1. - (t - 1.2).abs() / 0.2).max(0.) / 6.)
            } else {
                let base = if c.shape == "vib196" { 196. } else { 440. };
                base * 2f64.powf(35. * (TAU * 5. * t).sin() / 1200.)
            };
            phase.push(TAU * sum / fs);
            sum += hz;
            truth.push(hz);
        }
    } else {
        for i in 0..n {
            phase.push(TAU * f * (i as f64 / fs));
        }
    }
    let shape = match c.group.as_str() {
        "trajectory" => "six",
        "onset" => "pluck",
        _ => c.shape.as_str(),
    };
    let weights: Vec<f64> = match shape {
        "sine" => vec![1.],
        "six" | "pluck" => (1..=if shape == "six" { 6 } else { 8 })
            .map(|j| 1. / f64::from(j))
            .collect(),
        "weak" => vec![0.05, 1., 0.6, 0.35],
        "second" => vec![0.2, 1., 0.1, 0.25],
        "bright" => vec![1.; 6],
        _ => return Err("unknown pitch spectrum".into()),
    };
    let mut x = vec![0.; n];
    for (i, w) in weights.iter().enumerate() {
        let k = (i + 1) as f64;
        let shift = match shape {
            "bright" => 0.,
            "second" => 0.7 * k,
            _ => 0.13 * k * k,
        };
        for (v, p) in x.iter_mut().zip(&phase) {
            *v += w * (k * p + shift).sin();
        }
    }
    if shape == "pluck" {
        for (i, v) in x.iter_mut().enumerate() {
            let t = i as f64 / fs;
            *v *= -(-t / 0.001).exp_m1() * (-t / 0.28).exp();
        }
    }
    let scale = c.peak.parse::<f64>()? / x.iter().map(|v| v.abs()).fold(0., f64::max);
    for v in &mut x {
        *v *= scale;
    }
    if c.group == "onset" && c.shape == "noisy" {
        for (v, n) in x.iter_mut().zip(noise(384, 0.1)) {
            *v += n;
        }
    }
    signal.extend(x);
    Ok((signal, truth))
}
