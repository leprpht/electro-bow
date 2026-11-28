//! Reconstruct the independent controller evidence for each consumer-side transaction.
use super::*;
use crate::identity::stereo_clock::input;
fn event(kind: &str) -> Value {
    json!({"kind":kind})
}
fn confirm(owner: i32, through: usize) -> Value {
    json!({"kind":"confirm","token":[0,owner],"valid_through":through})
}
fn launch(owner: i32) -> Value {
    json!({"kind":"launch","token":[0,owner],"epoch":owner,"energy":0.})
}
fn dispose(epoch: i32, energy: f64) -> Value {
    json!({"kind":"dispose","epoch":epoch,"energy":energy})
}
fn carrier_clock(count: usize) -> Case {
    let mut c = event_fixture(48000., 479);
    c.count = count;
    let base = c.frames[&239].clone();
    c.frames = (239..count)
        .step_by(240)
        .map(|t| (t, base.clone()))
        .collect();
    c.stamps = (239..count).step_by(240).map(|t| (0, t)).collect();
    c
}
pub(crate) fn records(fault: Option<usize>) -> Result<Vec<Value>> {
    let mut c = carrier_clock(18500);
    c.events = [
        (0, vec![event("pick")]),
        (479, vec![confirm(1, 5500), launch(1)]),
        (7679, vec![event("hold")]),
        (9479, vec![event("pick")]),
        (9719, vec![dispose(1, 0.), confirm(2, 11000), launch(2)]),
        (13000, vec![event("mute")]),
        (14680, vec![dispose(2, 0.)]),
        (15000, vec![event("pick")]),
        (15599, vec![confirm(3, 17000), launch(3)]),
    ]
    .into();
    for (t, f) in &mut c.frames {
        f.owner = [
            0,
            if *t < 9479 {
                1
            } else if *t < 15000 {
                2
            } else {
                3
            },
        ];
        f.supported = !(7679..9479).contains(t);
    }
    let rows = replay_with_fault(&c, &[127], input, fault)?;
    ensure(
        replay_with_fault(&c, &[0, 17, 256, 1, 1024, 64], input, fault)? == rows,
        "joined event partition",
    )?;
    Ok(rows)
}
pub(crate) fn controls() -> Result<Value> {
    let mut c = carrier_clock(24000);
    let mut report = json!({});
    for (key, events) in [
        (
            "timeout_attempt",
            vec![
                (0, vec![event("pick")]),
                (5760, vec![event("timeout")]),
                (23900, vec![launch(1)]),
            ],
        ),
        (
            "automatic_expiry",
            vec![
                (0, vec![event("pick")]),
                (23900, vec![confirm(1, c.count), launch(1)]),
            ],
        ),
        (
            "confirmed_expiry",
            vec![
                (0, vec![event("pick")]),
                (480, vec![confirm(1, 3000)]),
                (3010, vec![launch(1)]),
            ],
        ),
    ] {
        c.events = events.into_iter().collect();
        let rows = replay_input(&c, &[127], input)?;
        ensure(
            rows.iter().all(|r| {
                r["epoch"].is_null()
                    && !(r["tick"].as_u64().unwrap() > 5760
                        && r["actions"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|a| a == "publish"))
            }),
            "expired identity",
        )?;
        report[key] = rows.last().ok_or("expiry evidence")?.clone();
    }
    c.events = [
        (0, vec![event("pick")]),
        (479, vec![confirm(1, c.count), launch(1)]),
        (7440, vec![event("pick")]),
        (8399, vec![confirm(2, 9000)]),
        (8400, vec![launch(2)]),
        (8401, vec![dispose(1, 0.1), launch(2)]),
        (8402, vec![dispose(2, 0.), launch(2)]),
        (8403, vec![dispose(1, 0.), launch(2)]),
    ]
    .into();
    let stale = replay_input(&c, &[127], input)?;
    ensure(
        stale
            .iter()
            .any(|r| r["tick"] == 8403 && r["epoch"].is_null()),
        "stale replacement support",
    )?;
    for (_, f) in c.frames.range_mut(7440..) {
        f.owner = [0, 2];
    }
    let r = replay_input(&c, &[127], input)?;
    let repl = r
        .iter()
        .filter(|r| {
            r["tick"]
                .as_u64()
                .is_some_and(|t| (8400..=8403).contains(&t))
        })
        .collect::<Vec<_>>();
    ensure(
        repl.len() == 4 && repl[..3].iter().all(|r| r["epoch"] == 1) && repl[3]["epoch"] == 2,
        "replacement ownership",
    )?;
    report["replacement"] = json!(repl);
    let mut windows = Vec::new();
    for kind in ["complete", "missing-A2", "missing-I1"] {
        let rows = uneven(kind)?;
        let w = rows
            .iter()
            .find(|r| r["window"].is_array())
            .ok_or("missing first uneven window")?;
        windows.push(w["window"].clone());
    }
    report["uneven"] = json!(windows);
    Ok(report)
}

pub(crate) fn projection() -> Result<Value> {
    use crate::numerics;
    let mut publications = 0;
    let mut maximum = 0f64;
    for (rate, origin) in [(48000usize, 0usize), (47001, 19)] {
        let fs = rate as f64;
        let d = rate / 16000;
        let da = rate / 8000;
        let fa = fs / d as f64;
        let hop = d * (0.005 * fa + 0.5).floor() as usize;
        let a2 = da * (0.005 * fs / da as f64 + 0.5).floor() as usize;
        let mut c = event_fixture(fs, 479);
        c.count = 8200;
        let owner = [i64::from(origin != 0), 1];
        let mut frame = c.frames.values().next().ok_or("prepared frame")?.clone();
        frame.owner = owner;
        c.frames = (origin + a2 - 1..c.count)
            .step_by(a2)
            .map(|t| (t, frame.clone()))
            .collect();
        c.events = [
            (origin + 1, vec![event("pick")]),
            (
                origin + 2 * a2 - 1,
                vec![json!({"kind":"confirm","token":owner,"valid_through":c.count})],
            ),
        ]
        .into();
        if origin != 0 {
            c.events.insert(origin, vec![event("reset")]);
        }
        c.stamps = (origin + hop - 1..c.count)
            .step_by(hop)
            .map(|t| (owner[0], t))
            .collect();
        let signal: Vec<_> = (0..c.count)
            .map(|t| {
                (1..=10)
                    .map(|k| {
                        0.12 * (TAU * 196. * f64::from(k) * (t as f64 / fs)).cos()
                            / f64::from(k).powf(0.7)
                    })
                    .sum::<f64>()
            })
            .collect();
        let rows = replay_input(&c, &[127], |i| signal[i])?;
        let coeff = (-TAU * 30. / fs).exp();
        let fir = pitch::fir(d);
        let mut history = Vec::new();
        let mut decimated = Vec::new();
        let mut delay = 0.;
        for (i, &v) in signal[origin..].iter().enumerate() {
            let y = coeff.mul_add(v, delay);
            delay = (-coeff).mul_add(v, coeff * y);
            history.push(y);
            if (i + 1) % d == 0 {
                decimated.push(
                    fir.iter()
                        .zip(history.iter().rev())
                        .map(|(a, b)| a * b)
                        .sum::<f64>(),
                );
            }
        }
        let m = (4. * fa / 196. + 0.5).floor() as usize;
        let mut previous = [0.; 12];
        for r in rows.iter().filter(|r| {
            r["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a == "publish")
        }) {
            let t = r["tick"].as_u64().ok_or("publish tick")? as usize;
            let end = (t - hop - origin + 1) / d;
            let a = numerics::partials(&decimated[end - m..end], fa, 196., 12)
                .into_iter()
                .map(|z| z.norm())
                .collect::<Vec<_>>();
            let floor = (3. * 0.00032f64)
                .max(1e-5)
                .max(0.015 * a.iter().copied().fold(0., f64::max));
            let weights = a
                .iter()
                .map(|v| ((v - floor) / (3. * floor)).clamp(0., 1.))
                .collect::<Vec<_>>();
            let levels = a
                .iter()
                .enumerate()
                .map(|(i, v)| 20. * ((i + 1) as f64 * v.max(floor)).log10())
                .collect::<Vec<_>>();
            ensure(
                weights.iter().filter(|v| **v > 0.).count() >= 2,
                "supported capture window",
            )?;
            let mean = weights.iter().zip(&levels).map(|(a, b)| a * b).sum::<f64>()
                / weights.iter().sum::<f64>();
            for j in 0..12 {
                let ln = (0.45 * (levels[j] - mean) * weights[j]).clamp(-5., 5.);
                previous[j] += -(-(hop as f64) / (0.04 * fs)).exp_m1() * (ln - previous[j]);
                maximum = maximum
                    .max((previous[j] - r["published"][j].as_f64().ok_or("published gain")?).abs());
            }
            publications += 1;
        }
    }
    ensure(
        publications == 48 && maximum <= 2e-11,
        "sliced source projection",
    )?;
    Ok(json!({"publications":publications,"max_db_error":maximum}))
}
