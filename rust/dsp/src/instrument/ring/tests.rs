use super::*;
use std::hint::black_box;

fn prepared(rate: f64, arm: Arm) -> DelayRing {
    let mut ring = DelayRing::prepare(rate, arm).unwrap();
    ring.launch(440.0, 0.1).unwrap();
    ring
}

fn push(ring: &mut DelayRing, wave: f64) {
    ring.stage(wave).unwrap().commit();
}

// Eager negative-time prefix followed by every prescribed outgoing sample.
// No modulo indexing, write cursor, validity count or shared seed routine.
fn prefill(capacity: usize, arm: Arm, period: f64, speed: f64) -> Vec<f64> {
    (-(capacity as i32)..0)
        .map(|n| {
            let t = f64::from(n) / period;
            let (phase, direction) = match arm {
                Arm::Bridge => (t, 1.0),
                Arm::Nut => (t + 1.0 - 0.127, -1.0),
            };
            direction * (speed / 0.127) * (phase.rem_euclid(1.0) - 0.5)
        })
        .collect()
}

fn compare(actual: f64, history: &[f64], delay: f64) {
    // Start at the oldest contributing sample, walking forward in absolute time.
    let lag = delay.ceil();
    let i = history.len() - lag as usize;
    let older = history[i];
    if lag == delay {
        assert_eq!(actual, older);
        return;
    }
    let newer = history[i + 1];
    let expected = older + (lag - delay) * (newer - older);
    let bound = 8.0 * f64::EPSILON * newer.abs().max(older.abs());
    assert!(
        (actual - expected).abs() <= bound,
        "D={delay} {actual} vs {expected}"
    );
}

#[test]
fn impulse_before_commit() {
    let mut ring = prepared(48000.0, Arm::Bridge);
    let c = ring.samples.len();
    // Flush the launch seed through ordinary commits, then inject at n=0.
    for _ in 0..c {
        push(&mut ring, 0.0);
    }
    for n in 0..4 * c {
        for (delay, m, u) in [
            (1.0, 1, 0.0),
            (3.25, 3, 0.25),
            ((c - 2) as f64, c - 2, 0.0),
            (c as f64 - 0.25, c - 1, 0.75),
            (c as f64, c, 0.0),
        ] {
            let expected = if n == m {
                1.0 - u
            } else if n == m + 1 {
                u
            } else {
                0.0
            };
            assert_eq!(ring.read(delay), Ok(expected), "tick {n}, tap {delay}");
        }
        push(&mut ring, f64::from(n == 0));
    }
}

#[test]
fn full_capacity_neighbor_across_wraps() {
    let mut ring = prepared(32000.0, Arm::Nut);
    let c = ring.samples.len();
    let mut history = Vec::new();
    for n in 0..5 * c {
        if n >= c {
            for d in [c as f64, c as f64 - 0.75, (c as f64).next_down(), 1.0] {
                compare(ring.read(d).unwrap(), &history, d);
            }
        }
        let wave = (n % 197) as f64 / 64.0;
        history.push(wave);
        push(&mut ring, wave);
    }
}

#[test]
fn moving_taps_match_chronological_history() {
    for rate in [
        32000.0, 44100.0, 44150.5, 48000.0, 88200.0, 96000.0, 176400.0, 192000.0,
    ] {
        for arm in [Arm::Bridge, Arm::Nut] {
            let mut ring = prepared(rate, arm);
            let c = ring.samples.len();
            assert_eq!(c, (rate / 65.0).ceil() as usize + 4);
            let launch_period = rate / 440.0;
            let mut history = prefill(c, arm, launch_period, 0.1);
            if matches!(arm, Arm::Bridge) {
                for offset in [1, 2] {
                    let db = 0.127 * launch_period - 1.0;
                    compare(
                        ring.read_at(db, offset).unwrap(),
                        &history[..c - offset],
                        db,
                    );
                }
            }
            let ticks = (rate * 0.1).ceil() as usize;
            let mut random = 1u32;
            for n in 0..ticks {
                // Downward retune while older lags still address the launch seed;
                // then sweep both ways to the supported playing limits.
                let f = if n < 100 {
                    440.0
                } else if n < ticks / 2 {
                    65.0 + 1255.0 * (n - 100) as f64 / (ticks / 2 - 100) as f64
                } else {
                    1320.0 - 1255.0 * (n - ticks / 2) as f64 / (ticks - 1 - ticks / 2) as f64
                };
                let p = rate / f;
                let feedback = match arm {
                    Arm::Bridge => 0.127 * p - 1.0,
                    Arm::Nut => (1.0 - 0.127) * p,
                };
                let crossing = [7.0_f64.next_down(), 7.0, 7.0_f64.next_up(), 7.0, 6.75][n % 5];
                let jump = if n % 2 == 0 { 1.0 } else { c as f64 - 0.25 };
                for d in [feedback, 0.127 * p / 2.0, crossing, jump] {
                    compare(ring.read(d).unwrap(), &history, d);
                }
                // Four distinct evolving histories, each carried across wraps.
                random ^= random << 13;
                random ^= random >> 17;
                random ^= random << 5;
                let wave = match (4 * n / ticks, n % 2) {
                    (0, _) => 0.375,
                    (1, _) => n as f64 / ticks as f64,
                    (2, 0) => -1.0,
                    (2, _) => 1.0,
                    _ => 2.0 * f64::from(random) / f64::from(u32::MAX) - 1.0,
                };
                history.push(wave);
                push(&mut ring, wave);
            }
        }
    }
}

#[test]
fn invalidation_and_relaunch_hide_stale_slots() {
    let mut ring = prepared(192000.0, Arm::Nut);
    let c = ring.samples.len();
    for _ in 0..c + 17 {
        push(&mut ring, 3.25);
    }
    ring.invalidate();
    for lag in 1..=c {
        assert_eq!(ring.read(lag as f64), Ok(0.0));
    }
    assert!(ring.stage(0.0).is_err());
    ring.launch(65.0, 0.16).unwrap();
    let mut fresh = prefill(c, Arm::Nut, 192000.0 / 65.0, 0.16);
    for n in 0..=c {
        for d in [1.0, 1.25, c as f64 - 0.5, c as f64] {
            compare(ring.read(d).unwrap(), &fresh, d);
        }
        let wave = (n % 7) as f64 / 8.0;
        fresh.push(wave);
        push(&mut ring, wave);
    }
}

#[test]
fn rejected_operations_preserve_both_arms() {
    for rate in [
        f64::NAN,
        f64::INFINITY,
        32000.0_f64.next_down(),
        192000.0_f64.next_up(),
    ] {
        assert!(DelayRing::prepare(rate, Arm::Bridge).is_none());
    }
    let mut b = DelayRing::prepare(44100.0, Arm::Bridge).unwrap();
    let mut n = prepared(44100.0, Arm::Nut);
    assert_eq!(b.read(1.5), Ok(0.0));
    assert!(b.stage(0.0).is_err());
    b.launch(1320.0, 0.06).unwrap();
    push(&mut b, -4.0);
    push(&mut n, 4.0);
    let snapshot = |r: &DelayRing| {
        (
            r.write,
            r.count,
            (1..=r.samples.len())
                .map(|l| r.read(l as f64).unwrap())
                .collect::<Vec<_>>(),
        )
    };
    let before = (snapshot(&b), snapshot(&n));
    for value in [
        f64::NAN,
        f64::INFINITY,
        (-4.0_f64).next_down(),
        4.0_f64.next_up(),
    ] {
        let _pending = b.stage(1.0).unwrap();
        assert!(n.stage(value).is_err());
    }
    for d in [
        f64::NAN,
        f64::INFINITY,
        1.0_f64.next_down(),
        (b.samples.len() as f64).next_up(),
    ] {
        assert_eq!(b.read(d), Err(RingFault));
    }
    assert_eq!(b.read_at(1.0, usize::MAX), Err(RingFault));
    assert_eq!(b.read_at(b.samples.len() as f64, 1), Err(RingFault));
    for (f, v) in [
        (0.0, 0.1),
        (f64::NAN, 0.1),
        (440.0, f64::INFINITY),
        (440.0, -0.1),
    ] {
        assert_eq!(b.launch(f, v), Err(RingFault));
    }
    assert_eq!(before, (snapshot(&b), snapshot(&n)));
}

#[test]
fn prepared_ring_has_zero_heap_operations() {
    let mut ring = prepared(192000.0, Arm::Bridge);
    let operations = crate::ffi::heap_operations(|| {
        for _ in 0..3 {
            ring.invalidate();
            assert_eq!(ring.read(1.0), Ok(0.0));
            ring.launch(1320.0, 0.1).unwrap();
            for i in 0..4 * ring.samples.len() {
                let d = 1.25 + (i % (ring.samples.len() - 1)) as f64;
                black_box(ring.read(black_box(d)).unwrap());
                push(&mut ring, black_box(0.125));
            }
            assert!(ring.stage(f64::NAN).is_err());
        }
    });
    assert_eq!(operations, 0);
    println!("ring metadata bytes {}", std::mem::size_of::<DelayRing>());
}

#[test]
fn measured_arm_impulse_phase_delay() {
    let mut worst = 0.0_f64;
    for rate in [
        32000.0, 44100.0, 48000.0, 88200.0, 96000.0, 176400.0, 192000.0,
    ] {
        for hz in [65.41, 440.0, 1318.51] {
            let p = rate / hz;
            let omega = std::f64::consts::TAU / p;
            for (arm, d) in [
                (Arm::Bridge, 0.127 * p - 1.0),
                (Arm::Nut, (1.0 - 0.127) * p),
            ] {
                let mut ring = prepared(rate, arm);
                for _ in 0..ring.samples.len() {
                    push(&mut ring, 0.0);
                }
                let mut first = None;
                let (mut re, mut im) = (0.0, 0.0);
                for n in 0..=ring.samples.len() {
                    let v = ring.read(d).unwrap();
                    if v != 0.0 {
                        let offset = n - *first.get_or_insert(n);
                        assert!(v > 0.0 && offset <= 1);
                        let angle = -(offset as f64) * omega;
                        re += v * angle.cos();
                        im += v * angle.sin();
                    }
                    push(&mut ring, f64::from(n == 0));
                }
                let measured = first.unwrap() as f64 - im.atan2(re) / omega;
                worst = worst.max((measured - d).abs());
            }
        }
    }
    assert!(worst < 0.0011);
    println!("42 isolated arm impulse phase delays, maximum error {worst} samples");
}

// Read-only instrumentation for the integrated guide's transaction and epoch tests.
impl DelayRing {
    pub(in crate::instrument) fn epoch_state(&self) -> (usize, usize, Option<(f64, f64)>) {
        (
            self.write,
            self.count,
            self.seed.map(|s| (s.period, s.amplitude)),
        )
    }

    pub(in crate::instrument) fn storage(&self) -> &[f64] {
        &self.samples
    }

    pub(in crate::instrument) fn poison_history(&mut self) {
        self.samples.fill(f64::NAN);
    }
}
