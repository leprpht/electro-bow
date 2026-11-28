//! Reject changed trajectories and source evidence using their real prerequisite and sample windows.
use super::{Bank, legacy, paths};
use crate::{Result, ensure, evidence, publication};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, io::Read, path::Path};
fn words(bytes: &[u8]) -> Result<Vec<f64>> {
    let (x, tail) = bytes.as_chunks::<8>();
    ensure(tail.is_empty(), "binary64 sample storage")?;
    Ok(x.iter().map(|w| f64::from_le_bytes(*w)).collect())
}
fn npy(z: &mut zip::ZipArchive<fs::File>, name: &str, n: usize) -> Result<Vec<f64>> {
    let mut file = z.by_name(&format!("{name}.npy"))?;
    let mut b = Vec::new();
    file.read_to_end(&mut b)?;
    ensure(
        b.len() >= 10 && b[..8] == *b"\x93NUMPY\x01\x00",
        "NPY version",
    )?;
    let start = 10 + usize::from(u16::from_le_bytes([b[8], b[9]]));
    ensure(
        start <= b.len() && b.len() - start == 8 * n,
        "NPY sample inventory",
    )?;
    words(&b[start..])
}
fn checked_nonnegative(v: f64, magnitude: f64, count: usize) -> Result<f64> {
    let tolerance = 64. * count as f64 * f64::EPSILON * magnitude;
    ensure(
        tolerance.is_finite() && v.is_finite() && v >= -tolerance,
        "nonfinite or negative moment",
    )?;
    Ok(v.max(0.))
}
fn trajectory(scale: &[f64], ramp: usize) -> Result<()> {
    ensure(
        scale.iter().all(|v| (0. ..=1.).contains(v))
            && scale
                .windows(2)
                .all(|w| (w[1] - w[0]).abs() <= 1. / ramp as f64 + 1e-12),
        "smoothing trajectory",
    )
}
pub fn run(root: &Path) -> Result<Value> {
    let suite = "i1-motion";
    let receipt = publication::inspect(&root.join(suite), suite)?;
    ensure(receipt["status"] == "passed", "fresh motion measurement")?;
    let base = root
        .join(suite)
        .join(receipt["run_id"].as_str().ok_or("run id")?)
        .join("motion");
    let mut result = BTreeMap::new();
    let mut abrupt = None;
    for kind in ["veto", "fast-retune", "steady"] {
        let n = 144000;
        let key = format!("{kind}-48000");
        let mut z = zip::ZipArchive::new(fs::File::open(base.join(format!("{key}.npz")))?)?;
        let q = npy(&mut z, "q", n * 4)?;
        let d = npy(&mut z, "d", n * 4)?;
        let pitch = npy(&mut z, "pitch", n)?;
        let raw = words(&fs::read(base.join(format!("{key}.f64le")))?)?;
        let mut f = vec![[0.; 20]; n];
        let mut delta = Vec::with_capacity(n);
        for i in 0..n {
            f[i][0] = pitch[i];
            f[i][1] = raw[30 * i + 11];
            f[i][2..6].copy_from_slice(&q[4 * i..4 * i + 4]);
            f[i][18] = f64::from(kind != "veto" || !(64800..74400).contains(&i));
            f[i][19] = f64::from(kind != "fast-retune");
            delta.push(d[4 * i..4 * i + 4].try_into()?);
        }
        if kind == "steady" {
            let mut bank = Bank::new(48000.)?;
            let mut out = vec![[0.; 20]; n];
            let mut x = [0.; 17];
            x[0] = 196.;
            x[5..].fill(10f64.powf(0.25) - 1.);
            for (r, v) in f.iter().zip(&mut out) {
                x[1..5].copy_from_slice(&r[2..6]);
                v[..4].copy_from_slice(&bank.apply(&x)?[..4]);
            }
            let mut scale = legacy::scales(&f, &out, 48000.)?;
            let t = (96000..144000)
                .max_by(|i, j| out[*i][0].abs().total_cmp(&out[*j][0].abs()))
                .unwrap();
            let jump = scale[t] * out[t][0];
            ensure(
                (jump + 0.16903128689).abs() < 1e-6,
                "historical abrupt neutralization",
            )?;
            scale[t..].fill(0.);
            let failure = trajectory(&scale, 240).unwrap_err().to_string();
            abrupt = Some(json!({"tick":t,"jump_FS":jump,"rejection":failure}));
        } else {
            if kind == "veto" {
                f[0][2..6].copy_from_slice(&[-0., 0., -0., 0.]);
            } else {
                f[48000][2..6].copy_from_slice(&[-0., 0., 1e-27, -0.25]);
            }
            let oracle = paths::drive(&f, &delta, 48000.)?;
            for ((r, w), s) in f.iter().zip(&oracle.wet).zip(&oracle.scale) {
                if *s == 0. {
                    ensure(
                        (0..4).all(|j| r[2 + j].to_bits() == w[j].to_bits()),
                        "signed neutral passthrough",
                    )?;
                }
            }
            if kind == "veto" {
                ensure(
                    oracle.scale[..48000].iter().any(|v| *v > 0.)
                        && oracle.scale[96000..].iter().any(|v| *v > 0.),
                    "independent veto recovery",
                )?;
            } else {
                ensure(oracle.scale.iter().all(|v| *v == 0.), "untrusted retune")?;
            }
        }
    }
    ensure(
        checked_nonnegative(-1e-16, 1., 200)? == 0.
            && [-0.1, f64::NAN, f64::INFINITY]
                .iter()
                .all(|v| checked_nonnegative(*v, 1., 200).is_err()),
        "moment roundoff classification",
    )?;
    for key in [
        "always-neutral",
        "infinitesimal",
        "single-cell",
        "stationary-ends",
    ] {
        let mut d = vec![[0.; 2]; 144000];
        match key {
            "infinitesimal" => d.fill([0.0066e-6, 0.]),
            "single-cell" => d[48000..48960].fill([0.0066, 0.]),
            "stationary-ends" => {
                d[..24000].fill([0.0066, 0.]);
                d[120000..].fill([0.0066, 0.]);
            }
            _ => {}
        }
        let r = super::score(&d, 48000, 0, 0.0066)?;
        ensure(r["passed"] == false, "false positive moving witness")?;
        result.insert(
            key,
            json!({"active_cells":r["active"],"longest_gap_ms":r["longest_gap_ms"]}),
        );
    }
    result.insert("abrupt_neutralization", abrupt.ok_or("slew control")?);
    result.insert(
        "neutral",
        json!({"signed_zero":true,"neutral_retune":true,"veto_recovery":true}),
    );
    let rec = evidence::strict(&fs::read(publication::current_artifact(
        root,
        "recording-controls",
        "evidence.json",
    )?)?)?;
    Ok(json!({"sample_controls":result,"recording_controls":rec["results"]["controls"]}))
}
