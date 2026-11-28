//! Inject frozen, source-owned coefficient schedules into the offline kernel and grade its actual returns.
use super::{Bank, epoch::Kernel};
use crate::{
    Result,
    electrical::{carrier, replay_taps},
    ensure, numerics, replay,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
pub(super) struct Contract {
    pub(super) cases: Vec<Case>,
    pub(super) schedules: BTreeMap<String, Schedule>,
}
pub(super) type Schedule = (Option<[f64; 12]>, Vec<(usize, [f64; 12])>);
#[derive(Deserialize)]
pub(super) struct Case {
    pub(super) case: String,
    pub(super) rate: u32,
    pub(super) hz: f64,
    pub(super) physical: String,
    pub(super) schedule: String,
    smoother: f64,
    wet: String,
    correction: String,
    scale: String,
    predicates: Vec<Value>,
}
fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
}
fn ratio(num: f64, den: f64) -> Option<f64> {
    (den > 1e-12).then(|| 20. * (num / den).max(1e-12).log10())
}

pub(crate) struct Spectrum {
    amplitude: Vec<f64>,
    pub(crate) peak: Option<f64>,
}
impl Spectrum {
    pub(crate) fn new(x: &[f64], fs: u32) -> Self {
        let n = x.len();
        let mean = x.iter().sum::<f64>() / n as f64;
        let ac = (x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
        if ac <= 1e-12 {
            return Self {
                amplitude: vec![],
                peak: None,
            };
        }
        let mut window: Vec<_> = x
            .iter()
            .zip(numerics::hann(n))
            .map(|(v, w)| (v - mean) * w)
            .collect();
        window.resize(8 * n, 0.);
        let amplitude: Vec<_> = numerics::spectrum(&window)
            .into_iter()
            .map(|z| z.norm())
            .collect();
        let mut k = 1;
        for i in 2..amplitude.len() {
            if amplitude[i] > amplitude[k] {
                k = i;
            }
        }
        let mut offset = 0.;
        if k + 1 < amplitude.len()
            && amplitude[k] >= amplitude[k - 1]
            && amplitude[k - 1..=k + 1].iter().all(|a| *a > 0.)
        {
            let a = amplitude[k - 1].ln();
            let b = amplitude[k].ln();
            let c = amplitude[k + 1].ln();
            let curvature = a - 2. * b + c;
            if curvature < 0. {
                offset = 0.5 * (a - c) / curvature;
            }
        }
        Self {
            amplitude,
            peak: Some((k as f64 + offset) * f64::from(fs) / (8 * n) as f64),
        }
    }
    fn metrics(&self, fs: u32, f0: f64) -> Value {
        self.band_metrics(fs, f0, f64::from(fs) / 2.)
    }
    pub(crate) fn band_metrics(&self, fs: u32, f0: f64, ceiling: f64) -> Value {
        let (mut total, mut harmonic, mut high, mut moment) = (0., 0., 0., 0.);
        for (k, amp) in self.amplitude.iter().enumerate().skip(1) {
            let f = k as f64 / 8.;
            if f > ceiling {
                break;
            }
            let order = (f / f0).round_ties_even();
            let p = amp * amp;
            total += p;
            moment += f * p;
            if f >= 6000. {
                high += p;
            }
            if order >= 1.
                && order * f0 < f64::from(fs) / 2.
                && (f - order * f0).abs() <= 2f64.max(0.001 * order * f0)
            {
                harmonic += p;
            }
        }
        let eta = harmonic / total;
        json!({"eta":eta,"residual":1.-eta,"centroid_hz":moment/total,"high_fraction":high/total,"high_flag":high/total>0.01})
    }
}
pub(crate) fn near(hz: Option<f64>, f: f64) -> bool {
    hz.is_some_and(|v| v > 0. && ((v - f).abs() <= 2. || (1200. * (v / f).log2()).abs() <= 15.))
}
pub(crate) fn grade(
    raw: &[[f64; 30]],
    wet: &[[f64; 4]],
    fs: u32,
    f: f64,
    j: usize,
) -> Result<Value> {
    ensure(
        raw.len() == wet.len() && raw.len() >= fs as usize,
        "aligned V17 samples",
    )?;
    let start = raw.len() - fs as usize;
    let tail = &raw[start..];
    let y = &wet[start..];
    let column = |j: usize| tail.iter().map(|r| r[j]).collect::<Vec<_>>();
    let m5 = column(11);
    let q = column(25 + j);
    let neutral = column(25);
    let actual: Vec<_> = y.iter().map(|r| r[j]).collect();
    let off: Vec<_> = y.iter().map(|r| r[0]).collect();
    let diff = |a: &[f64], b: &[f64]| {
        (a.iter().zip(b).map(|(v, w)| (v - w).powi(2)).sum::<f64>() / a.len() as f64).sqrt()
    };
    let anchor = Spectrum::new(&m5, fs).peak;
    let spectrum = Spectrum::new(&actual, fs);
    let voiced = near(anchor, f) && spectrum.peak.is_some();
    let level = ratio(rms(&actual), rms(&m5));
    let color = ratio(rms(&actual), rms(&q));
    let metrics = if voiced {
        Some(spectrum.metrics(fs, anchor.unwrap()))
    } else {
        None
    };
    let eta_pass = metrics
        .as_ref()
        .and_then(|v| v["eta"].as_f64())
        .is_some_and(|eta| eta >= 0.85);
    let residual = if j == 1 {
        ratio(diff(&actual, &off), rms(&off))
    } else {
        None
    };
    let predicates = json!({"bridge_anchor":near(anchor,f),"level":level.is_some_and(|v|v.abs()<=3.),"coloration":color.is_some_and(|v|v.abs()<=3.),"fundamental":voiced && near(spectrum.peak,f),"high_note_eta":(fs==32000 && f==1318.51 && j==0).then_some(eta_pass),"factory_eta":(j==1).then_some(eta_pass),"factory_residual":(j==1).then(||residual.is_some_and(|v|v<= -28.))});
    let harmonics = if voiced {
        let ph = numerics::partials(&actual, f64::from(fs), anchor.unwrap(), 12);
        let valid = |i: usize| {
            ph[i].norm() > 1e-12
                && ph[0].norm() > 1e-12
                && actual.len() as f64 * anchor.unwrap() >= 2. * f64::from(fs)
        };
        Some(
            json!({"peak_FS":ph.iter().map(|z|z.norm()).collect::<Vec<_>>(),"dBr":(0..ph.len()).map(|i|if valid(i){ratio(ph[i].norm(),ph[0].norm())}else{None}).collect::<Vec<_>>(),"phase_deg":ph.iter().enumerate().map(|(i,z)|valid(i).then(||(*z*rustfft::num_complex::Complex64::from_polar(1.,-((i+1) as f64)*ph[0].arg())).arg().to_degrees())).collect::<Vec<_>>()}),
        )
    } else {
        None
    };
    Ok(
        json!({"passes":predicates,"failure_reason":(!near(anchor,f)).then_some("ineligible M5 bridge anchor"),"peak_FS":wet.iter().map(|r|r[j].abs()).fold(0.,f64::max),"rms_FS":rms(&actual),"same_source_difference_rms_FS":diff(&q,&neutral),"harmonics":harmonics,"spectral":metrics,"measured_f0":anchor,"strongest_hz":spectrum.peak,"final_over_M5":level,"corrected_over_neutral":color,"factory_difference_over_off":residual,"factory_wet_difference_rms_FS":(j==1).then(||diff(&actual,&off)),"spatial_over_N":ratio(rms(&column(21)),rms(&column(3))),"load_over_blocked":ratio(rms(&column(24)),rms(&column(22).iter().map(|v|0.85*v/std::f64::consts::SQRT_2).collect::<Vec<_>>())),"detail_over_neutral":ratio(rms(&q),rms(&column(24))),"common_over_M5":ratio(std::f64::consts::SQRT_2*rms(&q),rms(&m5)),"local_q_difference_over_off":ratio(diff(&q,&neutral),rms(&neutral))}),
    )
}

fn render_case(
    c: &Case,
    schedule: &Schedule,
    raw: &[[f64; 30]],
    exe: &Path,
    kernel: &Kernel,
) -> Result<(Vec<[f64; 4]>, Value)> {
    let fs = f64::from(c.rate);
    let velocity: Vec<_> = raw.iter().map(|r| r[21]).collect();
    let fourth = replay_taps(exe, fs, &velocity, 0.5)?;
    let mut epoch = kernel.epoch(fs);
    let mut corrected = super::candidate::Differential::new(fs);
    let mut revised_wet = Vec::with_capacity(raw.len());
    let mut bank = Bank::new(fs)?;
    let mut request = Sha256::new();
    if let Some(constant) = schedule.0 {
        for v in constant {
            request.update(v.to_le_bytes());
        }
    }
    let mut last = [0.; 12];
    let mut last_scale = 0.;
    let mut timeline: Vec<Value> = Vec::new();
    let mut start = 0;
    let mut wet_hash = Sha256::new();
    let mut delta_hash = Sha256::new();
    let mut scale_hash = Sha256::new();
    let mut wet = Vec::with_capacity(raw.len());
    let mut target = [0.; 12];
    let mut z = [0.; 12];
    let mut updates = schedule.1.iter().peekable();
    let mut max_error = 0.0_f64;
    for (block, rows) in raw.chunks(8192).enumerate() {
        let mut frames = Vec::with_capacity(rows.len());
        let mut original = Vec::with_capacity(rows.len());
        for (i, r) in rows.iter().enumerate() {
            let n = block * 8192 + i;
            let mut frame = [0.; 20];
            frame[0] = c.hz;
            frame[1] = r[11];
            frame[2..5].copy_from_slice(&r[25..28]);
            frame[5] = fourth[n][3];
            ensure(
                frame[5].to_bits() == r[26].to_bits(),
                "fourth returned factory tap",
            )?;
            if let Some(constant) = schedule.0 {
                frame[6..18].copy_from_slice(&constant);
            } else {
                while let Some((tick, g)) = updates.peek() {
                    if *tick > n {
                        break;
                    }
                    target = *g;
                    updates.next();
                }
                for k in 0..12 {
                    // The pinned SciPy direct-form filter fuses the feed-forward product and delay.
                    let gain = c.smoother.mul_add(target[k], z[k]);
                    frame[6 + k] = gain;
                    z[k] = (1. - c.smoother) * gain;
                }
            }
            last.copy_from_slice(&frame[6..18]);
            if schedule.0.is_none() {
                for v in last {
                    request.update(v.to_le_bytes());
                }
            }
            let mut bank_input = [0.; 17];
            bank_input[0] = c.hz;
            bank_input[1..].copy_from_slice(&frame[2..18]);
            original.push(bank.apply(&bank_input)?);
            frame[18..].fill(1.);
            frames.push(frame);
        }
        let mut output = vec![[0.; 20]; frames.len()];
        epoch.run(&frames, &mut output, 0)?;
        for (i, (v, independent)) in output.iter().zip(original).enumerate() {
            let observed = corrected.check(&frames[i], v)?;
            revised_wet.push(observed[4..8].try_into()?);
            let at = block * 8192 + i;
            if v[10] != 0. {
                let end = at + 1;
                timeline.push(json!({"start":start,"end":end,"conditioned":v[12]!=0.,"bridge_ok":v[13]!=0.,"available":v[14]!=0.,"realized_ok":v[11]!=0.,"observable":true,"trusted":true,"selection_tick":null,"veto":null,"scale":null}));
                start = end;
            }
            if let Some(last) = timeline.last_mut()
                && at == start + 16
            {
                last["selection_tick"] = at.into();
                last["veto"] = (v[15] != 0.).into();
                last["scale"] = v[16].into();
            }
            last_scale = v[8];
            for j in 0..4 {
                let error = (v[j] - independent[j]).abs();
                ensure(error <= 2e-11, "fourth-bank correction")?;
                max_error = max_error.max(error);
                if v[8] == 0. {
                    ensure(
                        v[4 + j].to_bits() == frames[i][2 + j].to_bits(),
                        "neutral signed wet bits",
                    )?;
                }
                wet_hash.update(v[4 + j].to_le_bytes());
                delta_hash.update(v[j].to_le_bytes());
            }
            scale_hash.update(v[8].to_le_bytes());
            wet.push(v[4..8].try_into()?);
        }
    }
    let hashes = [wet_hash, delta_hash, scale_hash].map(|h| format!("{:x}", h.finalize()));
    ensure(
        hashes == [&c.wet, &c.correction, &c.scale].map(String::as_str),
        &format!("{} historical wet/correction/scale word identity", c.case),
    )?;
    let mut corrected_depths = Vec::new();
    for j in 0..3 {
        let row = grade(raw, &revised_wet, c.rate, c.hz, j)?;
        ensure(
            row["passes"] == c.predicates[j],
            "Stage 08.3 stationary V17",
        )?;
        corrected_depths.push(row["passes"].clone());
    }
    let effective = last.map(|v| last_scale * v);
    let peaks = super::sweeps::analytical(fs, c.hz, &effective)?;
    Ok((
        wet,
        json!({"stage_08_3":{"passes":corrected_depths,"changed_wet_words":corrected.changed_wet,"changed_f32":corrected.changed_audio},"hashes":hashes,"bank_max_error":max_error,"timeline":timeline,"requested":format!("{:x}",request.finalize()),"sweep":peaks,"requested_dB":last.map(|v|20.*(1.+v).log10()),"accepted_dB":effective.map(|v|20.*(1.+v).log10()),"failures":[],"eligible":true}),
    ))
}
pub fn run(out: &Path) -> Result<Value> {
    let contract: Contract = serde_json::from_str(include_str!("../../contracts/stationary.json"))?;
    let cases = carrier::cases()?;
    let identities = carrier::identities();
    let exe = replay::build(out, None)?;
    let kernel = Kernel::build(out)?;
    let mut revised = BTreeMap::new();
    let mut historical = BTreeMap::new();
    for c in contract.cases {
        let raw = carrier::render(&exe, &cases[&c.physical])?;
        let mut hash = Sha256::new();
        for r in &raw {
            hash.update(r.map(f64::to_le_bytes).as_flattened());
        }
        ensure(
            format!("{:x}", hash.finalize()) == identities[c.physical.as_str()][2],
            "stationary carrier source",
        )?;
        let schedule = &contract.schedules[&c.schedule];
        let (wet, mut evidence) = render_case(&c, schedule, &raw, &exe, &kernel)?;
        let uncolored: Vec<_> = raw.iter().map(|r| [r[25], r[26], r[27], r[26]]).collect();
        let mut neutral = Vec::new();
        let mut depths = Vec::new();
        for j in 0..3 {
            neutral.push(grade(&raw, &uncolored, c.rate, c.hz, j)?);
            let row = grade(&raw, &wet, c.rate, c.hz, j)?;
            ensure(
                row["passes"] == c.predicates[j],
                "stationary depth decision",
            )?;
            depths.push(row);
        }
        evidence["neutral"] = neutral.into();
        evidence["depths"] = depths.into();
        if matches!(c.schedule.as_str(), "upper" | "lower" | "inversion") {
            let mut bank = Bank::new(f64::from(c.rate))?;
            let mut unbounded = Vec::with_capacity(raw.len());
            for r in &raw {
                let mut a = [0.; 17];
                a[0] = c.hz;
                a[1..4].copy_from_slice(&r[25..28]);
                a[4] = r[26];
                a[5..].copy_from_slice(&schedule.0.ok_or("historical gain vector")?);
                let d = bank.apply(&a)?;
                unbounded.push(std::array::from_fn(|i| a[i + 1] + d[i]));
            }
            let row = grade(&raw, &unbounded, c.rate, c.hz, 0)?;
            let failures: Vec<_> = row["passes"]
                .as_object()
                .unwrap()
                .iter()
                .filter_map(|(k, v)| (v == false).then_some(k.as_str()))
                .collect();
            let expected = if c.schedule == "inversion" {
                vec!["fundamental"]
            } else {
                vec!["coloration", "level"]
            };
            ensure(failures == expected, "historical unsafe disposition")?;
            historical.insert(c.case.clone(), row);
        }
        ensure(
            revised.insert(c.case, evidence).is_none(),
            "duplicate stationary case",
        )?;
    }
    ensure(
        revised.len() == 36 && historical.len() == 9,
        "stationary and historical inventories",
    )?;
    let sweep = super::sweeps::measured(&exe, &revised)?;
    let expression = super::sweeps::expression(&revised)?;
    Ok(json!({"stationary":revised,"historical":historical,"sweep":sweep,"expression":expression}))
}
