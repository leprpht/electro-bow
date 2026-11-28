use super::epoch::Kernel;
use crate::{Result, ensure};
use nalgebra::{DMatrix, DVector};
use serde_json::{Value, json};
use std::{
    f64::consts::{SQRT_2, TAU},
    path::Path,
};

pub(super) fn basis(angles: &[f64]) -> DMatrix<f64> {
    DMatrix::from_fn(angles.len(), 6, |i, j| match j {
        0 => 1.,
        1 => angles[i].cos(),
        2 => angles[i].sin(),
        3 => (2. * angles[i]).cos(),
        4 => (2. * angles[i]).sin(),
        _ => {
            if i % 2 == 0 {
                1.
            } else {
                -1.
            }
        }
    })
}
fn streams(input: &[[f64; 20]], out: &[[f64; 20]]) -> DMatrix<f64> {
    DMatrix::from_fn(input.len(), 13, |i, j| {
        if j == 12 {
            return input[i][1];
        }
        let p = j / 3;
        match j % 3 {
            0 => input[i][2 + p],
            1 => out[i][p],
            _ => out[i][4 + p],
        }
    })
}
fn moments(b: &DMatrix<f64>, x: &DMatrix<f64>, out: &[[f64; 20]]) -> Vec<f64> {
    let g = b.transpose() * b;
    let mut sums = Vec::with_capacity(123);
    for i in 0..6 {
        for j in 0..=i {
            sums.push(g[(i, j)]);
        }
    }
    sums.extend((b.transpose() * x).iter().copied());
    for j in 0..4 {
        let q = x.column(3 * j);
        let d = x.column(3 * j + 1);
        sums.extend([q.norm_squared(), q.dot(&d), d.norm_squared()]);
    }
    for j in 0..4 {
        sums.push(x.column(3 * j + 2).norm_squared());
    }
    let q = x.column(3) - x.column(0);
    let d = x.column(4) - x.column(1);
    let paired = x.column(5) - x.column(2);
    let pre = x.column(9)
        + DVector::from_iterator(out.len(), out.iter().map(|r| r[18])).component_mul(&x.column(10));
    sums.extend([
        x.column(12).norm_squared(),
        q.norm_squared(),
        q.dot(&d),
        d.norm_squared(),
        paired.norm_squared(),
        pre.norm_squared(),
        x.column(9).dot(&x.column(11)),
        x.column(10).dot(&x.column(11)),
    ]);
    sums
}
pub(super) struct Projection {
    basis: DMatrix<f64>,
    qt: DMatrix<f64>,
    r: DMatrix<f64>,
}
impl Projection {
    pub(super) fn new(basis: DMatrix<f64>) -> Self {
        let (q, r) = basis.clone().qr().unpack();
        Self {
            basis,
            qt: q.transpose(),
            r,
        }
    }
    fn fit(&self, x: &DVector<f64>) -> Result<DVector<f64>> {
        self.r
            .solve_upper_triangular(&(&self.qt * x))
            .ok_or_else(|| "singular QR".into())
    }
    pub(super) fn passes(
        &self,
        x: &DVector<f64>,
        neutral: f64,
        m5: f64,
        recovery: bool,
    ) -> Result<bool> {
        let n = x.len() as f64;
        let a = self.fit(x)?;
        let residual = (x - &self.basis * &a).norm() / n.sqrt();
        let h = |i| (self.basis.columns(i, 2) * a.rows(i, 2)).norm() / n.sqrt();
        let p = x.norm_squared() / n;
        let margin = 10f64.powf(if recovery { 0.30 } else { 0.05 } / 20.);
        let factor = 10f64.powf(if recovery { 2.85 } else { 3. } / 10.);
        Ok(neutral.min(m5) > 1e-24
            && p >= neutral.max(m5) / factor
            && p <= neutral.min(m5) * factor
            && h(1) >= margin * h(3).max(SQRT_2 * a[5].abs()).max(residual) + 1e-12)
    }
}
pub(super) fn case(kernel: &Kernel, fs: f64, kind: &str, ratio: f64) -> Result<Value> {
    let frequency = if kind == "degenerate" { 1320. } else { 196. };
    let n = 2 * ((4. * fs / frequency + 0.5).floor() as usize);
    let mut angles = Vec::new();
    let mut input = Vec::new();
    let mut phase = 0.;
    for i in 0..n + 17 {
        let scaled = match kind {
            "vibrato" | "bend" | "max-retune" => {
                let range = if kind == "vibrato" { 35. } else { 200. } / 1200.;
                frequency * (i as f64 * (range / (n + 16) as f64)).exp2()
            }
            "degenerate" if (1..n).contains(&i) => 65.,
            _ => frequency,
        };
        phase += scaled * (TAU / fs);
        angles.push(phase);
        let m = phase.cos();
        let mut frame = [0.; 20];
        frame[0] = scaled;
        frame[1] = m;
        frame[2..6].fill(ratio * m);
        if !kind.starts_with("threshold") {
            frame[3] += 0.004 * (2. * phase + 0.7).cos();
            frame[4] += 0.006 * (2. * phase).sin();
            frame[5] += 0.005 * (2. * phase + 0.3).sin() * (i as f64 * 0.03).cos();
        }
        for pair in frame[6..18].as_chunks_mut::<2>().0 {
            pair.copy_from_slice(&[-0.38, 0.77]);
        }
        frame[18..].fill(1.);
        input.push(frame);
    }
    let mut epoch = kernel.epoch(fs);
    let mut output = vec![[0.; 20]; n + 17];
    epoch.run(&input[..n - 1], &mut output[..n - 1], 0)?;
    let stats = epoch.stats(false);
    let x = streams(&input[..n - 1], &output[..n - 1]);
    let expected = moments(&basis(&angles[..n - 1]), &x, &output[..n - 1]);
    let mut moment_error = 0.0f64;
    for (i, (s, e)) in stats.iter().zip(expected).enumerate() {
        let delta = (s - e).abs() / (n - 1) as f64;
        ensure(delta <= 2e-11, &format!("packed moment {i} error {delta}"))?;
        moment_error = moment_error.max(delta);
    }
    epoch.run(&input[n - 1..], &mut output[n - 1..], 0)?;
    ensure(
        output[n..]
            .iter()
            .enumerate()
            .all(|(j, r)| r[9] == j as f64),
        "17 candidate slots",
    )?;
    ensure(
        output[..n - 1]
            .iter()
            .zip(&angles)
            .all(|(r, a)| (r[17] - a).abs() <= 2e-10),
        "committed phase",
    )?;
    if kind == "degenerate" {
        ensure(
            output[n - 1][12] == 0. && output.iter().all(|r| r[8] == 0.),
            "finite rank veto",
        )?;
        return Ok(json!({"kind":kind,"ordinary_veto":true,"moment_error":moment_error}));
    }
    let b = basis(&angles[..n]);
    let (q, r) = b.clone().qr().unpack();
    let oracle = Projection {
        basis: b,
        qt: q.transpose(),
        r,
    };
    let stats = epoch.stats(true);
    let x = streams(&input[..n], &output[..n]);
    let mut fit_error = 0.0f64;
    let mut residual_error = 0.0f64;
    for j in 0..13 {
        let signal = x.column(j).into_owned();
        let fit = oracle.fit(&signal)?;
        let packed = DVector::from_column_slice(&stats[21 + 6 * j..27 + 6 * j]);
        fit_error = fit_error.max((&fit - &packed).amax());
        let residual = (&signal - &oracle.basis * fit).norm_squared() / n as f64;
        let reference =
            (signal.norm_squared() - (&oracle.basis * packed).norm_squared()) / n as f64;
        residual_error = residual_error.max((residual - reference).abs());
    }
    ensure(
        fit_error <= 2e-10 && residual_error <= 2e-11,
        "full-window QR fit/residual",
    )?;
    let m = x.column(12).norm_squared() / n as f64;
    let mut masks = vec![];
    for tick in 0..17 {
        let w = (16 - tick) as f64 / 16.;
        let z = |j| x.column(3 * j) + w * x.column(3 * j + 1);
        let mut accept = true;
        for j in 0..3 {
            accept &= oracle.passes(&z(j), x.column(3 * j).norm_squared() / n as f64, m, true)?;
        }
        accept &= (z(1) - z(0)).norm_squared() <= 10f64.powf(-2.85) * z(0).norm_squared();
        ensure(
            f64::from(accept) == output[n + tick][19],
            "direct candidate classification",
        )?;
        masks.push(accept);
    }
    let mut live = true;
    for j in 0..4 {
        live &= oracle.passes(
            &x.column(3 * j + 2).into_owned(),
            x.column(3 * j).norm_squared() / n as f64,
            m,
            false,
        )?;
    }
    live &=
        (x.column(5) - x.column(2)).norm_squared() <= 10f64.powf(-2.8) * x.column(2).norm_squared();
    ensure(
        f64::from(live) == output[n - 1][11],
        "direct realized decision",
    )?;
    Ok(
        json!({"rate":fs,"kind":kind,"ratio":ratio,"moment_error":moment_error,"fit_error":fit_error,"residual_error":residual_error,"mask":masks,"live":live}),
    )
}
pub fn run(out: &Path) -> Result<Value> {
    let kernel = Kernel::build(out)?;
    let threshold = 10f64.powf(0.1425);
    let mut rows = Vec::new();
    for (fs, kind, r) in [
        (48000., "steady", 1.),
        (47001., "bend", 1.),
        (192000., "vibrato", 1.),
        (32000., "max-retune", 1.),
        (48000., "degenerate", 1.),
        (48000., "threshold-low", threshold * (1. - 2e-12)),
        (48000., "threshold-high", threshold * (1. + 2e-12)),
    ] {
        rows.push(case(&kernel, fs, kind, r).map_err(|e| format!("{kind}/{fs}: {e}"))?);
    }
    Ok(rows.into())
}
