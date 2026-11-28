//! Absolute-time physical reference. Does not import the production ring.
use crate::{Result, ensure, evidence};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    f64::consts::{PI, TAU},
    fs,
    path::Path,
};

type Impulse = BTreeMap<i32, f64>;
fn taps(delay: f64, reverse: bool) -> Impulse {
    let n = delay.floor() as i32;
    let u = delay - f64::from(n);
    let pair = if reverse { [u, 1. - u] } else { [1. - u, u] };
    [(n, pair[0]), (n + 1, pair[1])]
        .into_iter()
        .filter(|(_, x)| *x != 0.)
        .collect()
}
fn convolve(a: &Impulse, b: &Impulse) -> Impulse {
    let mut out = Impulse::new();
    for (t, x) in a {
        for (s, y) in b {
            *out.entry(t + s).or_default() += x * y;
        }
    }
    out
}
fn circuit(period: f64, reverse: bool) -> Impulse {
    let fir = [(0, -0.98 * 0.125), (1, -0.98 * 0.75), (2, -0.98 * 0.125)].into();
    let bridge = convolve(&taps(0.127 * period - 1., reverse), &fir);
    let nut = taps((1. - 0.127) * period, false)
        .into_iter()
        .map(|(k, x)| (k, -x))
        .collect();
    convolve(&bridge, &nut)
}
fn delay(impulse: &Impulse, omega: f64) -> Result<f64> {
    let first = *impulse.keys().next().ok_or("empty impulse")?;
    ensure(
        impulse.values().all(|v| *v > 0.)
            && f64::from(impulse.keys().last().unwrap() - first) * omega < PI,
        "unambiguous impulse phase",
    )?;
    let (mut re, mut im) = (0., 0.);
    for (n, amp) in impulse {
        let angle = -omega * f64::from(n - first);
        re += amp * angle.cos();
        im += amp * angle.sin();
    }
    Ok(f64::from(first) - im.atan2(re) / omega)
}
fn history(neck: bool, words: &mut Vec<u8>) -> Result<()> {
    let capacity = 743i32;
    let seed = |t: i32| {
        let mut phase = f64::from(t) / (48000. / 440.);
        if neck {
            phase = phase + 1. - 0.127;
        }
        (if neck { -1. } else { 1. }) * (0.1 / 0.127) * (phase - phase.floor() - 0.5)
    };
    let mut eager: Vec<_> = (-capacity..0).map(seed).collect();
    let mut lazy = vec![3.25; capacity as usize];
    for n in 0..4800i32 {
        let period = 48000. / if n < 100 { 440. } else { 65. };
        let mut delays = vec![if neck {
            (1. - 0.127) * period
        } else {
            0.127 * period - 1.
        }];
        if !neck {
            delays.push(0.127 * period / 2.);
            if n == 0 {
                delays.extend([0.127 * period - 1. + 1., 0.127 * period - 1. + 2.]);
            }
        }
        for d in delays {
            let m = f64::floor(d) as i32;
            let u = d - f64::from(m);
            words.extend_from_slice(&(n as u64).to_le_bytes());
            words.extend_from_slice(&d.to_le_bytes());
            let mut reads = [[0.; 2]; 2];
            for (i, lag) in [m, m + 1].into_iter().enumerate() {
                let age = n.min(capacity);
                let a = if lag <= age {
                    lazy[(n - lag).rem_euclid(capacity) as usize]
                } else {
                    seed(age - lag)
                };
                let b = eager[(capacity + n - lag) as usize];
                reads[i] = [a, b];
                words.extend_from_slice(&a.to_le_bytes());
                words.extend_from_slice(&b.to_le_bytes());
            }
            ensure(
                (1. - u) * reads[0][0] + u * reads[1][0]
                    == (1. - u) * reads[0][1] + u * reads[1][1],
                "chronological read",
            )?;
        }
        let wave = (0.731 * f64::from(n)).sin();
        eager.push(wave);
        lazy[(n % capacity) as usize] = wave;
    }
    Ok(())
}

pub fn run(out: &Path, reference: Option<&Path>) -> Result<Value> {
    let omega = TAU * 1320. / 32000.;
    let phase = (0..=100000)
        .map(|i| {
            let u = f64::from(i) / 100000.;
            ((u * omega.sin()).atan2(1. - u + u * omega.cos()) / omega - u).abs()
        })
        .fold(0., f64::max);
    let mut bytes = Vec::new();
    let mut rows = Vec::new();
    let mut worst = [0.0_f64; 5];
    for fs in [32000, 44100, 48000, 88200, 96000, 176400, 192000] {
        for fundamental in [65.41, 440., 1318.51] {
            let p = f64::from(fs) / fundamental;
            let impulse = circuit(p, false);
            let shifted = |offset| impulse.iter().map(|(k, v)| (k + offset, *v)).collect();
            let controls = [
                impulse.clone(),
                [(0, 0.98)].into(),
                shifted(-1),
                shifted(1),
                circuit(p, true),
            ];
            let mut errors = Vec::new();
            for (i, control) in controls.iter().enumerate() {
                let measured = delay(control, TAU / p)?;
                bytes.extend_from_slice(&measured.to_le_bytes());
                let error = (measured - p).abs();
                worst[i] = worst[i].max(error);
                errors.push(error);
                if i == 0 {
                    ensure(error <= 0.003, "absolute circuit phase")?;
                }
            }
            rows.push(json!({"rate":fs,"hz":fundamental,"errors_samples":errors}));
        }
    }
    ensure(
        worst[1..].iter().all(|e| *e > 0.003),
        "absolute delay negative controls",
    )?;
    for d in [1.0_f64, 3.25, 14.] {
        let mut ring = [0.; 16];
        let m = d.floor() as i32;
        let u = d.fract();
        for n in 0..64i32 {
            let v = (1. - u) * ring[(n - m).rem_euclid(16) as usize]
                + u * ring[(n - m - 1).rem_euclid(16) as usize];
            ensure(
                v == (1. - u) * f64::from(n == m) + u * f64::from(n == m + 1),
                "wrap impulse",
            )?;
            ring[(n % 16) as usize] = f64::from(n == 0);
        }
    }
    history(false, &mut bytes)?;
    history(true, &mut bytes)?;
    let artifact = out.join("ring.bin");
    fs::write(&artifact, &bytes)?;
    if let Some(root) = reference {
        let original = root.join("ring.bin");
        ensure(
            evidence::digest(&original)?
                == "540cd3e58f3a37cffe7d21b2add85384f35472d8f6d7e65a0890d5f6e3d424e1",
            "frozen ring corpus digest",
        )?;
        let frozen = fs::read(original)?;
        ensure(frozen.len() == bytes.len(), "ring corpus count")?;
        // Delays are measured against the existing .003-sample physical bound.
        // Exact equality applies to every chronological history input and read.
        let split = 21 * 5 * 8;
        for (a, b) in bytes[..split]
            .as_chunks::<8>()
            .0
            .iter()
            .zip(frozen[..split].as_chunks::<8>().0)
        {
            ensure(
                (f64::from_le_bytes(*a) - f64::from_le_bytes(*b)).abs() <= 0.003,
                "frozen circuit delay",
            )?;
        }
        ensure(
            bytes[split..] == frozen[split..],
            "frozen chronological history words",
        )?;
    }
    Ok(
        json!({"phase_max_samples":phase,"max_errors_samples":worst,"circuits":rows,"ticks_per_arm":4800,"history_slots":743,"artifact":evidence::digest(&artifact)?}),
    )
}
