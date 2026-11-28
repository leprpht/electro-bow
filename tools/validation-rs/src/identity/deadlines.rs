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
        let frames: Vec<_> = (0..n)
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
            .collect();
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
