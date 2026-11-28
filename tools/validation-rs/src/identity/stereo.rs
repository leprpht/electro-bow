//! Reconstruct the two-lane transaction downstream of the sparse capture publication boundary.
use super::{epoch::Kernel, motion::interp};
use crate::{
    Result,
    electrical::{carrier, scheduled_taps},
    ensure, replay,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, f64::consts::TAU, path::Path};
const N: usize = 18500;
const RATE: f64 = 48000.;
#[derive(Deserialize)]
struct Fixture {
    schedule: Vec<(usize, Option<usize>, [f64; 12])>,
    samples: String,
    gains: String,
    columns: Vec<String>,
    right: Option<String>,
}
struct Carrier {
    rows: Vec<[f64; 30]>,
    delivery: Vec<[f64; 8]>,
    pitch: Vec<f64>,
    depths: Vec<f64>,
}
fn render(exe: &Path, epoch: usize) -> Result<Carrier> {
    let mut pitch = Vec::with_capacity(N);
    let mut depths = Vec::with_capacity(N);
    let mut input = Vec::with_capacity(N * 24);
    for i in 0..N {
        let f = if epoch == 3 {
            196.
        } else {
            196. * (35. * (TAU * 5. * i as f64 / RATE).sin() / 1200.).exp2()
        };
        pitch.push(f);
        depths.push(0.5 + 0.45 * (TAU * 7. * i as f64 / RATE).sin());
        for v in [
            f,
            0.1,
            if epoch == 1 && i >= 8000 {
                0.
            } else {
                1.8 * 0.1 / 1.65
            },
        ] {
            input.extend(v.to_le_bytes());
        }
    }
    let rows = carrier::scheduled(exe, 48000, N, &input)?;
    let velocity = rows.iter().map(|r| r[21]).collect::<Vec<_>>();
    let delivery = scheduled_taps(exe, RATE, &velocity, |i| depths[i])?;
    Ok(Carrier {
        rows,
        delivery,
        pitch,
        depths,
    })
}
fn hash<const K: usize>(rows: &[[f64; K]]) -> String {
    let mut h = Sha256::new();
    for r in rows {
        h.update(r.map(f64::to_le_bytes).as_flattened());
    }
    format!("{:x}", h.finalize())
}
pub(super) fn envelope(i: usize) -> f64 {
    interp(
        i as f64,
        &[
            0., 479., 863., 9479., 9719., 10103., 13000., 14680., 15599., 15983., 18500.,
        ],
        &[0., 0., 1., 1., 0., 1., 1., 0., 0., 1., 1.],
    )
}
fn fault(name: &str) -> Option<(usize, i32)> {
    Some(match name {
        "candidate" => (479, 2),
        "moment" => (6100, 5),
        "bank" => (7800, 3),
        "wet" => (9000, 9),
        "conversion" => (9911, 10),
        "delivery-state" => (6000, 21),
        "pre-guide" => (5600, 20),
        "post-o2" => (5600, 21),
        "reference" => return None,
        _ => unreachable!(),
    })
}
struct Lane<'a> {
    kernel: &'a Kernel,
    current: Option<super::epoch::Epoch<'a>>,
    owner: Option<usize>,
    pointer: usize,
    target: [f64; 12],
    coefficients: [f64; 12],
    held: [f64; 2],
    fade: Option<usize>,
    losses: usize,
    schedule_index: usize,
    failed_epoch: Option<usize>,
}
impl<'a> Lane<'a> {
    fn new(kernel: &'a Kernel) -> Self {
        Self {
            kernel,
            current: None,
            owner: None,
            pointer: 0,
            target: [0.; 12],
            coefficients: [0.; 12],
            held: [0.; 2],
            fade: None,
            losses: 0,
            schedule_index: 0,
            failed_epoch: None,
        }
    }
    fn tick(
        &mut self,
        t: usize,
        schedule: &super::stereo_clock::Schedule,
        paths: &[Carrier],
        name: &str,
        signed: f64,
    ) -> Result<([f64; 22], [f64; 24])> {
        if let Some(&(at, epoch, vector)) = schedule.get(self.schedule_index)
            && at == t
        {
            self.schedule_index += 1;
            if epoch != self.owner && (epoch.is_none() || epoch != self.failed_epoch) {
                self.owner = epoch;
                self.current = epoch.map(|_| self.kernel.epoch(RATE));
                self.pointer = 0;
                self.held = [0.; 2];
                self.coefficients = [0.; 12];
            }
            self.target = vector;
        }
        let mut row = [0.; 22];
        let mut gains = [0.; 24];
        row[1] = -1.;
        row[13] = -1.;
        row[14] = 1.;
        row[15] = -1.;
        if let (Some(epoch), Some(state)) = (self.owner, &mut self.current) {
            let path = &paths[epoch - 1];
            let i = self.pointer;
            row[0] = epoch as f64;
            row[1] = i as f64;
            let probe = fault(name)
                .filter(|(when, _)| t == *when || t == when + 4)
                .map_or(0, |(_, probe)| probe);
            if probe != 20 {
                row[2] = 1.;
                self.pointer += 1;
                if !(probe == 21 && name == "post-o2") {
                    row[3] = 1.;
                    row[5] = path.depths[i];
                    row[6] = path.delivery[i][3];
                }
            }
            let mut live_q = [path.rows[i][25], path.rows[i][26], path.rows[i][27], row[6]];
            if probe == 21 && name == "delivery-state" {
                live_q[3] = f64::NAN;
            }
            let mut output = [[0.; 20]];
            let result = if probe == 20
                || (probe == 21 && name == "post-o2")
                || !live_q.iter().all(|v| v.is_finite())
            {
                Err(format!("epoch fault site={probe} offset=0").into())
            } else {
                let mut frame = [0.; 20];
                frame[..2].copy_from_slice(&[path.pitch[i], path.rows[i][11]]);
                frame[2..6].copy_from_slice(&live_q);
                for ((a, goal), v) in self
                    .coefficients
                    .iter_mut()
                    .zip(self.target)
                    .zip(&mut frame[6..18])
                {
                    *a += -(-1. / (0.015 * RATE)).exp_m1() * (goal - *a);
                    *v = *a;
                }
                frame[18] = f64::from(!(6200..7100).contains(&t));
                frame[19] = 1.;
                state.run(&[frame], &mut output, probe)
            };
            match result {
                Ok(()) => {
                    let v = output[0];
                    row[7] = v[8];
                    row[8] = v[7];
                    row[9] = envelope(t);
                    row[10] = row[9] * (signed * row[8]);
                    self.held = [row[9], row[10]];
                    row[14] = v[15];
                    for (dest, index) in row[15..].iter_mut().zip([9, 10, 17, 18, 4, 5, 6]) {
                        *dest = v[index];
                    }
                    gains[..12].copy_from_slice(&self.target);
                    gains[12..].copy_from_slice(&self.coefficients);
                }
                Err(error) => {
                    ensure(
                        fault(name) == Some((t, probe))
                            && error.to_string() == format!("epoch fault site={probe} offset=0"),
                        &format!("{name} unexpected fault at {t}: {error}"),
                    )?;
                    self.failed_epoch = self.owner;
                    self.current = None;
                    self.owner = None;
                    self.losses += 1;
                    self.fade = Some(0);
                }
            }
        }
        if let Some(index) = self.fade {
            let w = (239 - index) as f64 / 240.;
            row[13] = index as f64;
            row[9] = self.held[0] * w;
            row[10] = self.held[1] * w;
            if index == 239 {
                self.fade = None;
                self.held = [0.; 2];
            } else {
                self.fade = Some(index + 1);
            }
        }
        row[4] = self.losses as f64;
        row[11..13].copy_from_slice(&self.held);
        Ok((row, gains))
    }
}
pub fn run(out: &Path) -> Result<Value> {
    let fixtures: BTreeMap<String, Fixture> =
        serde_json::from_str(include_str!("../../contracts/stereo.json"))?;
    let exe = replay::build(out, None)?;
    let kernel = Kernel::build(out)?;
    let paths = [render(&exe, 1)?, render(&exe, 2)?, render(&exe, 3)?];
    let mut reports = BTreeMap::new();
    for (name, fixture) in &fixtures {
        let computed = super::stereo_clock::schedule(fault(name).map(|(t, _)| t))?;
        ensure(
            computed.len() == fixture.schedule.len(),
            &format!(
                "{name}: capture schedule count {} vs {}",
                computed.len(),
                fixture.schedule.len()
            ),
        )?;
        let mut max_capture_error = 0f64;
        for ((t, epoch, g), (expected, index, old)) in computed.iter().zip(&fixture.schedule) {
            ensure(
                (t, epoch) == (expected, index),
                &format!("{name} publication tick and epoch {t}/{epoch:?} vs {expected}/{index:?}"),
            )?;
            for (v, w) in g.iter().zip(old) {
                max_capture_error = max_capture_error.max((v - w).abs());
            }
        }
        ensure(
            max_capture_error <= 2e-11,
            &format!("{name}: held capture error {max_capture_error}"),
        )?;
        let mut left = Lane::new(&kernel);
        let mut right = Lane::new(&kernel);
        let mut rows = Vec::with_capacity(N);
        let mut other = Vec::with_capacity(N);
        let mut gains = Vec::with_capacity(N);
        for t in 0..N {
            let (sample, coeff) = left.tick(t, &fixture.schedule, &paths, name, 1.)?;
            let (peer, _) =
                right.tick(t, &fixtures["reference"].schedule, &paths, "reference", 1.)?;
            rows.push(sample);
            gains.push(coeff);
            other.push(peer);
        }
        let mut data =
            std::io::BufWriter::new(std::fs::File::create(out.join(format!("{name}.f64le")))?);
        use std::io::Write;
        for row in &rows {
            data.write_all(row.map(f64::to_le_bytes).as_flattened())?;
        }
        data.flush()?;
        if name == "reference" {
            let mut coeff = std::io::BufWriter::new(std::fs::File::create(
                out.join("reference-coefficients.f64le"),
            )?);
            for r in &gains {
                coeff.write_all(r.map(f64::to_le_bytes).as_flattened())?;
            }
            coeff.flush()?;
        }
        for (j, digest) in fixture.columns.iter().enumerate() {
            let mut h = Sha256::new();
            for r in &rows {
                h.update(r[j].to_le_bytes());
            }
            ensure(
                format!("{:x}", h.finalize()) == *digest,
                &format!("{name} joined sample column {j}"),
            )?;
        }
        ensure(
            hash(&rows) == fixture.samples && hash(&gains) == fixture.gains,
            "joined sample and resolved coefficients",
        )?;
        ensure(
            hash(&other)
                == fixture
                    .right
                    .as_ref()
                    .unwrap_or(&fixtures["reference"].samples)
                    .as_str(),
            "unaffected stereo peer",
        )?;
        let mut signed_runs = Vec::new();
        for gain in [-0.75, 0.625] {
            let mut lane = Lane::new(&kernel);
            let mut mixed = Vec::with_capacity(N * 4);
            let mut worst = 0f64;
            for (t, reference) in rows.iter().enumerate() {
                let (s, _) = lane.tick(t, &computed, &paths, name, gain)?;
                let direct = super::stereo_clock::input(t);
                let actual = ((1. - s[9]) * direct + s[10]) as f32;
                let when = fault(name).map(|(tick, _)| tick);
                let (energy, product) =
                    if let Some(start) = when.filter(|start| (*start..*start + 240).contains(&t)) {
                        // The intact peer independently supplies the preceding normalized wet.
                        let w = (start + 239 - t) as f64 / 240.;
                        let prior = if other[start - 1][0] == 0. {
                            0.
                        } else {
                            envelope(start - 1)
                        };
                        (prior * w, (prior * (gain * other[start - 1][8])) * w)
                    } else if reference[0] == 0. {
                        (0., 0.)
                    } else {
                        let e = envelope(t);
                        (e, e * (gain * reference[8]))
                    };
                let expected = ((1. - energy) * direct + product) as f32;
                worst = worst.max((s[10] - product).abs());
                ensure(
                    s[9] == energy
                        && (s[10] - product).abs() <= 2e-11
                        && actual.to_bits() == expected.to_bits(),
                    &format!("{name} signed product or converted A4 at tick {t}"),
                )?;
                mixed.extend(actual.to_le_bytes());
            }
            let file = format!(
                "{name}-{}.f32le",
                if gain < 0. { "negative" } else { "positive" }
            );
            std::fs::write(out.join(&file), &mixed)?;
            signed_runs.push(json!({"gain":gain,"max_product_error":worst,"a4_file":file,"mix_sha256":format!("{:x}",Sha256::digest(&mixed))}));
        }
        reports.insert(name,json!({"samples":fixture.samples,"gains":fixture.gains,"other":hash(&other),"fault":fault(name),"ticks":N,"maximum_capture_error":max_capture_error,"signed_runs":signed_runs}));
    }
    Ok(
        json!({"cases":reports,"bitwise_comparison_input":"frozen published features","signed_delivery_input":"independently computed joined DI clock"}),
    )
}
