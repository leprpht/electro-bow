//! Independent DI and typed event clock for the 48 kHz joined capture fixture.
use super::capture;
use crate::{Result, ensure, pitch};
use std::f64::consts::TAU;
type Token = (u32, u32);
pub type Schedule = Vec<(usize, Option<usize>, [f64; 12])>;
#[derive(Default)]
struct Events {
    gesture: Option<Gesture>,
    confirm: Option<(Token, usize)>,
    launch: Option<(Token, usize, f64)>,
    dispose: Option<(usize, f64)>,
}
#[derive(Copy, Clone)]
enum Gesture {
    Pick,
    Mute,
    Hold,
}
#[derive(PartialEq, Copy, Clone)]
enum Phase {
    Idle,
    Pending,
    Confirmed,
    Active,
    Hold,
    Release,
}
struct Job {
    pub_at: usize,
    done: usize,
    owner: Option<Token>,
    target: Option<[f64; 12]>,
}
fn events(t: usize) -> Events {
    let mut e = Events::default();
    match t {
        0 | 9479 | 15000 => e.gesture = Some(Gesture::Pick),
        479 => {
            e.confirm = Some(((0, 1), 5500));
            e.launch = Some(((0, 1), 1, 0.));
        }
        7679 => e.gesture = Some(Gesture::Hold),
        9719 => {
            e.dispose = Some((1, 0.));
            e.confirm = Some(((0, 2), 11000));
            e.launch = Some(((0, 2), 2, 0.));
        }
        13000 => e.gesture = Some(Gesture::Mute),
        14680 => e.dispose = Some((2, 0.)),
        15599 => {
            e.confirm = Some(((0, 3), 17000));
            e.launch = Some(((0, 3), 3, 0.));
        }
        _ => {}
    }
    e
}
pub fn input(t: usize) -> f64 {
    (1..=10)
        .map(|k| {
            0.12 * (TAU * 196. * f64::from(k) * (t as f64 / 48000.)).cos() / f64::from(k).powf(0.7)
        })
        .sum()
}
pub fn schedule(fault: Option<usize>) -> Result<Schedule> {
    const HOP: usize = 240;
    const D: usize = 3;
    const M: usize = 327;
    const BUDGET: usize = 5;
    let fir = pitch::fir(D);
    let mut hp = Vec::new();
    let mut reduced = Vec::new();
    let mut previous = 0.;
    let alpha = -(-240f64 / (48000. * 0.04)).exp_m1();
    let mut serial = 0;
    let mut history = std::collections::BTreeMap::new();
    let mut owner = None;
    let mut confirmed = None;
    let mut live = None;
    let mut epoch = None;
    let mut last_epoch = 0;
    let mut phase = Phase::Idle;
    let mut expires = 0;
    let mut anchor = 0;
    let mut blocked = 0;
    let mut published = [0.; 12];
    let mut held = [0.; 12];
    let mut published_end = None;
    let mut job: Option<Job> = None;
    let mut schedule = Schedule::new();
    for t in 0..18500 {
        let e = if t >= blocked {
            events(t)
        } else {
            Events::default()
        };
        match e.gesture {
            Some(Gesture::Pick) => {
                serial += 1;
                owner = Some((0, serial));
                confirmed = None;
                phase = Phase::Pending;
                anchor = t;
                published = [0.; 12];
                published_end = None;
                job = None;
                history.clear();
            }
            Some(Gesture::Mute) => {
                owner = None;
                confirmed = None;
                phase = Phase::Release;
                job = None;
                published = [0.; 12];
            }
            Some(Gesture::Hold) if owner.is_some() && owner == live => {
                job = None;
                phase = Phase::Hold;
            }
            _ => {}
        }
        if (phase == Phase::Pending && t - anchor >= 5760)
            || (phase == Phase::Confirmed && t > expires)
        {
            owner = None;
            confirmed = None;
            job = None;
            published_end = None;
            published = [0.; 12];
            phase = Phase::Idle;
        }
        let x = input(t);
        let coeff = (-TAU * 30. / 48000.).exp();
        hp.push(coeff * (hp.last().copied().unwrap_or(0.) + x - previous));
        previous = x;
        if (t + 1) % D == 0 {
            let y: f64 = fir.iter().zip(hp.iter().rev()).map(|(w, s)| w * s).sum();
            reduced.push(y);
        }
        let due = (t + 1) % HOP == 0;
        if due {
            let frame_owner = Some((
                0,
                if t < 9479 {
                    1
                } else if t < 15000 {
                    2
                } else {
                    3
                },
            ));
            let good = !(7679..9479).contains(&t) && frame_owner == owner;
            history.insert(t, good);
            if !good {
                job = None;
            }
        }
        let supported = history.last_key_value().is_some_and(|(_, v)| *v);
        if let Some((token, until)) = e.confirm
            && owner == Some(token)
            && phase == Phase::Pending
            && supported
            && until >= t
        {
            confirmed = owner;
            expires = until;
            phase = Phase::Confirmed;
        }
        let mut launch = e.launch;
        if let Some((old, energy)) = e.dispose {
            if epoch != Some(old) || energy != 0. || super::stereo::envelope(t) != 0. {
                launch = None;
            } else {
                if owner == live {
                    owner = None;
                    confirmed = None;
                    job = None;
                    published = [0.; 12];
                    phase = Phase::Idle;
                }
                live = None;
                epoch = None;
                held = [0.; 12];
            }
        }
        if let Some((token, next, energy)) = launch
            && Some(token) == confirmed
            && confirmed == owner
            && phase == Phase::Confirmed
            && epoch.is_none()
            && energy == 0.
            && super::stereo::envelope(t) == 0.
            && next > last_epoch
        {
            epoch = Some(next);
            last_epoch = next;
            live = owner;
            phase = Phase::Active;
            if published_end.is_none_or(|end| t - end > 2 * HOP) {
                published = [0.; 12];
            }
            held = published;
        }
        if due {
            if let Some(retired) = job.take()
                && retired.pub_at == t
                && retired.done <= t
                && retired.owner == owner
                && supported
                && let Some(target) = retired.target
            {
                for (v, w) in published.iter_mut().zip(target) {
                    *v += alpha * (w - *v);
                }
                published_end = Some(t - HOP);
                if phase == Phase::Active && owner == live {
                    held = published;
                }
            }
            if owner.is_some()
                && matches!(phase, Phase::Active | Phase::Confirmed | Phase::Pending)
                && supported
            {
                let span = (M - 1) * D;
                if t >= span + 32 * D + anchor {
                    let center = t - span - 16 * D;
                    let baseline = (center + 1) / HOP * HOP;
                    if center - anchor >= 1200
                        && baseline > 0
                        && (baseline - 1..=t)
                            .step_by(HOP)
                            .all(|at| history.get(&at) == Some(&true))
                    {
                        let target = capture::target(&reduced[reduced.len() - M..], 16000., 196.);
                        job = Some(Job {
                            pub_at: t + HOP,
                            done: t + M.div_ceil(BUDGET) - 1,
                            owner,
                            target,
                        });
                    }
                }
            }
        }
        let value = held.map(|x| 10f64.powf(x / 20.) - 1.);
        if (t == 0 || due || e.gesture.is_some() || e.confirm.is_some() || e.dispose.is_some())
            && schedule
                .last()
                .is_none_or(|(_, ep, old)| *ep != epoch || *old != value)
        {
            schedule.push((t, epoch, value));
        }
        if fault == Some(t) {
            blocked = t + 240;
            owner = None;
            confirmed = None;
            live = None;
            epoch = None;
            job = None;
            published_end = None;
            phase = Phase::Idle;
            held = [0.; 12];
            published = [0.; 12];
            history.clear();
        }
    }
    ensure(!schedule.is_empty(), "empty joined capture")?;
    Ok(schedule)
}
