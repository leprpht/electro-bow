//! Re-execute Review 8.2 against the actual offline epoch. A reproduced defect is reported explicitly.
use super::epoch::Kernel;
use crate::{Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, f64::consts::TAU, path::Path};
pub fn run(dir: &Path) -> Result<Value> {
    let kernel = Kernel::build(dir)?;
    let mut rates = BTreeMap::new();
    for rate in [32000, 44100, 47001, 48000, 88200, 96000, 176400, 192000] {
        let fs = f64::from(rate);
        let revoke = 6 * (4. * fs / 196. + 0.5).floor() as usize;
        let fade = (0.005 * fs + 0.5).floor() as usize;
        let n = revoke + fade + 2;
        let frames = withdrawal_frames(fs, n, revoke);
        let mut output = vec![[0.; 20]; n];
        kernel.epoch(fs).run(&frames, &mut output, 0)?;
        ensure(
            output[revoke - 1][8] == 1.,
            "deadline full permission setup",
        )?;
        let first = output[revoke..]
            .iter()
            .position(|row| row[8] == 0.)
            .ok_or("permission did not reach zero")?
            + 1;
        let end = revoke + fade - 1;
        let endpoint = output[end][8];
        let wet_delta = output[end][7] - frames[end][5];
        let late = matches!(rate, 32000 | 44100 | 48000 | 96000 | 176400);
        ensure(
            first == fade + usize::from(late)
                && (endpoint != 0.) == late
                && (wet_delta != 0.) == late,
            "R82-2 disposition changed",
        )?;
        rates.insert(rate,json!({"revoke":revoke,"expected_zero_steps":fade,"actual_zero_steps":first,"endpoint_permission":endpoint,"endpoint_correction":wet_delta,"contract_passed":!late}));
    }
    let mut state_faults = BTreeMap::new();
    for (label, value, site) in [
        ("nan", f64::NAN, 1),
        ("infinity", f64::INFINITY, 1),
        ("maximum", f64::MAX, 5),
    ] {
        let mut epoch = kernel.epoch(48000.);
        let mut frame = [1.; 20];
        frame[0] = 196.;
        frame[1..6].fill(0.125);
        frame[6..18].fill(0.1);
        epoch.run(&[frame], &mut [[0.; 20]], 0)?;
        epoch.inject_stored_phase(value);
        let err = epoch
            .run(&[frame], &mut [[0.; 20]], 0)
            .unwrap_err()
            .to_string();
        ensure(
            err == format!("epoch fault site={site} offset=0"),
            &format!("{label} classification: {err}"),
        )?;
        state_faults.insert(label, err);
    }
    Ok(json!({"review":"8.2","slew":rates,"phase_faults":state_faults,"late_deadlines":5}))
}

/// Historical comparison only. The declared envelope is independent of either producer's cache.
pub fn signed(reference: &Path) -> Result<Value> {
    use crate::evidence;
    use std::fs;
    let inputs = [
        (
            "entry",
            "f25d2e2f4a9d54d5166f36dfb769f8b529274049815ec25fcea1c94ea3f1b446",
        ),
        (
            "half-product",
            "973cc65ef226016499c8716ec197ac1c2984921dc1929ef19a64d4ba601f2527",
        ),
    ];
    let mut reports = Vec::new();
    let mut nominal = Vec::new();
    for (which, sha) in inputs {
        let path = reference.join(format!("{which}-joined.bin"));
        ensure(
            evidence::digest(&path)? == sha,
            "joined frozen producer stream",
        )?;
        let data = fs::read(path)?;
        ensure(data.len() == 18500 * 64, "joined sample count")?;
        let mut first = Value::Null;
        let mut changed = 0;
        let mut cache_mismatches = 0;
        for (i, b) in data.as_chunks::<64>().0.iter().enumerate() {
            let words = std::array::from_fn(|j| f64::from_le_bytes(b.as_chunks::<8>().0[j]));
            let [epoch, index, e, wet, product, cached_e, cache, direct] = words;
            let bounds = [
                0., 479., 863., 9479., 9719., 10103., 13000., 14680., 15599., 15983., 18500.,
            ];
            let levels = [0., 0., 1., 1., 0., 1., 1., 0., 0., 1., 1.];
            let energy = if epoch == 0. {
                0.
            } else {
                super::motion::interp(i as f64, &bounds, &levels)
            };
            ensure(
                e == energy && cached_e == energy,
                "joined expected envelope/cache energy",
            )?;
            let expected = energy * wet;
            if which == "entry" {
                nominal.push([epoch, index, wet, direct]);
            } else {
                ensure(
                    nominal[i].map(f64::to_bits) == [epoch, index, wet, direct].map(f64::to_bits),
                    "mutant normalized samples or direct changed",
                )?;
            }
            if cache != expected {
                cache_mismatches += 1;
            }
            if product != expected {
                changed += 1;
                if first.is_null() {
                    first = json!({"tick":i,"expected":expected,"actual":product,"wet":wet,"energy":energy,"predicate":"P == E*(signed_gain*wet)"});
                }
            }
        }
        if which == "entry" {
            ensure(
                changed == 0 && cache_mismatches == 0,
                "nominal signed product",
            )?;
        } else {
            ensure(
                changed == 17099 && cache_mismatches == 17099 && first["tick"] == 480,
                "R82-1 signed product witness",
            )?;
        }
        reports.push(json!({"producer":which,"frozen_sha256":sha,"first_mismatch":first,"wrong_live_products":changed,"wrong_cache_products":cache_mismatches,"historical_complete_exit":"passed"}));
    }
    Ok(json!({"runs":reports,"emitted_a4_available":false}))
}

fn withdrawal_frames(fs: f64, n: usize, revoke: usize) -> Vec<[f64; 20]> {
    (0..n)
        .map(|i| {
            let q = 0.4 * (TAU * 196. * (i + 1) as f64 / fs).cos();
            let mut frame = [0.; 20];
            frame[0] = 196.;
            frame[1..6].fill(q);
            frame[6] = 0.1;
            frame[18] = 1.;
            frame[19] = f64::from(i < revoke);
            frame
        })
        .collect()
}

pub fn corrected() -> Result<Value> {
    use super::candidate::Slew;
    let mut rows = Vec::new();
    for fs in [
        48000.,
        32000.,
        44100.,
        88200.,
        96000.,
        176400.,
        192000.,
        47001.,
        32008.333333333336,
        95999.5,
    ] {
        let revoke = 6 * (4. * fs / 196. + 0.5_f64).floor() as usize;
        let n = (0.005 * fs).round() as usize;
        let end = revoke + n - 1;
        let mut frames = withdrawal_frames(fs, end + 2, revoke);
        // Distinct signs survive the exact zero path, even with live nonzero bank history.
        frames[end][2..6].copy_from_slice(&[-0., 0., -0., -0.]);
        let mut output = vec![[0.; 20]; frames.len()];
        Kernel::Corrected.epoch(fs).run(&frames, &mut output, 0)?;
        ensure(output[revoke - 1][8] == 1., "corrected full-scale setup")?;
        ensure(
            output[revoke..end].iter().all(|r| r[8] > 0.) && output[end][8] == 0.,
            "exact neutral on Nth decrement",
        )?;
        ensure(
            (0..4).all(|j| output[end][4 + j].to_bits() == frames[end][2 + j].to_bits()),
            "first neutral q bits",
        )?;
        let bound = 1. / n as f64 + 2. * f64::EPSILON;
        ensure(
            output
                .windows(2)
                .all(|w| (w[1][8] - w[0][8]).abs() <= bound),
            "binary64 slew step",
        )?;
        let mut reversals = 0;
        for grid in 0..=16 {
            let mut ramp = Slew::default();
            for _ in 0..n {
                ramp.advance(grid as f64 / 16., n as u32);
            }
            for (target, ticks) in [
                (1., 7),
                (0., 3),
                (0.375, 13),
                (0., n),
                (1., n),
                (0.25, n),
                (0., n),
            ] {
                let origin = ramp.value;
                for tick in 1..=ticks {
                    let before = ramp.value;
                    let x = ramp.advance(target, n as u32);
                    let distance = tick as f64 / n as f64;
                    let expected = if origin < target {
                        (origin + distance).min(target)
                    } else {
                        (origin - distance).max(target)
                    };
                    ensure(
                        (x - expected).abs() <= 2. * f64::EPSILON && (x - before).abs() <= bound,
                        "interrupted origin and reversal",
                    )?;
                    if distance >= (target - origin).abs() {
                        ensure(x == target, "interrupted exact endpoint")?;
                    }
                }
                reversals += 1;
            }
        }
        rows.push(json!({"rate":fs,"revocation":revoke,"increments":n,"first_exact_neutral":end,"q_bits":frames[end][2..6].iter().map(|x|x.to_bits()).collect::<Vec<_>>(),"step_bound":bound,"interrupted_runs":reversals}));
    }
    Ok(json!(rows))
}
