use super::*;
use std::hint::black_box;

const RATES: [usize; 7] = [32000, 44100, 48000, 88200, 96000, 176400, 192000];
const K: f64 = 1.8 * 0.1 / 1.65;

fn launched(rate: f64) -> Guide {
    let mut g = Guide::prepare(rate).unwrap();
    g.launch(440.0, 0.1).unwrap();
    g
}

fn absolute_delay(response: &[(usize, f64)], omega: f64) -> f64 {
    let first = response.first().unwrap().0;
    let (mut re, mut im, mut mass) = (0.0, 0.0, 0.0);
    for &(n, gain) in response {
        let offset = n - first;
        assert!(
            gain > 0.0 && offset <= 4 && offset as f64 * omega < std::f64::consts::PI,
            "positive compact return"
        );
        let angle = -(offset as f64) * omega;
        re += gain * angle.cos();
        im += gain * angle.sin();
        mass += gain;
    }
    assert!((mass - 0.98).abs() < 1e-12);
    first as f64 - im.atan2(re) / omega
}

// Only the outgoing bridge write is replaced at this cut. The capture is
// a, upstream of the cut, after the actual signed reflections and both taps.
#[test]
fn passive_absolute_arrival() {
    for fs in RATES {
        for hz in [65.41, 440.0, 1318.51] {
            let rate = fs as f64;
            let mut g = launched(rate);
            let capacity = g.bridge.storage().len();
            g.edit_proposal = Some(|s| s.contact.to_bridge = 0.0);
            for _ in 0..3 * capacity {
                g.tick(hz, 0.0, 0.0).unwrap();
            }
            let omega = std::f64::consts::TAU * hz / rate;
            let mut response = Vec::new();
            for n in 0..2 * capacity {
                if n == 0 {
                    g.edit_proposal = Some(|s| s.contact.to_bridge = 1.0);
                }
                if n == 1 {
                    g.edit_proposal = Some(|s| s.contact.to_bridge = 0.0);
                }
                let gain = g.tick(hz, 0.0, 0.0).unwrap().unwrap().from_nut;
                if gain != 0.0 {
                    response.push((n, gain));
                }
            }
            let measured = absolute_delay(&response, omega);
            let period = rate / hz;
            println!(
                "passive {fs} {hz} {measured:.12} {:+.12}",
                measured - period
            );
            assert!(
                (measured - period).abs() <= 0.003,
                "absolute arrival {fs} {hz} {measured}"
            );
            let early = response
                .iter()
                .map(|&(n, v)| (n - 1, v))
                .collect::<Vec<_>>();
            let late = response
                .iter()
                .map(|&(n, v)| (n + 1, v))
                .collect::<Vec<_>>();
            for wrong in [&[(0, 0.98)][..], &early, &late] {
                assert!((absolute_delay(wrong, omega) - period).abs() > 0.003);
            }
        }
    }
}

// An absolute-time eager prefix and append-only timeline. The interpolant
// walks forward from the older sample, independently of the ring's cursor.
fn eager_read(history: &[f64], delay: f64) -> f64 {
    let lag = delay.ceil();
    let i = history.len() - lag as usize;
    if lag == delay {
        history[i]
    } else {
        history[i] + (lag - delay) * (history[i + 1] - history[i])
    }
}

// Test-only graph enumeration for this fixed positive-load trajectory. Walk all
// segments in ascending q, keeping the greatest admissible sliding root.
const KNOTS: [f64; 10] = [0.0, 0.005, 0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28];

fn eager_contact(d: f64, previous: Mode, loads: &[f64; 10]) -> (Mode, f64, f64) {
    let h = d.abs();
    let slide = Mode::Slide(if d > 0.0 {
        Direction::Positive
    } else {
        Direction::Negative
    });
    if d == 0.0 || (previous != slide && h <= loads[0]) {
        return (Mode::Stick, d, 0.0);
    }
    let mut q = 0.0_f64;
    for (i, pair) in loads.windows(2).enumerate() {
        if pair[0] <= h && h <= pair[1] {
            let root = if pair[0] == pair[1] {
                KNOTS[i + 1]
            } else {
                KNOTS[i] + (h - pair[0]) * (KNOTS[i + 1] - KNOTS[i]) / (pair[1] - pair[0])
            };
            q = q.max(root);
        }
    }
    if h >= loads[9] {
        q = q.max(KNOTS[9] + (h - loads[9]));
    }
    if q <= 1e-12 {
        assert!(h <= loads[0] + 1e-12);
        (Mode::Stick, d, 0.0)
    } else {
        let slip = q.copysign(d);
        (slide, d - slip, slip)
    }
}

#[test]
fn evolving_eager_trajectory() {
    let mut worst = 0.0_f64;
    for fs in RATES {
        let rate = fs as f64;
        let mut g = launched(rate);
        let c = (rate / 65.0).ceil() as usize + 4;
        // Stale physical memory must stay hidden after deliberate replacement.
        for _ in 0..c + 3 {
            g.tick(880.0, 0.12, 1.8 * 0.12 / 1.65).unwrap();
        }
        g.invalidate();
        for arm in [&g.bridge, &g.nut] {
            assert_eq!(arm.epoch_state(), (0, 0, None));
            assert_eq!(arm.read(1.0), Ok(0.0));
        }
        g.launch(440.0, 0.1).unwrap();
        let p0 = rate / 440.0;
        let ramp = |phase: f64| (0.1 / 0.127) * (phase.rem_euclid(1.0) - 0.5);
        let mut bridge: Vec<_> = (1..=c).rev().map(|lag| ramp(-(lag as f64) / p0)).collect();
        let mut nut: Vec<_> = (1..=c)
            .rev()
            .map(|lag| -ramp(-(lag as f64) / p0 + 1.0 - 0.127))
            .collect();
        let mut loss = [1, 2].map(|i| eager_read(&bridge[..c - i], 0.127 * p0 - 1.0));
        let mut mode = Mode::Slide(Direction::Positive);
        let loads =
            KNOTS.map(|q| q + K * (0.35 + 0.85 * (-q / 0.01).exp() + 0.45 * (-q / 0.1).exp()));
        for n in 0..fs / 10 {
            for arm in [&g.bridge, &g.nut] {
                assert_eq!(
                    arm.epoch_state(),
                    (n % c, n.min(c), Some((p0, 0.1 / 0.127))),
                    "epoch {fs}/{n}"
                );
            }
            let hz = if n < 100 { 440.0 } else { 65.0 };
            let p = rate / hz;
            let x = eager_read(&bridge, 0.127 * p - 1.0);
            let a = -eager_read(&nut, (1.0 - 0.127) * p);
            let b = -0.98 * (0.125 * x + 0.75 * loss[0] + 0.125 * loss[1]);
            let pickup = eager_read(&bridge, 0.127 * p / 2.0);
            let (next_mode, j, slip) = eager_contact((0.1 - a) - b, mode, &loads);
            let (to_bridge, to_nut) = (a + j, b + j);
            let previous = g.loss;
            assert_eq!(g.mode, mode, "preceding branch {fs}/{n}");
            let s = g.tick(hz, 0.1, K).unwrap().unwrap();
            for (name, actual, expected) in [
                ("pre x1", previous[0], loss[0]),
                ("pre x2", previous[1], loss[1]),
                ("x", s.x, x),
                ("a", s.from_nut, a),
                ("b", s.from_bridge, b),
                ("p", s.pickup, pickup),
                ("B", s.contact.to_bridge, to_bridge),
                ("N", s.contact.to_nut, to_nut),
                ("j", s.contact.j, j),
                ("r", s.contact.slip, slip),
                ("committed x1", g.loss[0], x),
                ("committed x2", g.loss[1], loss[0]),
                ("stored B", g.bridge.read(1.0).unwrap(), to_bridge),
                ("stored N", g.nut.read(1.0).unwrap(), to_nut),
            ] {
                let error = (actual - expected).abs();
                assert!(
                    error <= 1e-9,
                    "first divergence {fs}/{n} {name} {actual} vs {expected}"
                );
                worst = worst.max(error);
            }
            assert_eq!(s.contact.mode, next_mode, "branch {fs}/{n}");
            assert_eq!(g.mode, next_mode);
            assert_eq!((g.launch_count, g.fault_count), (2, 0));
            bridge.push(to_bridge);
            nut.push(to_nut);
            loss = [x, loss[0]];
            mode = next_mode;
        }
    }
    println!("evolving eager max abs {worst:e}");
}

fn check_fault(g: &mut Guide, hz: f64, speed: f64, k: f64) {
    // Observe the same private transaction before public tick invalidates.
    let state = |g: &Guide| {
        (
            g.bridge.epoch_state(),
            g.nut.epoch_state(),
            g.mode,
            g.loss.map(f64::to_bits),
            g.previous_pickup.to_bits(),
            g.previous_dc.to_bits(),
        )
    };
    let slots = |ring: &DelayRing| {
        ring.storage()
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    };
    let before = state(g);
    let storage = (slots(&g.bridge), slots(&g.nut));
    assert!(g.transaction(hz, speed, k).is_err());
    assert_eq!(state(g), before, "partial scalar/cursor commit");
    assert!(
        storage == (slots(&g.bridge), slots(&g.nut)),
        "partial write"
    );
    let faults = g.fault_count;
    assert!(g.tick(hz, speed, k).is_err());
    assert_eq!(g.fault_count, faults + 1);
    assert!(!g.live);
    for arm in [&g.bridge, &g.nut] {
        assert_eq!(arm.epoch_state(), (0, 0, None));
        assert_eq!(arm.read(1.0), Ok(0.0));
    }
    assert!(storage == (slots(&g.bridge), slots(&g.nut)));
    assert!(g.tick(440.0, 0.0, 0.0).unwrap().is_none());
}

#[test]
fn atomic_fault_boundary() {
    let edits: &[fn(&mut Sample)] = &[
        |s| s.x = f64::NAN,
        |s| s.from_nut = 4.01,
        |s| s.from_bridge = f64::NAN,
        |s| s.contact.j = -4.01,
        |s| s.contact.slip = f64::NAN,
        |s| s.contact.force = f64::INFINITY,
        |s| s.contact.to_bridge = f64::NAN,
        |s| s.contact.to_nut = f64::NAN,
        |s| s.contact.to_nut = -4.01,
        |s| s.pickup = 4.01,
        |s| s.pickup = f64::NAN,
    ];
    for edit in edits {
        let mut g = launched(48000.0);
        g.edit_proposal = Some(*edit);
        check_fault(&mut g, 440.0, 0.1, K);
    }
    for i in 0..5 {
        let mut g = launched(48000.0);
        for _ in 0..1000 {
            g.tick(440.0, 0.1, K).unwrap();
        }
        match i {
            0 => g.bridge.poison_history(),
            1 => g.nut.poison_history(),
            2 => g.loss[1] = f64::NAN,
            3 => g.previous_dc = f64::NAN,
            _ => g.loss[0] = -4.01,
        }
        check_fault(&mut g, 440.0, 0.1, K);
    }
    for (hz, v, k) in [
        (f64::NAN, 0.1, K),
        (64.9, 0.1, K),
        (1320.1, 0.1, K),
        (440.0, f64::INFINITY, K),
        (440.0, -0.1, K),
        (440.0, 0.0, K),
        (440.0, 0.1, -0.1),
        (440.0, 0.1, f64::NAN),
        (440.0, 0.1, 1.0),
    ] {
        check_fault(&mut launched(48000.0), hz, v, k);
    }
}

#[test]
fn lifecycle_and_allocation() {
    for rate in [32000.0, 192000.0] {
        let mut g = Guide::prepare(rate).unwrap();
        let capacity = g.bridge.storage().len();
        let operations = crate::ffi::heap_operations(|| {
            for _ in 0..(2.0 * rate) as usize {
                assert!(g.tick(196.0, 0.0, 0.0).unwrap().is_none());
            }
            for _ in 0..3 {
                g.launch(440.0, 0.1).unwrap();
                for n in 0..4 * capacity {
                    let hz = if n < 100 { 440.0 } else { 65.0 };
                    let k = if n < capacity { K } else { 0.0 };
                    let s = black_box(g.tick(hz, 0.1, k).unwrap().unwrap());
                    if k == 0.0 {
                        assert_eq!((s.contact.j, g.mode), (0.0, Mode::Stick));
                    }
                }
                assert!(g.tick(f64::NAN, 0.1, K).is_err());
                assert!(g.launch(440.0, f64::NAN).is_err());
                g.invalidate();
            }
        });
        assert_eq!(operations, 0);
        println!("rate {rate} capacity {capacity} heap operations {operations}");
    }
}

#[test]
fn expired_seed_cannot_reexcite_the_guide() {
    let mut g = launched(192000.0);
    g.edit_proposal = Some(|s| s.contact.to_bridge = 0.0);
    for _ in 0..3 * g.bridge.storage().len() {
        g.tick(1320.0, 0.0, 0.0).unwrap();
    }
    g.edit_proposal = None;
    for n in 0..3 * g.bridge.storage().len() {
        let hz = if n % 2 == 0 { 65.0 } else { 1320.0 };
        let (s, observation) = g.tick_observed(hz, 0.0, 0.0).unwrap().unwrap();
        assert_eq!(observation.velocity(), 0.0);
        assert_eq!(
            [
                s.from_nut,
                s.from_bridge,
                s.x,
                s.pickup,
                s.contact.to_nut,
                s.contact.to_bridge
            ],
            [0.0; 6]
        );
    }
}

#[path = "o1_tests.rs"]
mod o1;
