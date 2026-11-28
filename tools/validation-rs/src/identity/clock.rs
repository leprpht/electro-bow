//! Independent host-sample controller, capture support and publication replay.
mod controls;
pub(super) mod joined;
use super::capture;
use crate::{Result, ensure, evidence, pitch};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::TAU,
    fs,
    path::Path,
};
type Token = [i64; 2];
#[derive(Clone, Deserialize)]
struct Frame {
    hz: f64,
    quality: f64,
    amplitude: f64,
    floor: f64,
    supported: bool,
    movement: f64,
    owner: Token,
    compatible: bool,
}
impl Frame {
    fn good(&self) -> bool {
        self.supported
            && self.compatible
            && (65. ..=1320.).contains(&self.hz)
            && (0.74..=1.).contains(&self.quality)
            && self.amplitude.is_finite()
            && self.amplitude >= (3. * self.floor).max(0.001)
            && (0.0001..=0.002).contains(&self.floor)
            && (0. ..=1.).contains(&self.movement)
    }
}
#[derive(Clone, Deserialize)]
struct Case {
    frames: BTreeMap<usize, Frame>,
    events: BTreeMap<usize, Vec<Value>>,
    stamps: BTreeSet<(i64, usize)>,
    count: usize,
    rate: f64,
    defect: Option<String>,
    constant_after: Option<usize>,
    digest: String,
}
#[derive(Default)]
struct Actions<'a> {
    gesture: Option<&'a str>,
    confirm: Option<(Token, i64)>,
    launch: Option<(Token, usize, f64)>,
    dispose: Option<(usize, f64)>,
}
const PRIORITY: [&str; 9] = [
    "reset",
    "fault",
    "pick",
    "mute",
    "change",
    "hold",
    "compatible",
    "release",
    "timeout",
];
fn decode(group: &[Value], generation: i64) -> Option<Actions<'_>> {
    let mut seen = BTreeSet::new();
    let mut a = Actions::default();
    for v in group {
        let k = v.get("kind")?.as_str()?;
        if !seen.insert(k) {
            return None;
        }
        let token = || {
            let t: Token = serde_json::from_value(v["token"].clone()).ok()?;
            (t[0] == generation && t[1] >= 0).then_some(t)
        };
        let epoch = || {
            let n = usize::try_from(v["epoch"].as_u64()?).ok()?;
            (n > 0).then_some(n)
        };
        match k {
            "confirm" => a.confirm = Some((token()?, v["valid_through"].as_i64()?)),
            "launch" => a.launch = Some((token()?, epoch()?, v["energy"].as_f64()?)),
            "dispose" => a.dispose = Some((epoch()?, v["energy"].as_f64()?)),
            _ if PRIORITY.contains(&k) => {}
            _ => return None,
        }
    }
    a.gesture = PRIORITY.iter().copied().find(|k| seen.contains(k));
    Some(a)
}
fn deadline(rate: f64) -> Result<usize> {
    use num_rational::BigRational;
    use num_traits::ToPrimitive;
    let r = BigRational::from_float(rate).ok_or("invalid sample rate")?;
    ((r * BigRational::new(3.into(), 25.into()))
        .ceil()
        .to_integer())
    .to_usize()
    .ok_or_else(|| "invalid acquisition interval".into())
}
struct Job {
    owner: Token,
    pub_at: usize,
    start: usize,
    done: usize,
    target: Option<[f64; 12]>,
    hz: f64,
    alpha: f64,
}
fn replay(c: &Case, parts: &[usize]) -> Result<Vec<Value>> {
    replay_input(c, parts, |t| {
        (1..=10)
            .map(|k| {
                0.08 * (TAU * f64::from(k) * 196. * (t as f64 / c.rate)).cos()
                    / f64::from(k).powf(0.65)
            })
            .sum()
    })
}
fn replay_input(c: &Case, parts: &[usize], sample: impl Fn(usize) -> f64) -> Result<Vec<Value>> {
    replay_with_fault(c, parts, sample, None)
}
fn replay_with_fault(
    c: &Case,
    parts: &[usize],
    sample: impl Fn(usize) -> f64,
    fault: Option<usize>,
) -> Result<Vec<Value>> {
    let rate = c.rate;
    let d = (rate / 16000.).floor() as usize;
    let fa = rate / d as f64;
    let hop = d * (0.005 * fa + 0.5).floor() as usize;
    let da = (rate / 8000.).floor() as usize;
    let ha = da * (0.005 * rate / da as f64 + 0.5).floor() as usize;
    let budget = ((4. * fa / 65.).ceil() as usize + 1).div_ceil(hop);
    let acquire = deadline(rate)?;
    let mut serial = 0;
    let mut generation = 0;
    let mut owner = None;
    let mut live = None;
    let mut confirmed = None;
    let mut epoch = None;
    let mut last_epoch = 0;
    let mut origin = 0;
    let mut anchor = 0;
    let mut expiry = 0;
    let mut phase = "idle";
    let mut published = [0.; 12];
    let mut held = [0.; 12];
    let mut published_end = None;
    let mut job: Option<Job> = None;
    let mut history: BTreeMap<usize, Option<&Frame>> = BTreeMap::new();
    let taps = pitch::fir(d);
    let coeff = (-TAU * 30. / rate).exp();
    let mut hp = Vec::<f64>::new();
    let mut dec = Vec::<f64>::new();
    let mut prev = 0.;
    let mut rows = Vec::new();
    let mut start = 0;
    ensure(parts.iter().any(|v| *v > 0), "nonadvancing partition")?;
    for &part in parts.iter().cycle() {
        if start >= c.count {
            break;
        }
        let end = c.count.min(start + part);
        for t in start..end {
            let group = c.events.get(&t).map_or(&[][..], Vec::as_slice);
            let mut actions = decode(group, generation);
            if fault.is_some_and(|f| t > f && t < f + (0.005 * rate).round_ties_even() as usize)
                && !group.iter().any(|v| v["kind"] == "reset")
            {
                actions = Some(Actions::default());
            }
            let mut note = Vec::new();
            if actions.is_none() {
                note.push("invalid frame");
            }
            let Actions {
                gesture,
                confirm,
                mut launch,
                mut dispose,
            } = actions.unwrap_or_default();
            let mut consumed = None;
            match gesture {
                Some("reset") => {
                    generation += 1;
                    origin = t;
                    last_epoch = 0;
                    hp.clear();
                    dec.clear();
                    prev = 0.;
                    owner = None;
                    live = None;
                    confirmed = None;
                    epoch = None;
                    job = None;
                    published_end = None;
                    phase = "idle";
                    held = [0.; 12];
                    published = [0.; 12];
                    history.clear();
                }
                Some("fault" | "release" | "mute" | "timeout") => {
                    owner = None;
                    confirmed = None;
                    phase = "release";
                    job = None;
                    published = [0.; 12];
                }
                Some("pick" | "change") => {
                    serial += 1;
                    owner = Some([generation, serial]);
                    confirmed = None;
                    phase = "pending";
                    published_end = None;
                    anchor = t;
                    published = [0.; 12];
                    job = None;
                    history.clear();
                }
                Some("hold") if owner.is_some() && owner == live => {
                    job = None;
                    phase = "hold";
                }
                Some("compatible") if owner.is_some() && owner == live => phase = "active",
                _ => {}
            }
            if (phase == "pending" && t - anchor >= acquire)
                || (phase == "confirmed" && t as i64 > expiry)
            {
                owner = None;
                confirmed = None;
                job = None;
                published_end = None;
                published = [0.; 12];
                phase = "idle";
                note.push("expired");
            }
            let x = sample(t);
            let x = if c.constant_after.is_some_and(|at| t >= at) {
                -0.9
            } else {
                x
            };
            hp.push(coeff * (hp.last().copied().unwrap_or(0.) + x - prev));
            prev = x;
            let age = t - origin;
            if (age + 1) % d == 0 {
                dec.push(taps.iter().zip(hp.iter().rev()).map(|(a, b)| a * b).sum());
            }
            if (age + 1) % ha == 0 {
                let fr = c.frames.get(&t);
                history.insert(t, fr);
                if fr.is_none_or(|f| {
                    !f.good()
                        || Some(f.owner) != owner
                        || job
                            .as_ref()
                            .is_some_and(|j| (1200. * (f.hz / j.hz).log2()).abs() > 25.)
                }) {
                    job = None;
                }
                if let Some(erase) = t
                    .checked_sub((((4. / 65. + 0.035) * rate / ha as f64).ceil() as usize + 3) * ha)
                {
                    history.remove(&erase);
                }
            }
            let last = history.last_key_value().and_then(|(_, v)| *v);
            if let Some((token, limit)) = confirm
                && (Some(token) != owner || limit < t as i64)
            {
                note.push("invalid confirmation");
            }
            if let Some((old, energy)) = dispose
                && (epoch != Some(old) || energy != 0.)
            {
                note.push("invalid disposal");
                dispose = None;
                launch = None;
            }
            if let Some((token, until)) = confirm
                && !matches!(
                    gesture,
                    Some("reset" | "fault" | "mute" | "release" | "timeout")
                )
                && Some(token) == owner
                && phase == "pending"
                && last.is_some_and(|f| f.good() && Some(f.owner) == owner)
                && until >= t as i64
            {
                confirmed = owner;
                expiry = until;
                phase = "confirmed";
            }
            if gesture == Some("fault") || dispose.is_some() {
                if owner == live {
                    owner = None;
                    confirmed = None;
                    job = None;
                    published = [0.; 12];
                    phase = "idle";
                }
                live = None;
                epoch = None;
                held = [0.; 12];
            }
            if let Some((token, next, energy)) = launch {
                if Some(token) != confirmed || energy != 0. || epoch.is_some() || next <= last_epoch
                {
                    note.push("invalid launch");
                }
                if Some(token) == confirmed
                    && confirmed == owner
                    && phase == "confirmed"
                    && epoch.is_none()
                    && energy == 0.
                    && next > last_epoch
                {
                    epoch = Some(next);
                    last_epoch = next;
                    live = owner;
                    phase = "active";
                    if published_end.is_none_or(|at| t - at > 2 * hop) {
                        published = [0.; 12];
                    }
                    held = published;
                    consumed = live;
                }
            }
            if let Some(j) = &job
                && t == j.done
            {
                note.push(if j.target.is_some() {
                    "complete"
                } else {
                    "feature rejected"
                });
            }
            let mut due = (age + 1) % hop == 0;
            if due && !c.stamps.contains(&(generation, t)) {
                job = None;
                note.push("missing identity hop");
                due = false;
            }
            if due && let Some(j) = job.take() {
                if Some(j.owner) == owner
                    && j.pub_at == t
                    && t >= j.done
                    && last.is_some_and(|f| f.good() && (1200. * (f.hz / j.hz).log2()).abs() <= 25.)
                    && let Some(v) = j.target
                {
                    published_end = Some(t - hop);
                    for (a, b) in published.iter_mut().zip(v) {
                        *a += j.alpha * (b - *a);
                    }
                    if phase == "active" && owner == live {
                        held = published;
                        consumed = live;
                    }
                    note.push("publish");
                } else {
                    note.push("discard");
                }
            }
            let mut window = None;
            if due
                && let (Some(token), Some(frame)) = (owner, last)
                && matches!(phase, "pending" | "confirmed" | "active")
                && frame.good()
                && frame.owner == token
            {
                let m = (4. * fa / frame.hz + 0.5).floor() as usize;
                let span = (m - 1) * d;
                if t >= origin + span + 32 * d && t >= anchor + span + 32 * d {
                    let first = t - span - 32 * d;
                    let center = t - span - 16 * d;
                    let base = origin + (center - origin + 1) / ha * ha;
                    if center - anchor >= (0.025 * rate).ceil() as usize
                        && base > 0
                        && (base - 1..=t).step_by(ha).all(|at| {
                            history.get(&at).copied().flatten().is_some_and(|f| {
                                f.good()
                                    && f.owner == token
                                    && (1200. * (f.hz / frame.hz).log2()).abs() <= 25.
                            })
                        })
                    {
                        let done = t + m.div_ceil(budget) - 1;
                        let target = capture::target(&dec[dec.len() - m..], fa, frame.hz);
                        let mut j = Job {
                            owner: token,
                            pub_at: t + hop,
                            start: t,
                            done,
                            target,
                            hz: frame.hz,
                            alpha: -(-(hop as f64) / (rate * (0.060 - 0.040 * frame.movement)))
                                .exp_m1(),
                        };
                        if done == t {
                            note.push(if j.target.is_some() {
                                "complete"
                            } else {
                                "feature rejected"
                            });
                        }
                        window = Some([first, t, center + t - 16 * d, done, t + hop]);
                        if c.defect.as_deref() == Some("wrong epoch") {
                            j.owner = [-1, -1];
                        }
                        if c.defect.as_deref() == Some("late") {
                            j.pub_at += 1;
                            j.start += 1;
                        }
                        job = Some(j);
                        note.push("snapshot");
                    }
                }
            }
            if !group.is_empty() || due || !note.is_empty() {
                rows.push(json!({"tick":t,"generation":generation,"event":gesture,"phase":phase,"owner":owner,"live":live,"epoch":epoch,"confirmed":confirmed,"pending":if matches!(phase,"pending"|"confirmed"){owner}else{None},"consumed":consumed,"actions":note,"window":window,"held":held,"target":held.map(|v|10f64.powf(v/20.)-1.),"job":job.as_ref().map(|j|(j.owner[0],j.owner[1],j.start)),"held_without_support":live.is_some() && (phase!="active" || last.is_none_or(|f|!f.good())),"published":published}));
            }
            if fault == Some(t) {
                owner = None;
                confirmed = None;
                live = None;
                epoch = None;
                job = None;
                published_end = None;
                phase = "idle";
                held = [0.; 12];
                published = [0.; 12];
                history.clear();
            }
        }
        start = end;
    }
    Ok(rows)
}
fn discrete(rows: &[Value]) -> Result<String> {
    let mut items = rows.to_vec();
    for r in &mut items {
        let o = r.as_object_mut().ok_or("clock row")?;
        for k in ["target", "held", "published"] {
            o.remove(k);
        }
    }
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&items)?)))
}
pub fn run(dir: &Path, reference: Option<&Path>) -> Result<Value> {
    let cases: Vec<Case> = serde_json::from_str(include_str!("../../contracts/clock.json"))?;
    let mut result = BTreeMap::new();
    let negatives = controls::run(&cases)?;
    for c in cases {
        let rows = replay(&c, &[127])?;
        let digest = discrete(&rows)?;
        evidence::atomic(&dir.join(format!("{}.json", c.digest)), &json!(rows))?;
        ensure(
            digest == c.digest,
            &format!("clock lifecycle {}, actual {digest}", c.digest),
        )?;
        let mut worst = 0f64;
        if let Some(path) = reference {
            let old = evidence::strict(&fs::read(
                path.join("clock").join(format!("{digest}.json")),
            )?)?;
            let frozen = old.as_array().ok_or("frozen rows")?;
            ensure(frozen.len() == rows.len(), "clock row count")?;
            for (a, b) in rows.iter().zip(frozen) {
                for k in ["target", "held", "published"] {
                    let actual: [f64; 12] = serde_json::from_value(a[k].clone())?;
                    let expected: [f64; 12] = serde_json::from_value(b[k].clone())?;
                    for (v, w) in actual.into_iter().zip(expected) {
                        worst = worst.max((v - w).abs());
                    }
                }
            }
            ensure(worst <= 2e-11, "clock held, target or published vector")?;
        }
        for p in [
            &[1][..],
            &[17],
            &[64],
            &[256],
            &[1024],
            &[4097],
            &[0, 17, 256, 1, 1024, 64],
        ] {
            ensure(replay(&c, p)? == rows, "clock partition")?;
        }
        result.insert(
            digest,
            json!({"rows":rows.len(),"maximum_vector_error":worst,"defect":c.defect}),
        );
    }
    Ok(json!({"cases":result,"negatives":negatives}))
}

fn event_fixture(rate: f64, launch: usize) -> Case {
    let da = (rate / 8000.).floor() as usize;
    let ha = da * (0.005 * rate / da as f64 + 0.5).floor() as usize;
    let d = (rate / 16000.).floor() as usize;
    let hop = d * (0.005 * rate / d as f64 + 0.5).floor() as usize;
    let count = launch.max(3500) + 400;
    let frames = (ha - 1..count)
        .step_by(ha)
        .map(|t| {
            (
                t,
                Frame {
                    hz: 196.,
                    quality: 0.95,
                    amplitude: 0.08,
                    floor: 0.00032,
                    supported: true,
                    movement: 0.5,
                    owner: [0, 1],
                    compatible: true,
                },
            )
        })
        .collect();
    Case {
        frames,
        events: [
            (0, vec![json!({"kind":"pick"})]),
            (
                launch,
                vec![
                    json!({"kind":"confirm","token":[0,1],"valid_through":12000}),
                    json!({"kind":"launch","token":[0,1],"epoch":1,"energy":0.}),
                ],
            ),
        ]
        .into(),
        stamps: (hop - 1..count).step_by(hop).map(|t| (0, t)).collect(),
        count,
        rate,
        defect: None,
        constant_after: None,
        digest: String::new(),
    }
}
pub fn event_controls() -> Result<Value> {
    let mut checks = BTreeMap::new();
    let mut collisions = BTreeMap::new();
    for (fs, t, ok) in [
        (48000., 5759, true),
        (48000., 5760, false),
        (48000., 5761, false),
        (32008.333333333336, 3841, true),
        (32008.333333333336, 3842, false),
    ] {
        let rows = replay(&event_fixture(fs, t), &[0, 17, 127])?;
        let row = rows
            .iter()
            .find(|r| r["tick"] == t)
            .ok_or("missing event")?;
        ensure(
            row["epoch"].is_u64() == ok,
            "elapsed acquisition eligibility",
        )?;
        checks.insert(format!("{fs}/{t}"), json!(row["epoch"]));
        let epoch = row["epoch"].as_u64().unwrap_or(0);
        if fs == 48000. {
            collisions.insert(t.to_string(), json!([epoch, row["actions"]]));
        } else {
            collisions.insert(format!("fractional/{t}"), json!(epoch));
        }
    }
    for (name, t) in [("timeout", 5760), ("reset", 5759), ("lost support", 5759)] {
        let mut c = event_fixture(48000., t);
        if name == "lost support" {
            c.frames.get_mut(&t).unwrap().supported = false;
        } else {
            c.events.get_mut(&t).unwrap().push(json!({"kind":name}));
        }
        ensure(
            replay(&c, &[127])?.iter().all(|r| r["epoch"].is_null()),
            "confirmation collision",
        )?;
    }
    let nominal = replay(&event_fixture(48000., 479), &[127])?;
    ensure(
        nominal
            .iter()
            .find(|r| r["tick"] == 479)
            .is_some_and(|r| r["epoch"] == 1),
        "physical epoch ownership",
    )?;
    for e in [
        json!({"kind":"hold"}),
        json!({"kind":"pick"}),
        json!({"kind":"confirm","token":[0,1],"valid_through":9000}),
        json!({"kind":"dispose","epoch":1,"energy":0.}),
        json!({"kind":"launch","token":[0,1],"epoch":2,"energy":0.}),
    ] {
        let mut c = event_fixture(48000., 479);
        c.events.insert(2000, vec![e.clone(), e]);
        let rows = replay(&c, &[127])?;
        let row = rows
            .iter()
            .find(|r| r["tick"] == 2000)
            .ok_or("duplicate frame")?;
        ensure(
            row["actions"] == json!(["invalid frame"])
                && row["phase"] == "active"
                && row["epoch"] == 1,
            "duplicate event rejection",
        )?;
    }
    let mut c = event_fixture(48000., 479);
    c.events.insert(3300, vec![json!({"kind":"hold"})]);
    let rows = replay(&c, &[127])?;
    let launch = rows.iter().find(|r| r["tick"] == 479).ok_or("no launch")?;
    ensure(launch["epoch"] == 1, "physical epoch ownership")?;
    ensure(
        rows.iter()
            .filter(|r| r["tick"].as_u64().is_some_and(|t| t >= 3300))
            .all(|r| !r["actions"].as_array().unwrap().contains(&json!("publish"))),
        "stale feature publication",
    )?;
    Ok(
        json!({"historical_collisions":collisions,"deadlines":checks,"duplicate_types":5,"collisions":3,"hold":3300}),
    )
}

pub(super) fn fault_schedule(fault: Option<usize>) -> Result<super::stereo_clock::Schedule> {
    let mut c = event_fixture(48000., 479);
    c.count = 8500;
    let base = c.frames[&239].clone();
    c.frames = (239..c.count)
        .step_by(240)
        .map(|t| {
            let mut f = base.clone();
            if fault.is_some() && t >= 6000 {
                f.owner = [0, 2];
            }
            (t, f)
        })
        .collect();
    c.stamps = (239..c.count).step_by(240).map(|t| (0, t)).collect();
    if let Some(t) = fault {
        // The numerical fault takes place after the normal capture/commit at t.
        c.events.insert(
            t + 1,
            vec![
                json!({"kind":"fault"}),
                json!({"kind":"dispose","epoch":1,"energy":0.}),
            ],
        );
        c.events.insert(6000, vec![json!({"kind":"pick"})]);
        c.events.insert(
            6479,
            vec![
                json!({"kind":"confirm","token":[0,2],"valid_through":8000}),
                json!({"kind":"launch","token":[0,2],"epoch":2,"energy":0.}),
            ],
        );
    }
    let mut schedule = Vec::new();
    for row in replay_input(&c, &[0, 1, 127, 8193], super::stereo_clock::input)? {
        let entry = (
            row["tick"].as_u64().ok_or("capture tick")? as usize,
            row["epoch"].as_u64().map(|i| i as usize),
            serde_json::from_value::<[f64; 12]>(row["target"].clone())?,
        );
        if schedule
            .last()
            .is_none_or(|previous: &(usize, Option<usize>, [f64; 12])| {
                previous.1 != entry.1 || previous.2 != entry.2
            })
        {
            schedule.push(entry);
        }
    }
    Ok(schedule)
}

pub(super) fn uneven(kind: &str) -> Result<Vec<Value>> {
    let mut c = event_fixture(47001., 2614);
    c.count = 14500;
    c.events = [
        (19, vec![json!({"kind":"reset"})]),
        (20, vec![json!({"kind":"pick"})]),
        (
            488,
            vec![json!({"kind":"confirm","token":[1,1],"valid_through":5000})],
        ),
        (
            2614,
            vec![json!({"kind":"launch","token":[1,1],"epoch":1,"energy":0.})],
        ),
    ]
    .into();
    let mut base = c.frames.values().next().unwrap().clone();
    base.owner = [1, 1];
    c.frames = (253..2604)
        .step_by(235)
        .map(|t| (t, base.clone()))
        .collect();
    c.stamps = (19 + 236 - 1..c.count)
        .step_by(236)
        .map(|t| (1, t))
        .collect();
    match kind {
        "missing-A2" => {
            c.frames.remove(&2603);
        }
        "missing-I1" => {
            c.stamps.remove(&(1, 2614));
        }
        "complete" => {}
        _ => return Err("uneven fixture".into()),
    }
    replay_input(&c, &[0, 1, 255, 19], |t| {
        (1..=10)
            .map(|k| {
                0.12 * (TAU * 196. * f64::from(k) * (t as f64 / c.rate)).cos()
                    / f64::from(k).powf(0.7)
            })
            .sum()
    })
}
pub(super) fn registered(rate: f64, launch: usize) -> Result<super::stereo_clock::Schedule> {
    let mut case = event_fixture(rate, launch);
    case.count = launch + 1;
    case.frames.retain(|&t, _| t == launch);
    let r = replay_input(&case, &[0, 1, 4097], |_| 0.)?;
    ensure(
        r.last()
            .is_some_and(|r| r["epoch"] == 1 && r["tick"] == launch),
        "registered held-control launch",
    )?;
    Ok(vec![(
        launch,
        Some(1),
        std::array::from_fn(|j| [-0.4376586748096509, 0.7782794100389229][j % 2]),
    )])
}

pub(super) fn recorded_source(x: &[f64]) -> Result<Vec<Value>> {
    let mut c = event_fixture(48000., 479);
    c.count = 15120;
    c.events.get_mut(&479).unwrap()[0]["valid_through"] = c.count.into();
    let base = c.frames[&239].clone();
    c.frames.clear();
    c.stamps = (239..c.count).step_by(240).map(|t| (0, t)).collect();
    let samples = |t: usize| if t < 14400 { x[t] } else { 0. };
    let hp = (-TAU * 30. / 48000.).exp();
    let alpha = -(-1f64 / (0.004 * 48000.)).exp_m1();
    let mut delay = 0.;
    let mut energy = 0.;
    for t in 0..c.count {
        let a = hp.mul_add(samples(t), delay);
        delay = (-hp).mul_add(samples(t), hp * a);
        let power = alpha.mul_add(a * a, energy);
        energy = (1. - alpha) * power;
        if (t + 1) % 240 == 0 {
            let mut f = base.clone();
            f.amplitude = power.sqrt();
            f.supported = t < 14400;
            c.frames.insert(t, f);
        }
    }
    replay_input(&c, &[127], samples)
}

pub(super) fn retire_clock() -> Result<Value> {
    let make = |pick: usize| {
        let expiry = 5760 + pick;
        let mut c = event_fixture(48000., expiry);
        let start = c.events.remove(&0).unwrap();
        c.events.insert(pick, start);
        c
    };
    let mut phases = Vec::new();
    for pick in [0, 120] {
        let expiry = 5760 + pick;
        let rows = replay_input(&make(pick), &[127], super::stereo_clock::input)?;
        let end = rows
            .iter()
            .find(|r| r["tick"] == expiry)
            .ok_or("expiry row")?;
        ensure(
            end["epoch"].is_null()
                && end["job"].is_null()
                && end["published"].as_array().unwrap().iter().all(|v| v == 0.),
            "expired scheduled work",
        )?;
        let prev = rows
            .iter()
            .rev()
            .find(|r| r["tick"].as_u64().unwrap() < (expiry as u64) && r["window"].is_array())
            .ok_or("pending window")?;
        phases.push(prev["window"][3].as_u64().unwrap() < expiry as u64);
    }
    ensure(phases == [false, true], "expiry work phases")?;
    Ok(json!(phases))
}
