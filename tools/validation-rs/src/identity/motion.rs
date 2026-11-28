use super::{Bank, Witness, epoch::Kernel, witness};
use crate::{
    Result,
    electrical::{carrier, scheduled_taps},
    ensure, replay,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, f64::consts::TAU, path::Path};

#[derive(Deserialize)]
struct Contract {
    controls: String,
    physical: String,
    hashes: BTreeMap<String, String>,
    monitor_sha256: String,
    windows: usize,
    witness: Option<Witness>,
    delivery_witness: Option<Witness>,
    old_witness: Option<Witness>,
}
pub(super) fn interp(t: f64, x: &[f64], y: &[f64]) -> f64 {
    let edge = x.partition_point(|v| *v <= t);
    if edge == 0 {
        return y[0];
    }
    if edge == x.len() || x[edge - 1] == t {
        return y[edge - 1];
    }
    let i = edge - 1;
    ((y[edge] - y[i]) / (x[edge] - x[i])).mul_add(t - x[i], y[i])
}
pub(super) fn schedule(kind: &str, fs: u32) -> (Vec<f64>, Vec<f64>, Vec<u8>) {
    let n = fs as usize * if kind == "lift" { 7 } else { 3 };
    let rate = f64::from(fs);
    let mut pitch = Vec::with_capacity(n);
    let mut depths = Vec::with_capacity(n);
    let mut input = Vec::with_capacity(24 * n);
    for i in 0..n {
        let t = i as f64 / rate;
        let f = match kind {
            "fast-retune" if i >= fs as usize => {
                let j = i - fs as usize;
                let length = (fs / 50) as usize;
                if j >= length - 1 {
                    440.
                } else {
                    196. + j as f64 * (244. / (length - 1) as f64)
                }
            }
            "max-retune" => (196. * (4. * (t - 1.)).max(0.).exp2()).min(1320.),
            "bend" => 196. * (200. * (1. - (t - 1.2).abs() / 0.2).max(0.) / 1200.).exp2(),
            "vibrato" => 196. * (35. * (TAU * 5. * t).sin() / 1200.).exp2(),
            _ => 196.,
        };
        let (speed, r) = if kind == "bow-controls" {
            (
                interp(
                    t,
                    &[0., 0.8, 0.85, 1.6, 1.65, 3.],
                    &[0.06, 0.06, 0.16, 0.16, 0.1, 0.1],
                ),
                interp(
                    t,
                    &[0., 1., 1.05, 2., 2.05, 3.],
                    &[1.2, 1.2, 2.4, 2.4, 1.8, 1.8],
                ),
            )
        } else {
            (0.1, 1.8)
        };
        let k = if kind == "lift" && i >= fs as usize {
            0.
        } else {
            speed * r / 1.65
        };
        let depth = if matches!(kind, "held" | "steady") {
            0.5
        } else if matches!(kind, "veto" | "vibrato" | "fast-retune") && (1.0..1.6).contains(&t) {
            ((i / 7) % 2) as f64
        } else {
            interp(t, &[0., 0.5, 1., 1.5, 2., 3.], &[0., 1., 0., 1., 0., 0.5])
        };
        pitch.push(f);
        depths.push(depth);
        for v in [f, speed, k] {
            input.extend(v.to_le_bytes());
        }
    }
    (pitch, depths, input)
}
fn digest_rows<const N: usize>(rows: &[[f64; N]]) -> String {
    let mut h = Sha256::new();
    for row in rows {
        h.update(row.map(f64::to_le_bytes).as_flattened());
    }
    format!("{:x}", h.finalize())
}
fn words(h: &mut Sha256, row: &[f64]) {
    for v in row {
        h.update(v.to_le_bytes());
    }
}
fn case(
    out: &Path,
    kind: &str,
    fs: u32,
    c: &Contract,
    exe: &Path,
    kernel: &Kernel,
) -> Result<Value> {
    let rate = f64::from(fs);
    let (pitch, depth, controls) = schedule(kind, fs);
    let n = pitch.len();
    ensure(
        format!("{:x}", Sha256::digest(&controls)) == c.controls,
        "physical controls",
    )?;
    let raw = carrier::scheduled(exe, fs, n, &controls)?;
    ensure(digest_rows(&raw) == c.physical, "physical trace identity")?;
    let directory = out.join("motion");
    let name = format!("{kind}-{fs}");
    super::artifacts::physical(&directory, &name, exe, fs, &controls, &raw)?;
    let velocity: Vec<_> = raw.iter().map(|r| r[21]).collect();
    let delivery = scheduled_taps(exe, rate, &velocity, |i| depth[i])?;
    let mut hashes: [Sha256; 7] = std::array::from_fn(|_| Sha256::new());
    let alt: [f64; 12] = std::array::from_fn(|i| {
        if i % 2 == 0 {
            -0.4376586748096509
        } else {
            0.7782794100389229
        }
    });
    let smoother = -(-1. / (0.015 * rate)).exp_m1();
    let mut state = [0.; 12];
    if kind != "veto" {
        words(&mut hashes[2], &alt);
    }
    let mut frames = Vec::with_capacity(n);
    for i in 0..n {
        let mut r = [0.; 20];
        r[..2].copy_from_slice(&[pitch[i], raw[i][11]]);
        r[2..5].copy_from_slice(&raw[i][25..28]);
        r[5] = delivery[i][3];
        r[6..18].copy_from_slice(&alt);
        if kind == "veto" {
            for k in 0..12 {
                let x = if (fs as usize..2 * fs as usize).contains(&i) {
                    alt[(k + 1) % 2]
                } else {
                    alt[k]
                };
                r[k + 6] = smoother.mul_add(x, state[k]);
                state[k] = (1. - smoother) * r[k + 6];
            }
            words(&mut hashes[2], &r[6..18]);
        }
        r[18] = f64::from(
            !(kind == "veto"
                && ((1.35 * rate).round_ties_even() as usize
                    ..(1.55 * rate).round_ties_even() as usize)
                    .contains(&i)),
        );
        r[19] = 1.;
        hashes[0].update(pitch[i].to_le_bytes());
        hashes[1].update(depth[i].to_le_bytes());
        words(&mut hashes[3], &r[2..6]);
        frames.push(r);
    }
    let mut output = vec![[0.; 20]; n];
    kernel.epoch(rate).run(&frames, &mut output, 0)?;
    let legacy_scale = super::legacy::scales(&frames, &output, rate)?;
    let mut legacy_hash = Sha256::new();
    words(&mut legacy_hash, &legacy_scale);
    let legacy_hash = format!("{:x}", legacy_hash.finalize());
    ensure(
        c.hashes["old_scale"] == legacy_hash,
        "historical fixed-bin permission",
    )?;
    {
        use super::artifacts::array;
        let request = if kind == "veto" {
            array(
                "request",
                n,
                Some(12),
                frames.iter().flat_map(|r| r[6..18].iter().copied()),
            )?
        } else {
            array("request", 12, None, alt)?
        };
        let entries = [
            array("pitch", n, None, pitch.iter().copied())?,
            array("depth", n, None, depth.iter().copied())?,
            request,
            array(
                "q",
                n,
                Some(4),
                frames.iter().flat_map(|r| r[2..6].iter().copied()),
            )?,
            array(
                "d",
                n,
                Some(4),
                output.iter().flat_map(|r| r[..4].iter().copied()),
            )?,
            array(
                "wet",
                n,
                Some(4),
                output.iter().flat_map(|r| r[4..8].iter().copied()),
            )?,
            array("scale", n, None, output.iter().map(|r| r[8]))?,
            array("old_scale", n, None, legacy_scale.iter().copied())?,
        ];
        crate::electrical::acoustic::listening::canonical_npz(
            &entries,
            &directory.join(format!("{name}.npz")),
        )?;
    }
    if fs == 48000 && matches!(kind, "steady" | "vibrato" | "bend" | "veto") {
        let n = 2 * fs as usize;
        let mut clips = Vec::new();
        if kind == "steady" {
            clips.push((
                "previous-stationary",
                (0..n)
                    .map(|i| frames[i][2] + legacy_scale[i] * output[i][0])
                    .collect::<Vec<_>>(),
            ));
        } else {
            let label = match kind {
                "vibrato" => "revised-vibrato",
                "bend" => "revised-bend",
                _ => "depth-veto-recovery",
            };
            clips.push((label, output[..n].iter().map(|r| r[7]).collect()));
            if kind == "vibrato" {
                clips.push((
                    "neutral-vibrato",
                    frames[..n].iter().map(|r| r[5]).collect(),
                ));
            }
        }
        for (name, x) in clips {
            let bytes = x.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>();
            std::fs::write(out.join(format!("{name}.f64le")), bytes)?;
        }
    }
    if fs == 48000 && matches!(kind, "steady" | "veto" | "vibrato" | "bend" | "held") {
        use std::io::Write;
        let mut samples = std::io::BufWriter::new(std::fs::File::create(
            out.join(format!("{kind}-evidence.f64le")),
        )?);
        for (f, r) in frames.iter().zip(&output) {
            let row: [f64; 13] = [
                f[2], f[3], f[4], f[5], r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8],
            ];
            samples.write_all(row.map(f64::to_le_bytes).as_flattened())?;
        }
        samples.flush()?;
    }
    let mut corrected = super::candidate::Differential::new(rate);
    let mut corrected_delta = Vec::with_capacity(n);
    let mut bank = Bank::new(rate)?;
    let mut maximum = 0.0f64;
    let mut delta = Vec::with_capacity(n);
    let mut corrections = Vec::with_capacity(n);
    let mut monitor = Sha256::new();
    let mut start = 0;
    let mut windows = 0;
    let mut timeline = Vec::new();
    for (i, r) in output.iter().enumerate() {
        let f = &frames[i];
        let revised = corrected.check(f, r)?;
        corrected_delta.push([revised[4] - f[2], revised[7] - f[5]]);
        let mut b = [0.; 17];
        b[0] = f[0];
        b[1..].copy_from_slice(&f[2..18]);
        let independent = bank.apply(&b)?;
        corrections.push(independent[..4].try_into().unwrap());
        for j in 0..4 {
            let err = (independent[j] - r[j]).abs();
            ensure(err <= 2e-11, "fourth-bank correction")?;
            maximum = maximum.max(err);
            if r[8] == 0. {
                ensure(
                    r[4 + j].to_bits() == f[j + 2].to_bits(),
                    "exact neutral passthrough",
                )?;
            }
        }
        words(&mut hashes[4], &r[..4]);
        hashes[5].update(r[8].to_le_bytes());
        words(&mut hashes[6], &r[4..8]);
        delta.push([r[4] - f[2], r[7] - f[5]]);
        if r[10] != 0. {
            let end = i + 1;
            let done = end + 16;
            for (j, next) in output[end..n.min(done + 1)].iter().enumerate() {
                ensure(next[9] == j as f64, "candidate schedule")?;
            }
            let selected = output.get(done);
            let observable = frames[start..end].iter().all(|row| row[18] != 0.);
            timeline.push(json!({"start":start,"end":end,"conditioned":r[12]!=0.,"bridge_ok":r[13]!=0.,"available":r[14]!=0.,"realized_ok":r[11]!=0.,"observable":observable,"trusted":true,"selection_tick":selected.map(|_|done),"veto":selected.map(|s|s[15]!=0.),"scale":selected.map(|s|s[16])}));
            words(
                &mut monitor,
                &[
                    start as f64,
                    end as f64,
                    r[12],
                    r[13],
                    r[14],
                    r[11],
                    selected.map_or(-1., |_| done as f64),
                    selected.map_or(-1., |x| x[15]),
                    selected.map_or(-1., |x| x[16]),
                ],
            );
            start = end;
            windows += 1;
        }
    }
    let oracle = super::paths::drive(&frames, &corrections, rate)?;
    let mut slew_error = 0f64;
    let mut wet_error = 0f64;
    for (i, r) in output.iter().enumerate() {
        let e = (r[8] - oracle.scale[i]).abs();
        ensure(
            e <= 2e-12,
            &format!(
                "{kind}/{fs} independent permission at {i}, expected {}, actual {}",
                oracle.scale[i], r[8]
            ),
        )?;
        slew_error = slew_error.max(e);
        for j in 0..4 {
            let e = (r[4 + j] - oracle.wet[i][j]).abs();
            ensure(e <= 2e-11, "independent motion wet trajectory")?;
            wet_error = wet_error.max(e);
        }
    }
    ensure(
        oracle.decisions.len() == windows
            && oracle
                .decisions
                .iter()
                .all(|(end, ok)| output[end - 1][10] != 0. && ((output[end - 1][11] != 0.) == *ok)),
        "full-window realized decisions",
    )?;
    let digests: BTreeMap<_, _> = ["pitch", "depth", "request", "q", "d", "scale", "wet"]
        .into_iter()
        .zip(hashes)
        .map(|(k, h)| (k.to_string(), format!("{:x}", h.finalize())))
        .collect();
    for (k, v) in &digests {
        ensure(c.hashes.get(k) == Some(v), &format!("{k} sample identity"))?;
    }
    ensure(
        format!("{:x}", monitor.finalize()) == c.monitor_sha256 && windows == c.windows,
        "retired-window dispositions",
    )?;
    let mut row = json!({"stage_08_3":{"changed_wet_words":corrected.changed_wet,"changed_f32":corrected.changed_audio},"frames":n,"windows":windows,"hashes":digests,"full_window_slew_error":slew_error,"full_window_wet_error":wet_error,"max_bank_error":maximum,"monitor_sha256":c.monitor_sha256});
    if kind == "steady" {
        let tail = n - fs as usize;
        let power = frames[tail..]
            .iter()
            .zip(&output[tail..])
            .zip(&legacy_scale[tail..])
            .map(|((r, d), s)| ((r[2] + s * d[0]) - r[2]).powi(2))
            .sum::<f64>();
        row["historical_stationary_calibration"] = (power / rate).sqrt().into();
    }
    let mut ccase = carrier::cases()?
        .remove("steady_48000_196_R1.8_V0.1_3s")
        .ok_or("diagnostic template")?;
    ccase.rate = fs;
    ccase.trajectory = kind.into();
    let mut metrics =
        super::motion_report::measures(kind, &ccase, &frames, &output, &legacy_scale)?;
    metrics["monitor"] = timeline.into();
    metrics["physical"] = c.physical.clone().into();
    metrics["trace"] = crate::evidence::digest(&directory.join(format!("{name}.npz")))?.into();
    if kind == "steady" {
        metrics["prerequisite"] = super::motion_report::prerequisite(&raw, fs, true)?;
        ensure(
            metrics["prerequisite"]["passed"] == true,
            "nominal upstream prerequisite",
        )?;
        if fs == 48000 {
            metrics["upstream_controls"] = super::motion_report::defects(&raw, kernel)?;
        }
    }
    row["measurements"] = metrics;
    row["old_scale_sha256"] = legacy_hash.into();
    if kind == "vibrato" {
        let historical: Vec<_> = frames
            .iter()
            .zip(&output)
            .zip(legacy_scale)
            .map(|((frame, result), scale)| [(frame[2] + scale * result[0]) - frame[2], 0.])
            .collect();
        for (j, k) in ["off", "delivery"].iter().enumerate() {
            let old = if j == 0 {
                &c.witness
            } else {
                &c.delivery_witness
            };
            let result = witness(
                &corrected_delta,
                fs,
                j,
                old.as_ref().ok_or("missing corrected motion witness")?,
            )?;
            ensure(result["active"] == 100, "corrected registered motion cells")?;
            row["stage_08_3"][*k] = result;
        }
        row["old_off"] = witness(
            &historical,
            fs,
            0,
            c.old_witness.as_ref().ok_or("missing historical witness")?,
        )?;
        row["off"] = witness(
            &delta,
            fs,
            0,
            c.witness.as_ref().ok_or("missing off witness")?,
        )?;
        row["delivery"] = witness(
            &delta,
            fs,
            1,
            c.delivery_witness
                .as_ref()
                .ok_or("missing delivery witness")?,
        )?;
    }
    Ok(row)
}
pub fn run(out: &Path) -> Result<Value> {
    let contract: BTreeMap<String, Contract> =
        serde_json::from_str(include_str!("../../contracts/motion.json"))?;
    let exe = replay::build(out, None)?;
    let kernel = Kernel::build(out)?;
    let mut rows = BTreeMap::new();
    for (name, c) in contract {
        let (kind, rate) = name.rsplit_once('-').ok_or("motion case")?;
        rows.insert(
            name.clone(),
            case(out, kind, rate.parse()?, &c, &exe, &kernel)
                .map_err(|e| format!("{name}: {e}"))?,
        );
    }
    ensure(rows.len() == 27, "motion inventory")?;
    Ok(json!({"cases":rows}))
}
