//! Acceptance side. Derive the normalized stream from the scheduled carrier,
//! independent four-bank implementation and rectangular full-window QR.
use super::*;
use crate::identity::{Bank, paths, stereo_clock};
use std::io::{BufWriter, Write};

fn normalized(schedule: &stereo_clock::Schedule, paths: &[Carrier]) -> Result<Vec<Option<f64>>> {
    let mut samples = vec![None; N];
    let mut start = 0;
    while start < schedule.len() {
        let (t, owner, _) = schedule[start];
        let end = (start + 1..schedule.len())
            .find(|&j| schedule[j].1 != owner)
            .unwrap_or(schedule.len());
        let stop = schedule.get(end).map_or(N, |s| s.0);
        if let Some(id) = owner {
            let path = &paths[id - 1];
            let mut bank = Bank::new(RATE)?;
            let mut coefficients = [0.; 12];
            let mut frames = Vec::new();
            let mut corrections = Vec::new();
            let mut slot = start;
            for tick in t..stop {
                if slot + 1 < end && schedule[slot + 1].0 == tick {
                    slot += 1;
                }
                let i = tick - t;
                let mut f = [0.; 20];
                f[..6].copy_from_slice(&[
                    path.pitch[i],
                    path.rows[i][11],
                    path.rows[i][25],
                    path.rows[i][26],
                    path.rows[i][27],
                    path.delivery[i][3],
                ]);
                for (c, target) in coefficients.iter_mut().zip(schedule[slot].2) {
                    *c += -(-1. / (0.015 * RATE)).exp_m1() * (target - *c);
                }
                f[6..18].copy_from_slice(&coefficients);
                f[18] = f64::from(!(6200..7100).contains(&tick));
                f[19] = 1.;
                let mut row = [0.; 17];
                row[0] = f[0];
                row[1..].copy_from_slice(&f[2..18]);
                corrections.push(bank.apply(&row)?[..4].try_into()?);
                frames.push(f);
            }
            let oracle = paths::drive(&frames, &corrections, RATE)?;
            for (dest, wet) in samples[t..stop].iter_mut().zip(oracle.wet) {
                *dest = Some(wet[3]);
            }
        }
        start = end;
    }
    Ok(samples)
}

fn character(t: usize) -> f64 {
    match t % 181 {
        0..=25 => 0.,
        26..=53 => 1.,
        i => (i as f64 - 53.) / 128.,
    }
}
fn right_direct(t: usize) -> f64 {
    -0.625 * stereo_clock::input(t) + 0.03125
}
fn gain(lane: usize, t: usize) -> f64 {
    match (lane, t) {
        (0, 0..9719) => 1.,
        (0, 9719..15599) => -1.4,
        (0, _) => 0.08,
        (_, 0..9719) => -1.4,
        (_, 9719..15599) => 0.08,
        _ => -0.625,
    }
}

fn joined(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let paths = [render(&exe, 1)?, render(&exe, 2)?, render(&exe, 3)?];
    let nominal = stereo_clock::schedule(None)?;
    let nominal_wet = normalized(&nominal, &paths)?;
    let kernel = Kernel::Corrected;
    let mut summaries = Vec::new();
    // Complete the unbroken lifecycle, then retire either lane from corrupt state.
    for (case, damaged, poison, reset) in [
        ("ordinary", None, 0., None),
        ("stored-max", Some(0), f64::MAX, None),
        ("stored-nan", Some(1), f64::NAN, None),
        ("stored-infinity", Some(1), f64::INFINITY, None),
        ("transport", Some(0), f64::MAX, Some(9860)),
    ] {
        // Fractional E during the attack of the replacement epoch.
        let detection = 9820;
        let after = stereo_clock::schedule(Some(detection))?;
        let after_wet = normalized(&after, &paths)?;
        let mut lanes = [Lane::new(&kernel), Lane::new(&kernel)];
        for l in &mut lanes {
            l.character = character;
        }
        lanes[1].direct = right_direct;
        if let Some(j) = damaged {
            lanes[j].challenge = Some(Challenge {
                tick: detection,
                probe: 0,
                site: 1,
                operand: Operand::StoredPhase(poison),
            });
        }
        let mut writer = BufWriter::new(std::fs::File::create(out.join(format!("{case}.f64le")))?);
        let mut preceding = [[0.; 2]; 2];
        let mut witness = Value::Null;
        let mut max_error = 0f64;
        for host in 0..N {
            if Some(host) == reset {
                // Independent transport field wins before either lane processes the sample.
                lanes = [Lane::new(&kernel), Lane::new(&kernel)];
                for l in &mut lanes {
                    l.character = character;
                }
                lanes[1].direct = right_direct;
                preceding = [[0.; 2]; 2];
            }
            let tick = if let Some(at) = reset.filter(|at| host >= *at) {
                host - at
            } else {
                host
            };
            for (j, lane) in lanes.iter_mut().enumerate() {
                let failing = damaged == Some(j) && reset.is_none_or(|at| host < at);
                let schedule = if failing { &after } else { &nominal };
                let independent = if failing { &after_wet } else { &nominal_wet };
                let signed_gain = gain(j, tick);
                let (r, _) = lane.tick(tick, schedule, &paths, "reference", signed_gain)?;
                let sample = lane.emitted;
                let direct = if j == 0 {
                    stereo_clock::input(tick)
                } else {
                    right_direct(tick)
                };
                let c = character(tick);
                let fading = failing && (detection..detection + 240).contains(&tick);

                let mut signed = 0.;
                let pair = if fading {
                    let index = tick - detection;
                    ensure(
                        r[13] == index as f64
                            && r[4] == 1.
                            && lane.site == if index == 0 { 1 } else { 0 },
                        "PHASE fade and single retirement",
                    )?;
                    if index == 0 {
                        ensure(
                            r[..4] == [2., (tick - 9719) as f64, 1., 1.]
                                && r[6].to_bits() == paths[1].delivery[tick - 9719][3].to_bits(),
                            "phase detection commit",
                        )?;
                    } else {
                        ensure(r[2..4] == [0., 0.], "physical replay during fade")?;
                    }
                    let s = (239 - index) as f64 / 240.;
                    [s * preceding[j][0], s * preceding[j][1]]
                } else if let Some(w) = independent[tick] {
                    let launch = if tick < 9719 {
                        479
                    } else if tick < 15599 {
                        9719
                    } else {
                        15599
                    };
                    ensure(
                        r[1] == (tick - launch) as f64 && r[2..4] == [1., 1.],
                        "fresh or unaffected lane progression",
                    )?;
                    max_error = max_error.max((sample.normalized - w).abs());
                    ensure(
                        sample.normalized.to_bits() == w.to_bits(),
                        "independent normalized wet",
                    )?;

                    signed = signed_gain * w;
                    let e = envelope(tick);
                    [e, e * signed]
                } else {
                    ensure(r[2..4] == [0., 0.], "idle lane commit")?;
                    [0.; 2]
                };
                ensure(
                    sample.direct.to_bits() == direct.to_bits()
                        && sample.character == c
                        && sample.gain == signed_gain
                        && sample.energy == pair[0],
                    "authoritative delivery inputs",
                )?;
                if !fading && independent[tick].is_some() {
                    ensure(
                        sample.signed.to_bits() == (signed_gain * sample.normalized).to_bits()
                            && sample.signed.to_bits() == signed.to_bits(),
                        "signed gain times normalized wet",
                    )?;
                    ensure(
                        sample.product.to_bits() == (pair[0] * sample.signed).to_bits(),
                        "P == E*(signed_gain*wet)",
                    )?;
                }
                let mix = (1. - c * pair[0]) * direct + c * pair[1];
                let ordered = (1. - c * sample.energy) * direct + c * sample.product;
                ensure(
                    sample.product.to_bits() == pair[1].to_bits(),
                    "independent emitted wet product",
                )?;
                ensure(
                    sample.mix.to_bits() == ordered.to_bits()
                        && sample.mix.to_bits() == mix.to_bits(),
                    if fading {
                        "cached-fade A4 mix"
                    } else {
                        "live A4 mix"
                    },
                )?;
                ensure(
                    sample.output.to_bits() == (mix as f32).to_bits()
                        && sample.output.to_bits() == (sample.mix as f32).to_bits(),
                    "emitted binary32",
                )?;
                if !fading {
                    preceding[j] = pair;
                }
                let cache = if fading && tick == detection + 239 {
                    [0.; 2]
                } else {
                    preceding[j]
                };
                ensure(
                    [r[11], r[12]].map(f64::to_bits) == cache.map(f64::to_bits),
                    "independent published cache",
                )?;
                if fading && tick == detection + 239 {
                    ensure(sample.product == 0., "terminal zero product")?;
                    preceding[j] = [0.; 2];
                }
                let words = [
                    host as f64,
                    j as f64,
                    sample.normalized,
                    signed_gain,
                    sample.signed,
                    sample.energy,
                    c,
                    direct,
                    sample.product,
                    sample.mix,
                    f64::from(sample.output),
                    r[11],
                    r[12],
                    f64::from(lane.site),
                    r[13],
                    r[2],
                    r[3],
                    r[4],
                    r[0],
                    r[1],
                ];
                writer.write_all(words.map(f64::to_le_bytes).as_flattened())?;
                if host == if failing { detection } else { 480 }
                    && (damaged == Some(j) || damaged.is_none() && j == 0)
                {
                    witness = json!(words);
                }
            }
        }
        writer.flush()?;
        summaries.push(json!({"case":case,"damaged_lane":damaged,"ticks":N,"max_normalized_error":max_error,"witness":witness}));
    }
    Ok(
        json!({"columns":["host","lane","normalized","signed_gain","signed_wet","emitted_E","character","direct","emitted_P","A4","converted_f32_exact_as_f64","cache_E","cache_P","fault_site","fade_index","guide_commit","o2_commit","invalidations","epoch","physical_index"],"cases":summaries}),
    )
}
pub fn qualify(out: &Path) -> Result<Value> {
    let deadlines = crate::identity::deadlines::corrected()?;
    let joined = joined(out)?;
    let faults = fault_cases(out, &Kernel::Corrected)?;
    let qr = crate::identity::qr::corrected()?;
    let layout = crate::identity::layout::prepared()?;
    Ok(
        json!({"joined":joined,"withdrawal":deadlines,"packed":qr,"layout":layout,"arithmetic":faults}),
    )
}
