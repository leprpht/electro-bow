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
    render_at(exe, epoch, N, RATE)
}
fn render_at(exe: &Path, epoch: usize, count: usize, rate: f64) -> Result<Carrier> {
    let mut pitch = Vec::with_capacity(count);
    let mut depths = Vec::with_capacity(count);
    let mut input = Vec::with_capacity(count * 24);
    for i in 0..count {
        let f = if epoch == 3 {
            196.
        } else {
            196. * (35. * (TAU * 5. * i as f64 / rate).sin() / 1200.).exp2()
        };
        pitch.push(f);
        depths.push(0.5 + 0.45 * (TAU * 7. * i as f64 / rate).sin());
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
    let rows = carrier::scheduled(exe, rate as u32, count, &input)?;
    let velocity = rows.iter().map(|r| r[21]).collect::<Vec<_>>();
    let delivery = scheduled_taps(exe, rate, &velocity, |i| depths[i])?;
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
#[derive(Clone, Copy)]
enum Operand {
    Native,
    StoredPhase(f64),
    M5,
    Cache,
}
#[derive(Clone, Copy)]
struct Challenge {
    tick: usize,
    probe: i32,
    site: i32,
    operand: Operand,
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
    fault_trial: bool,
    rate: f64,
    registered_launch: Option<usize>,
    envelope: fn(usize) -> f64,
    challenge: Option<Challenge>,
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
            fault_trial: false,
            rate: RATE,
            registered_launch: None,
            envelope,
            challenge: None,
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
                self.current = epoch.map(|_| self.kernel.epoch(self.rate));
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
            let chosen = self
                .challenge
                .map(|c| (c.tick, c.probe))
                .or_else(|| fault(name));
            let probe = chosen
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
            let mut energy = if self.fault_trial {
                if t <= 479 || t == 6479 { 0. } else { 0.63 }
            } else {
                (self.envelope)(t)
            };
            if let Some(launch) = self.registered_launch {
                energy = f64::from(t > launch);
            }
            let mut m5 = path.rows[i][11];
            if let Some(challenge) = self.challenge {
                match challenge.operand {
                    Operand::StoredPhase(value) if t == challenge.tick => {
                        state.inject_stored_phase(value)
                    }
                    Operand::M5 if (challenge.tick + 1 - 180..=challenge.tick).contains(&t) => {
                        m5 = 1e153
                    }
                    Operand::Cache if t == challenge.tick => {
                        energy = f64::MAX;
                        live_q[3] = 4.;
                        row[6] = 4.;
                    }
                    _ => {}
                }
            }
            let mut output = [[0.; 20]];
            let result = if probe == 20
                || (probe == 21 && name == "post-o2")
                || !live_q.iter().all(|v| v.is_finite())
            {
                Err(format!("epoch fault site={probe} offset=0").into())
            } else {
                let mut frame = [0.; 20];
                frame[..2].copy_from_slice(&[path.pitch[i], m5]);
                frame[2..6].copy_from_slice(&live_q);
                for ((a, goal), v) in self
                    .coefficients
                    .iter_mut()
                    .zip(self.target)
                    .zip(&mut frame[6..18])
                {
                    *a += -(-1. / (0.015 * self.rate)).exp_m1() * (goal - *a);
                    *v = *a;
                }
                frame[18] = f64::from(
                    self.fault_trial
                        || self.registered_launch.is_some()
                        || !(6200..7100).contains(&t),
                );
                frame[19] = 1.;
                state.run(&[frame], &mut output, probe)
            };
            let result = result.and_then(|()| {
                let product = energy * (signed * output[0][7]);
                ensure(product.is_finite(), "epoch fault site=22 offset=0")?;
                let direct = super::stereo_clock::input(t);
                let mix = (1. - energy) * direct + product;
                ensure(
                    (0. ..=1.).contains(&energy) && (mix as f32).is_finite(),
                    "epoch fault site=10 offset=0",
                )
            });
            match result {
                Ok(()) => {
                    let v = output[0];
                    row[7] = v[8];
                    row[8] = v[7];
                    row[9] = energy;
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
                    let site = self.challenge.map_or(probe, |c| c.site);
                    ensure(
                        chosen == Some((t, probe))
                            && error.to_string() == format!("epoch fault site={site} offset=0"),
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

/// Execute corruption in the actual native operation, preserve the detection-tick commits,
/// and grade both signed outputs against an unaffected, simultaneously advanced lane.
pub fn faults(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let kernel = Kernel::build(out)?;
    let paths = [render(&exe, 2)?, render(&exe, 2)?];
    let schedule = super::clock::fault_schedule(None)?;
    let signed = -1.3;
    let mut original = Lane::new(&kernel);
    original.fault_trial = true;
    let mut healthy = Vec::new();
    for t in 0..8500 {
        healthy.push(original.tick(t, &schedule, &paths, "reference", signed)?.0);
    }
    let closure = healthy
        .iter()
        .position(|r| r[16] != 0.)
        .ok_or("native block closure")?;
    let mut rows = BTreeMap::new();
    for (label, tick, probe, site, operand) in [
        ("phase", 3300, 1, 1, Operand::Native),
        ("increment", 3100, 11, 1, Operand::Native),
        ("bank", 2639, 3, 3, Operand::Native),
        ("bank-overflow", 3300, 13, 3, Operand::Native),
        ("candidate", 479, 2, 2, Operand::Native),
        ("trial", closure + 1, 4, 8, Operand::Native),
        ("trial-nan", closure + 1, 14, 4, Operand::Native),
        ("normalization", closure, 6, 6, Operand::Native),
        ("gram", closure, 7, 7, Operand::Native),
        ("residual", closure, 8, 8, Operand::Native),
        ("wet", 3300, 9, 9, Operand::Native),
        ("conversion", 3300, 10, 10, Operand::Native),
        ("moment", 3300, 5, 5, Operand::Native),
        ("cache", 3300, 0, 22, Operand::Cache),
        ("M5 accumulation", 658, 0, 5, Operand::M5),
        ("M5 closure", closure, 0, 5, Operand::M5),
        ("stored NaN", 3300, 0, 1, Operand::StoredPhase(f64::NAN)),
        ("stored maximum", 3300, 0, 5, Operand::StoredPhase(f64::MAX)),
    ] {
        let mut left = Lane::new(&kernel);
        left.fault_trial = true;
        left.challenge = Some(Challenge {
            tick,
            probe,
            site,
            operand,
        });
        let mut right = Lane::new(&kernel);
        right.fault_trial = true;
        let recovery = super::clock::fault_schedule(Some(tick))?;
        let prior_e = if tick - 1 <= 479 { 0. } else { 0.63 };
        let prior_p = prior_e * (signed * healthy[tick - 1][8]);
        let mut commits = 0;
        for (t, expected) in healthy.iter().enumerate() {
            let (r, _) = left
                .tick(t, &recovery, &paths, "reference", signed)
                .map_err(|e| format!("{label}: {e}"))?;
            let (peer, _) = right.tick(t, &schedule, &paths, "reference", signed)?;
            ensure(
                peer.map(f64::to_bits) == expected.map(f64::to_bits),
                "fault propagated to other lane",
            )?;
            if t <= tick {
                commits += r[2] as usize;
            }
            if t == tick {
                ensure(
                    r[1] == (tick - 479) as f64 && r[2] == 1. && r[3] == 1. && r[4] == 1.,
                    "detection physical/electrical commit",
                )?;
            }
            if (tick..tick + 240).contains(&t) {
                let factor = (tick + 239 - t) as f64 / 240.;
                ensure(
                    r[9] == prior_e * factor
                        && r[10] == prior_p * factor
                        && r[13] == (t - tick) as f64,
                    "independent signed cache retirement",
                )?;
            }
            if (tick + 1..6479).contains(&t) {
                ensure(
                    r[2] == 0. && r[3] == 0. && r[4] == 1.,
                    "duplicate fault or reused retired epoch",
                )?;
            }
            if t == 6479 {
                ensure(
                    r[..5] == [2., 0., 1., 1., 1.],
                    "fresh epoch after lane retirement",
                )?;
            }
        }
        ensure(
            commits == tick - 479 + 1 && left.losses == 1,
            "single numerical-fault owner",
        )?;
        rows.insert(label,json!({"site":site,"tick":tick,"zero":tick+239,"commits":commits,"previous_energy":prior_e,"previous_product":prior_p,"fresh":6479}));
    }
    Ok(
        json!({"faults":rows,"phase_classification_held":true,"signed_gain":signed,"closure":closure}),
    )
}

pub fn uneven(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let kernel = Kernel::build(out)?;
    let rate = 47001.;
    let count = 14500;
    let neutral = render_at(&exe, 3, 3 * 47001, rate)?;
    let taps: Vec<_> = neutral
        .rows
        .iter()
        .map(|r| [r[25], r[26], r[27], r[26]])
        .collect();
    for j in 0..3 {
        let grade = super::stationary::grade(&neutral.rows, &taps, 47001, 196., j)?;
        ensure(
            grade["passes"]
                .as_object()
                .ok_or("prerequisite")?
                .values()
                .all(|v| v != false),
            "uneven neutral prerequisite",
        )?;
    }
    let paths = [render_at(&exe, 2, count, rate)?];
    let original: Value = serde_json::from_str(include_str!("../../contracts/uneven.json"))?;
    let mut results = BTreeMap::new();
    for kind in ["complete", "missing-A2", "missing-I1"] {
        let rows = super::clock::uneven(kind)?;
        let schedule = rows
            .iter()
            .map(|r| {
                Ok((
                    r["tick"].as_u64().ok_or("tick")? as usize,
                    r["epoch"].as_u64().map(|v| v as usize),
                    serde_json::from_value::<[f64; 12]>(r["target"].clone())?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut actual = Lane::new(&kernel);
        actual.rate = rate;
        actual.envelope = |t| ((t as f64 - 2614.) / 376.).clamp(0., 1.);
        let mut frozen = Lane::new(&kernel);
        frozen.rate = rate;
        frozen.envelope = actual.envelope;
        let recorded = serde_json::from_value(original[kind]["schedule"].clone())?;
        let mut samples = Vec::new();
        let mut gains = Vec::new();
        let mut max_error = 0f64;
        let mut changed = false;
        for t in 0..count {
            let (r, g) = actual.tick(t, &schedule, &paths, "reference", 1.)?;
            let (expected, coeff) = frozen.tick(t, &recorded, &paths, "reference", 1.)?;
            for (a, b) in r.iter().zip(expected).chain(g.iter().zip(coeff)) {
                max_error = max_error.max((a - b).abs());
            }
            changed |= r[8] != r[6];
            samples.push(expected);
            gains.push(coeff);
        }
        ensure(
            max_error <= 2e-11 && changed == (kind == "complete"),
            &format!("{kind} uneven delivered feature: {max_error}"),
        )?;
        ensure(
            original[kind]["samples"] == hash(&samples) && original[kind]["gains"] == hash(&gains),
            &format!("{kind} historical uneven trace"),
        )?;
        results.insert(kind,json!({"changed":changed,"capture":rows,"samples":hash(&samples),"coefficients":hash(&gains),"maximum_error":max_error}));
    }
    Ok(json!({"cases":results,"rate":rate}))
}

pub fn joined_motion(out: &Path, root: &Path) -> Result<Value> {
    use crate::{evidence, guide, publication};
    let previous = evidence::strict(&std::fs::read(publication::current_artifact(
        root,
        "i1-motion",
        "evidence.json",
    )?)?)?;
    let exe = replay::build(out, None)?;
    let kernel = Kernel::build(out)?;
    let mut rows = BTreeMap::new();
    for rate in [32000u32, 44100, 48000, 88200, 96000, 176400, 192000] {
        let fs = f64::from(rate);
        let n = 3 * rate as usize;
        let da = rate as usize / 8000;
        let launch = da * (0.005 * fs / da as f64 + 0.5).floor() as usize - 1;
        let (pitch, depths, controls) = super::motion::schedule("vibrato", rate);
        let physical = carrier::scheduled(&exe, rate, n, &controls)?;
        let v = physical.iter().map(|r| r[21]).collect::<Vec<_>>();
        let delivery = scheduled_taps(&exe, fs, &v, |i| depths[i])?;
        let paths = [Carrier {
            rows: physical,
            delivery,
            pitch,
            depths,
        }];
        let schedule = super::clock::registered(fs, launch)?;
        let mut lane = Lane::new(&kernel);
        lane.rate = fs;
        lane.registered_launch = Some(launch);
        let mut idle = Lane::new(&kernel);
        idle.rate = fs;
        let mut wet: [Vec<f64>; 2] = std::array::from_fn(|_| Vec::with_capacity(n));
        let mut delta = Vec::with_capacity(n);
        let mut commits = 0;
        let mut veto = 0;
        let scored = rate as usize / 2..5 * rate as usize / 2;
        let idle_schedule = Vec::new();
        for t in 0..n + launch {
            let (s, _) = lane.tick(t, &schedule, &paths, "reference", 1.)?;
            let (peer, _) = idle.tick(t, &idle_schedule, &paths, "reference", 1.)?;
            ensure(
                peer[2] == 0. && peer[3] == 0. && s[4] == 0.,
                "registered lane isolation",
            )?;
            if let Some(i) = t.checked_sub(launch) {
                ensure(
                    s[1] == i as f64 && s[2] == 1. && s[3] == 1.,
                    "registered physical commits",
                )?;
                commits += 1;
                wet[0].push(s[19]);
                wet[1].push(s[8]);
                delta.push([s[19] - paths[0].rows[i][25], s[8] - paths[0].delivery[i][3]]);
                if scored.contains(&i) {
                    veto += s[14] as usize;
                }
            }
        }
        let mut c = carrier::cases()?
            .remove("vibrato_48000_196_R1.8_V0.1_3s")
            .ok_or("motion diagnostic")?;
        c.rate = rate;
        let reference=previous["results"]["cases"][format!("steady-{rate}")]["historical_stationary_calibration"].as_f64().ok_or("fresh fixed-bin stationary calibration")?;
        let mut result = json!({"physical_commits":commits,"realized_veto_samples":veto});
        for (j, key) in ["off", "delivery"].iter().enumerate() {
            let mut row = super::score(&delta, rate, j, reference)?;
            ensure(row["passed"] == true, "registered moving correction")?;
            let window = &delta[scored.clone()];
            let power = window.iter().map(|r| r[j] * r[j]).sum::<f64>() / window.len() as f64;
            let mut gap = 0;
            let mut maxgap = 0;
            for r in window {
                if r[j] == 0. {
                    gap += 1;
                    maxgap = maxgap.max(gap);
                } else {
                    gap = 0;
                }
            }
            row["rms"] = power.sqrt().into();
            row["peak"] = wet[j].iter().map(|v| v.abs()).fold(0., f64::max).into();
            row["neutral_samples"] = maxgap.into();
            row["pitch"] = guide::phase::auxiliary(&c, &wet[j])?;
            result[*key] = row;
        }
        rows.insert(rate, result);
    }
    Ok(json!({"rates":rows}))
}

pub fn stream_compare(reference: &Path, root: &Path) -> Result<Value> {
    use crate::{electrical::acoustic, evidence, publication};
    let old = reference.join("checkout/build/i1");
    let stream = evidence::strict(&std::fs::read(old.join("stream-checks.json"))?)?;
    let joined = evidence::strict(&std::fs::read(old.join("joined.json"))?)?;
    let read = |s| -> Result<Value> {
        Ok(
            evidence::strict(&std::fs::read(publication::current_artifact(
                root,
                s,
                "evidence.json",
            )?)?)?["results"]
                .clone(),
        )
    };
    let mut stats = (0, 0., 0.);
    let motion = read("i1-joined-motion")?;
    acoustic::compare(
        &stream["joined_motion"],
        &motion["rates"],
        "registered motion",
        &mut stats,
    )?;
    let faults = read("i1-faults")?;
    for (k, row) in stream["faults"].as_object().ok_or("faults")? {
        let result = &faults["faults"][k];
        let site = match result["site"].as_i64() {
            Some(1) => "PHASE",
            Some(2) => "COEFFICIENT",
            Some(3) => "BANK",
            Some(4) => "TRIAL",
            Some(5) => "MOMENT",
            Some(6) => "NORMALIZE",
            Some(7) => "GRAM",
            Some(8) => "RESIDUAL",
            Some(9) => "WET",
            Some(10) => "CONVERSION",
            Some(22) => "CACHE",
            _ => return Err("unrecognized fault".into()),
        };
        ensure(
            row["site"] == site
                && ["tick", "zero", "commits"]
                    .iter()
                    .all(|key| row[key] == result[key]),
            &format!("{k} numerical owner"),
        )?;
    }
    let uneven = read("i1-uneven")?;
    for (name, row) in joined["uneven_delivery"]
        .as_object()
        .ok_or("uneven delivery")?
    {
        let r = &uneven["cases"][name];
        acoustic::compare(&row["capture"], &r["capture"], "uneven capture", &mut stats)?;
        ensure(
            r["changed"] == row["changed"] && r["samples"] == row["trace"],
            "uneven trajectory",
        )?;
    }
    Ok(
        json!({"rates":7,"numerical_faults":16,"uneven_cases":3,"fields":stats.0,"largest_scaled_difference":stats.1}),
    )
}
