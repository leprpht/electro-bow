//! Source-level I1 path challenges, using complete-window QR and an independently evolved correction bank.
use super::{Bank, epoch::Kernel, motion, qr};
use crate::{
    Result,
    electrical::{carrier, scheduled_taps},
    ensure, evidence, process, replay,
};
use nalgebra::DVector;
use serde_json::{Value, json};
use std::{f64::consts::TAU, fs, path::Path, process::Command};
fn columns(r: &[[f64; 20]], j: usize) -> DVector<f64> {
    DVector::from_iterator(r.len(), r.iter().map(|f| f[j]))
}
pub(super) struct Trajectory {
    pub(super) scale: Vec<f64>,
    pub(super) wet: Vec<[f64; 4]>,
    pub(super) decisions: Vec<(usize, bool)>,
}
pub(super) fn drive(input: &[[f64; 20]], delta: &[[f64; 4]], fs: f64) -> Result<Trajectory> {
    let mut scale = vec![0.; input.len()];
    let mut wet = vec![[0.; 4]; input.len()];
    let mut decisions = vec![];
    let mut phase = 0.;
    let mut level = 0.;
    let mut target = 0.;
    let mut next = 0.;
    let mut ready = 0u32;
    let ramp = (0.005 * fs + 0.5).floor();
    let mut start = 0;
    while start < input.len() {
        let length = 2 * (4. * fs / input[start][0] + 0.5).floor() as usize;
        let end = (start + length).min(input.len());
        for (a, b, dest) in [
            (start, (start + 17).min(end), target),
            ((start + 17).min(end), end, next),
        ] {
            let initial = level;
            for i in a..b {
                let d: f64 = dest - initial;
                level = initial + d.signum() * d.abs().min((i - a + 1) as f64 / ramp);
                scale[i] = level;
                for j in 0..4 {
                    wet[i][j] = if level == 0. {
                        input[i][2 + j]
                    } else {
                        input[i][2 + j] + level * delta[i][j]
                    };
                }
            }
        }
        target = next;
        if end - start != length {
            break;
        }
        let rows = &input[start..end];
        let n = length as f64;
        let angles: Vec<_> = rows
            .iter()
            .map(|r| {
                phase += TAU / fs * r[0];
                phase
            })
            .collect();
        phase %= TAU;
        let b = qr::basis(&angles);
        let gram = b.transpose() * &b / n;
        let valid = (0..6).all(|i| {
            let rad = (0..6)
                .filter(|j| *j != i)
                .map(|j| gram[(i, j)].abs())
                .sum::<f64>();
            gram[(i, i)] - rad >= 0.25 && gram[(i, i)] + rad <= 2.
        });
        let oracle = qr::Projection::new(b);
        let m = columns(rows, 1);
        let mp = m.norm_squared() / n;
        let q: [_; 4] = std::array::from_fn(|j| columns(rows, 2 + j));
        let d: [_; 4] = std::array::from_fn(|j| {
            DVector::from_iterator(length, delta[start..end].iter().map(|r| r[j]))
        });
        let w: [_; 4] = std::array::from_fn(|j| {
            DVector::from_iterator(length, wet[start..end].iter().map(|r| r[j]))
        });
        let available = valid
            && rows.iter().all(|r| r[18] != 0. && r[19] != 0.)
            && oracle.passes(&m, mp, mp, false)?;
        let mut live = valid;
        for j in 0..4 {
            live &= oracle.passes(&w[j], q[j].norm_squared() / n, mp, false)?;
        }
        live &= (&w[1] - &w[0]).norm_squared() <= 10f64.powf(-2.8) * w[0].norm_squared();
        let mut best = None;
        let mut neutral = false;
        for trial in (0..=16).rev() {
            let weight = f64::from(trial) / 16.;
            let z: [_; 3] = std::array::from_fn(|j| &q[j] + &d[j] * weight);
            let mut safe = valid;
            for j in 0..3 {
                safe &= oracle.passes(&z[j], q[j].norm_squared() / n, mp, true)?;
            }
            safe &= (&z[1] - &z[0]).norm_squared() <= 10f64.powf(-2.85) * z[0].norm_squared();
            if safe && best.is_none() {
                best = Some(weight);
            }
            if trial == 0 {
                neutral = safe;
            }
        }
        ready = if available && live && neutral {
            (ready + 1).min(2)
        } else {
            0
        };
        next = if ready == 2 { best.unwrap_or(0.) } else { 0. };
        decisions.push((end, live));
        start = end;
    }
    Ok(Trajectory {
        scale,
        wet,
        decisions,
    })
}
struct Fixture {
    frames: Vec<[f64; 20]>,
    fixed: Vec<Vec<f64>>,
    inputs: Vec<[f64; 17]>,
    independent: Vec<[f64; 4]>,
}
impl Fixture {
    fn new(out: &Path) -> Result<Self> {
        let fs = 48000;
        let n = 24000;
        let exe = replay::build(out, None)?;
        let (pitch, depth, ctl) = motion::schedule("vibrato", fs);
        let raw = carrier::scheduled(&exe, fs, n, &ctl[..n * 24])?;
        let vel: Vec<_> = raw.iter().map(|r| r[21]).collect();
        let mut fixed = vec![];
        for d in [0., 0.5, 1.] {
            fixed.push(
                scheduled_taps(&exe, 48000., &vel, |_| d)?
                    .iter()
                    .map(|r| r[3])
                    .collect(),
            );
        }
        let fourth = scheduled_taps(&exe, 48000., &vel, |i| depth[i])?;
        let mut bank = Bank::new(48000.)?;
        let mut independent = vec![];
        let mut inputs = vec![];
        let mut frames = vec![];
        for (i, r) in raw.iter().enumerate() {
            let mut f = [0.; 20];
            f[..2].copy_from_slice(&[pitch[i], r[11]]);
            f[2..5].copy_from_slice(&r[25..28]);
            f[5] = fourth[i][3];
            if (7500..12500).contains(&i) {
                f[5] += if i % 2 == 0 { 0.5 } else { -0.5 };
            }
            for pair in f[6..18].as_chunks_mut::<2>().0 {
                pair.copy_from_slice(&[-0.4376586748096509, 0.7782794100389229]);
            }
            f[18..].fill(1.);
            let mut input = [0.; 17];
            input[0] = f[0];
            input[1..].copy_from_slice(&f[2..18]);
            let d = bank.apply(&input)?;
            independent.push(d[..4].try_into().unwrap());
            inputs.push(input);
            frames.push(f);
        }
        Ok(Self {
            frames,
            fixed,
            inputs,
            independent,
        })
    }
    fn check(&self, k: &Kernel, expected: &Trajectory) -> Result<()> {
        let mut frames = self.frames.clone();
        let mut y = vec![[0.; 20]; frames.len()];
        for (j, actual) in self.fixed.iter().enumerate() {
            for (r, v) in frames.iter_mut().zip(actual) {
                r[5] = *v;
            }
            k.epoch(48000.).run(&frames, &mut y, 0)?;
            ensure(
                y.iter().zip(&frames).all(|(r, f)| {
                    r[8] != 0. || (0..4).all(|j| r[j + 4].to_bits() == f[j + 2].to_bits())
                }),
                "unsafe independent gain",
            )?;
            ensure(
                y.iter().zip(&frames).any(|(r, f)| r[4 + j] != f[2 + j])
                    && y.iter().all(|r| r[4 + j].to_bits() == r[7].to_bits()),
                "constant delivery final wet",
            )?;
        }
        k.epoch(48000.).run(&self.frames, &mut y, 0)?;
        ensure(
            y.iter()
                .zip(&self.independent)
                .all(|(r, e)| (0..4).all(|j| (r[j] - e[j]).abs() <= 2e-11)),
            "fourth-bank correction",
        )?;
        ensure(
            expected.decisions.iter().any(|(_, ok)| !*ok)
                && expected
                    .decisions
                    .iter()
                    .all(|(end, ok)| (y[end - 1][11] != 0.) == *ok),
            "delivery-only realized veto",
        )?;
        ensure(
            y[..7500].iter().any(|r| r[8] > 0.)
                && y[10000..15000].iter().any(|r| r[8] == 0.)
                && y.last().unwrap()[8] > 0.,
            "delivery veto recovery",
        )?;
        ensure(
            y.iter()
                .zip(&expected.scale)
                .all(|(r, s)| (r[8] - s).abs() <= 2e-12),
            "independent scheduled slew",
        )?;
        ensure(
            y.iter()
                .zip(&expected.wet)
                .all(|(r, s)| (0..4).all(|j| (r[j + 4] - s[j]).abs() <= 2e-11)),
            "independent wet trajectory",
        )?;
        Ok(())
    }
}
pub fn run(dir: &Path) -> Result<Value> {
    let f = Fixture::new(dir)?;
    let trajectory = drive(&f.frames, &f.independent, 48000.)?;
    f.check(&Kernel::build(dir)?, &trajectory)?;
    let original = fs::read_to_string("tools/validation/i1_stream.cpp")?;
    let mut controls = vec![];
    for (name, old, new, predicate, group) in [
        (
            "always-neutral fourth",
            "if (probe == 1)",
            "d[3] = 0.; if (probe == 1)",
            "constant delivery final wet",
            "paths",
        ),
        (
            "independent gain",
            "gain == 0 ? direct : direct + gain * delta",
            "gain == 0 ? direct + delta : direct + delta",
            "unsafe independent gain",
            "paths",
        ),
        (
            "incorrect bank coefficient",
            "d[j] + coeff * taper[k]",
            "d[j] + .97 * coeff * taper[k]",
            "fourth-bank correction",
            "paths",
        ),
        (
            "bypassed slew",
            "scale += std::clamp(target - scale, -step, step);",
            "scale = step > 0 ? target : 0.;",
            "independent scheduled slew",
            "paths",
        ),
        (
            "disabled realized veto",
            "ok && available && live ?",
            "ok && available ?",
            "delivery veto recovery",
            "paths",
        ),
        (
            "wrong phase progression",
            "INFINITY : tau * f / fs",
            "INFINITY : tau * f / fs * 1.00001",
            "packed moment",
            "qr",
        ),
        (
            "wrong candidate schedule",
            "trial = 0;",
            "trial = 1;",
            "17 candidate slots",
            "qr",
        ),
        (
            "phase poison treated as veto",
            "phase = NAN;",
            "phase = NAN; if (!std::isfinite(phase)) {phase=0;ready=0;veto=true;}",
            "lane-local numerical ownership phase",
            "fault",
        ),
    ] {
        let temp = tempfile::tempdir_in(dir)?;
        let src = temp.path().join("epoch.cpp");
        let mut code = original.clone();
        replay::edit(&mut code, old, new)?;
        fs::write(&src, code)?;
        let kernel = Kernel::from_source(temp.path(), &src)?;
        let outcome = match group {
            "qr" => qr::case(&kernel, 48000., "steady", 1.).map(|_| ()),
            "fault" => {
                let mut epoch = kernel.epoch(48000.);
                let fault = epoch.run(&f.frames[..1], &mut [[0.; 20]], 1);
                ensure(
                    fault.is_err_and(|e| e.to_string() == "epoch fault site=1 offset=0"),
                    predicate,
                )
            }
            _ => f.check(&kernel, &trajectory),
        };
        let error = outcome
            .err()
            .ok_or_else(|| format!("surviving mutant {name}"))?
            .to_string();
        ensure(
            error.contains(predicate),
            &format!("{name}: unexpected {error} instead of {predicate}"),
        )?;
        controls.push(json!({"name":name,"predicate":error,"source_sha256":evidence::digest(&src)?,"library_sha256":evidence::digest(&temp.path().join(format!("epoch{}",std::env::consts::DLL_SUFFIX)))?}));
    }
    // The earlier independent C++ bank is a distinct executable and has its own coherent mutation.
    let temp = tempfile::tempdir_in(dir)?;
    let mut code = fs::read_to_string("tools/validation/i1_motion_bank.cpp")?;
    replay::edit(
        &mut code,
        "if (std::fwrite",
        "result[3]=0.; if (std::fwrite",
    )?;
    let src = temp.path().join("bank.cpp");
    fs::write(&src, code)?;
    let exe = temp.path().join("bank");
    process::check(
        Command::new("clang++")
            .args(["-O3", "-std=c++17"])
            .arg(&src)
            .arg("-o")
            .arg(&exe),
    )?;
    let mut bytes = vec![];
    for r in &f.inputs {
        bytes.extend(r.map(f64::to_le_bytes).as_flattened());
    }
    let o = replay::stream(&exe, &["48000".into()], &bytes)?;
    ensure(
        o.stdout.len() == 48 * f.inputs.len(),
        "historical bank mutant frame count",
    )?;
    let first = o
        .stdout
        .as_chunks::<48>()
        .0
        .iter()
        .zip(&f.independent)
        .position(|(r, d)| (f64::from_le_bytes(r[24..32].try_into().unwrap()) - d[3]).abs() > 2e-11)
        .ok_or("historical fourth bank survived")?;
    controls.push(json!({"name":"Review 8.1 fourth source","predicate":"fourth-bank correction","first":first,"executable":evidence::digest(&exe)?}));
    Ok(json!({"cases":controls,"frames":f.frames.len(),"retired_decisions":trajectory.decisions}))
}
