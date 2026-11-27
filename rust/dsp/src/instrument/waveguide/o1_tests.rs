//! Chronological oracle. The prefix maps indices 0..C to physical times -C..0.
use super::*;

const POSITIONS: [f64; 3] = [0.174 - 0.024 / 2.0, 0.174, 0.174 + 0.024 / 2.0];
const TOL: f64 = 1e-9;

fn at(history: &[f64], now: usize, delay: f64) -> f64 {
    let older = now - delay.ceil() as usize;
    let newer = now - delay.floor() as usize;
    let blend = delay.ceil() - delay;
    history[older] + blend * (history[newer] - history[older])
}

#[derive(Default)]
struct Metrics {
    ticks: usize,
    worst: f64,
    peak: f64,
}

impl Metrics {
    fn compare(&mut self, o: Observation, history: &[f64], rate: f64, hz: f64, n: usize) {
        let now = history.len() - 1;
        let p = rate / hz;
        let delays = POSITIONS.map(|s| [(s - 0.127) * p / 2.0, (2.0 - s - 0.127) * p / 2.0]);
        let mut check = |name, station: Option<usize>, actual: f64, expected: f64| {
            let error = (actual - expected).abs();
            assert!(
                error <= TOL,
                "first O1 divergence {name} rate={rate} tick={n} Hz={hz} station={station:?} delays={delays:?} physical indices={:?} {actual} vs {expected}",
                delays
                    .map(|pair| pair
                        .map(|d| (n as i64 - d.ceil() as i64, n as i64 - d.floor() as i64)))
            );
            self.worst = self.worst.max(error);
        };
        let mut aperture = 0.0;
        for (i, [d1, d2]) in delays.into_iter().enumerate() {
            let a = at(history, now, d1);
            let b = at(history, now, d2);
            let station = o.stations[i];
            check("direct", Some(i), station.direct, a);
            check("reflected history", Some(i), station.reflected_history, b);
            check("station", Some(i), station.velocity(), a - b);
            aperture += [1.0, 4.0, 1.0][i] / 6.0 * (a - b);
        }
        check("aperture", None, o.velocity(), aperture);
        assert!(o.velocity().abs() <= 8.0);
        self.peak = self.peak.max(o.velocity().abs());
        self.ticks += 1;
    }
}

fn rates() -> Vec<f64> {
    let mut rates: Vec<_> = RATES.map(|fs| fs as f64).into();
    let edge = 65.0_f64 * 740.0;
    rates.extend([
        32000.0_f64.next_up(),
        44150.5,
        edge.next_down(),
        edge,
        edge.next_up(),
        192000.0_f64.next_down(),
    ]);
    rates
}

fn check_fixture(
    rate: f64,
    launch_hz: f64,
    v: f64,
    ratio: f64,
    frames: usize,
    controls: impl Fn(usize) -> (f64, bool),
    metrics: &mut [Metrics; 2],
) {
    let [full, direct] = metrics;
    let c = (rate / 65.0).ceil() as usize + 4;
    let mut guide = launched(rate);
    // A replaced, saturated epoch leaves physical memory underneath the new seed.
    for _ in 0..c + 1 {
        guide.tick(440.0, 0.1, K).unwrap();
    }
    guide.launch(launch_hz, v).unwrap();
    let p0 = rate / launch_hz;
    let saw = |t: f64| v / 0.127 * (t.rem_euclid(1.0) - 0.5);
    let times = (-(c as i32)..0).map(|n| f64::from(n) / p0);
    let mut bridge: Vec<_> = times.clone().map(saw).collect();
    let mut nut: Vec<_> = times.map(|t| -saw(t + 1.0 - 0.127)).collect();
    let mut committed = nut.clone();
    let mut loss = [1, 2].map(|lag| at(&bridge, c - lag, 0.127 * p0 - 1.0));
    let k = ratio * v / 1.65;
    let loads = KNOTS.map(|q| q + k * (0.35 + 0.85 * (-q / 0.01).exp() + 0.45 * (-q / 0.1).exp()));
    let mut mode = Mode::Slide(Direction::Positive);
    for n in 0..frames {
        let (hz, lifted) = controls(n);
        let p = rate / hz;
        let x = at(&bridge, c + n, 0.127 * p - 1.0);
        let a = -at(&nut, c + n, (1.0 - 0.127) * p);
        let b = -0.98 * (0.125 * x + 0.75 * loss[0] + 0.125 * loss[1]);
        let (next, j, _) = if lifted {
            (Mode::Stick, 0.0, v - a - b)
        } else {
            eager_contact(v - a - b, mode, &loads)
        };
        mode = next;
        bridge.push(a + j);
        nut.push(b + j);
        loss = [x, loss[0]];
        let (sample, o) = guide
            .tick_observed(hz, v, if lifted { 0.0 } else { k })
            .unwrap()
            .unwrap();
        full.compare(o, &nut, rate, hz, n);
        committed.push(sample.contact.to_nut);
        direct.compare(o, &committed, rate, hz, n);
        assert_eq!(
            sample.contact.mode, mode,
            "branch rate={rate} tick={n} Hz={hz}"
        );
        let bow = Station::read(&guide.nut, p, BETA).velocity();
        assert!((bow - (sample.contact.to_nut + sample.from_nut)).abs() <= TOL);
        assert_eq!(Station::read(&guide.nut, p, 1.0).velocity().to_bits(), 0);
        let expected = ((n + 1) % c, (n + 1).min(c), Some((p0, v / 0.127)));
        assert_eq!(guide.nut.epoch_state(), expected);
        assert_eq!(guide.bridge.epoch_state(), expected);
        assert_eq!((guide.launch_count, guide.fault_count), (2, 0));
    }
}

#[test]
fn chronological_grid() {
    let mut metrics = [Metrics::default(), Metrics::default()];
    let mut cases = 0;
    for rate in rates() {
        let c = (rate / 65.0).ceil() as usize + 4;
        for hz in [65.0, 65.41, 82.41, 196.0, 440.0, 1318.51, 1320.0] {
            for v in [0.06, 0.16] {
                for r in [1.2, 2.4] {
                    check_fixture(rate, hz, v, r, 3 * c + 1, |_| (hz, false), &mut metrics);
                    cases += 1;
                }
            }
        }
        let frames = (rate * 0.1).ceil() as usize;
        check_fixture(
            rate,
            440.0,
            0.1,
            1.8,
            frames,
            |n| (if n < 100 { 440.0 } else { 65.0 }, false),
            &mut metrics,
        );
        // Cross the sub-sample boundary both ways, with moving seed/storage seams.
        check_fixture(
            rate,
            65.0,
            0.1,
            1.8,
            frames,
            |n| {
                let t = n as f64 / (frames - 1) as f64;
                (
                    65.0 + 1255.0 * (1.0 - (2.0 * t - 1.0).abs()),
                    n >= frames * 3 / 4,
                )
            },
            &mut metrics,
        );
        check_fixture(
            rate,
            440.0,
            0.1,
            1.8,
            (rate * 0.4) as usize,
            |n| {
                let t = n as f64 / rate;
                (
                    440.0 * 2.0_f64.powf(35.0 / 1200.0 * (std::f64::consts::TAU * 5.0 * t).sin()),
                    false,
                )
            },
            &mut metrics,
        );
        cases += 3;
    }
    let [full, direct] = metrics;
    println!(
        "O1 cases={cases} ticks={} full_max={:e} committed_max={:e} peak={} unexpected_faults=0 missing=0",
        full.ticks, full.worst, direct.worst, full.peak
    );
}

#[test]
fn analytical_histories() {
    // P=1000 makes every spatial delay a half-integer. For exp(i*omega*n)
    // the linear two-tap response is the ideal delay times cos(omega/2).
    // P=93.4 separately exercises asymmetric phase and amplitude error.
    for p in [1000.0, 93.4] {
        let mut ring = DelayRing::prepare(96000.0, Arm::Nut).unwrap();
        ring.launch(196.0, 0.1).unwrap();
        for _ in 0..1500 {
            ring.stage(3.5).unwrap().commit();
        }
        assert_eq!(Observation::read(&ring, p).velocity(), 0.0);
        for _ in 0..1500 {
            ring.stage(0.0).unwrap().commit();
        }
        for n in 0..1000 {
            ring.stage(f64::from(n == 0)).unwrap().commit();
            let o = Observation::read(&ring, p);
            for (station, s) in o.stations.iter().zip(POSITIONS) {
                let impulse = |d: f64| (1.0 - (n as f64 - d).abs()).max(0.0);
                assert!((station.direct - impulse((s - 0.127) * p / 2.0)).abs() < 1e-12);
                assert!(
                    (station.reflected_history - impulse((2.0 - s - 0.127) * p / 2.0)).abs()
                        < 1e-12
                );
            }
        }
        for k in [1, 5, 6, 12, 37, 83] {
            let k = f64::from(k);
            let angle = std::f64::consts::PI * k;
            let omega = 2.0 * angle / p;
            if omega >= std::f64::consts::PI {
                continue;
            }
            let aperture = (2.0 + (angle * 0.012).cos()) / 3.0;
            // Integrate sin(k*pi*(1-s)) exactly over the rectangular aperture.
            let integral =
                ((angle * (1.0 - 0.186)).cos() - (angle * (1.0 - 0.162)).cos()) / (angle * 0.024);
            let center = (angle * (1.0 - 0.174)).sin();
            assert!((integral - center * (angle * 0.012).sin() / (angle * 0.012)).abs() < 1e-12);
            for n in 0..1200 {
                ring.stage((omega * n as f64).cos()).unwrap().commit();
                if n < 900 {
                    continue;
                }
                let o = Observation::read(&ring, p);
                for (station, s) in o.stations.iter().zip(POSITIONS) {
                    for (ray, d) in [
                        (station.direct, (s - 0.127) * p / 2.0),
                        (station.reflected_history, (2.0 - s - 0.127) * p / 2.0),
                    ] {
                        let u = d.fract();
                        let re = 1.0 - u + u * omega.cos();
                        let im = -u * omega.sin();
                        let predicted =
                            re.hypot(im) * (omega * (n as f64 - d.floor()) + im.atan2(re)).cos();
                        assert!(
                            (ray - predicted).abs() < 1e-11,
                            "phasor k={k} d={d} tick={n}"
                        );
                    }
                }
                if p == 1000.0 {
                    let phase = omega * (n as f64 - (1.0 - 0.127) * p / 2.0);
                    let ideal = -2.0 * phase.sin() * center * aperture;
                    assert!(
                        (o.velocity() - ideal * (omega / 2.0).cos()).abs() < 1e-11,
                        "Simpson modal factor k={k}"
                    );
                }
            }
        }
    }
}

fn telemetry(s: Sample) -> ([u64; 11], Mode, u8) {
    let c = s.contact;
    (
        [
            s.from_nut,
            s.from_bridge,
            s.x,
            s.pickup,
            s.audio,
            c.to_bridge,
            c.to_nut,
            c.j,
            c.slip,
            c.force,
            c.string_velocity,
        ]
        .map(f64::to_bits),
        c.mode,
        c.segment_probes,
    )
}

#[test]
fn lifecycle_parity() {
    let mut physical = Guide::prepare(32000.0).unwrap();
    let mut observed = Guide::prepare(32000.0).unwrap();
    for n in 0..4000 {
        for g in [&mut physical, &mut observed] {
            match n {
                10 | 1300 | 2200 | 3200 => g.launch(440.0, 0.1).unwrap(),
                1250 | 3100 => g.invalidate(),
                2099 => g.edit_proposal = Some(|s| s.contact.to_nut = f64::NAN),
                2100 => g.edit_proposal = None,
                _ => {}
            }
        }
        let hz = if n == 3000 {
            f64::NAN
        } else if n % 401 < 200 {
            1320.0
        } else {
            65.0
        };
        let k = if observed.live && n % 500 < 400 {
            K
        } else {
            0.0
        };
        let a = physical.tick(hz, 0.1, k);
        let b = observed.tick_observed(hz, 0.1, k);
        assert_eq!(
            a.map(|v| v.map(telemetry)),
            b.map(|v| v.map(|(s, o)| {
                assert!(o.velocity().is_finite());
                telemetry(s)
            })),
            "observed lifecycle tick={n}"
        );
        for (a, b) in [
            (&physical.bridge, &observed.bridge),
            (&physical.nut, &observed.nut),
        ] {
            assert_eq!(a.epoch_state(), b.epoch_state());
            assert!(
                a.storage()
                    .iter()
                    .zip(b.storage())
                    .all(|(x, y)| x.to_bits() == y.to_bits())
            );
        }
        let scalar = |g: &Guide| {
            (
                g.loss.map(f64::to_bits),
                g.mode,
                g.previous_dc.to_bits(),
                g.previous_pickup.to_bits(),
                g.live,
                g.launch_count,
                g.fault_count,
            )
        };
        assert_eq!(scalar(&physical), scalar(&observed));
    }
    assert_eq!((observed.launch_count, observed.fault_count), (4, 2));
}

#[test]
fn observed_allocation() {
    for rate in [32000.0, 192000.0] {
        let mut g = Guide::prepare(rate).unwrap();
        let count = crate::ffi::heap_operations(|| {
            for _ in 0..3 {
                assert!(
                    g.tick_observed(440.0, 0.0, 0.0).unwrap().is_none(),
                    "cleared epoch produced O1"
                );
                g.launch(1320.0, 0.1).unwrap();
                for n in 0..12000 {
                    let hz = 65.0 + 1255.0 * (n % 4096) as f64 / 4095.0;
                    let (_, o) = g
                        .tick_observed(hz, 0.1, if n < 10000 { K } else { 0.0 })
                        .unwrap()
                        .unwrap();
                    black_box(o.velocity());
                }
                assert!(g.tick_observed(f64::NAN, 0.1, K).is_err());
                g.invalidate();
            }
        });
        assert_eq!(count, 0);
        println!("O1 rate={rate} heap operations={count}");
    }
}

#[test]
fn partition_invariance() {
    // Entries carry the absolute frame and all observed ray bits. Faults and
    // empty samples remain separate events on the same integer clock.
    let render = |blocks: &[usize]| {
        let mut g = Guide::prepare(48000.0).unwrap();
        let mut trace = Vec::new();
        for &size in blocks {
            let start = trace.len();
            for n in start..start + size {
                match n {
                    17 | 1501 | 4703 => g.launch(196.0, 0.1).unwrap(),
                    4200 => g.invalidate(),
                    _ => {}
                }
                let hz = if n == 1500 {
                    f64::NAN
                } else if n < 1451 {
                    196.0
                } else {
                    1320.0
                };
                let k = if g.live && n < 3900 { K } else { 0.0 };
                let event = g.tick_observed(hz, 0.1, k).map(|v| {
                    v.map(|(_, o)| {
                        (
                            o.stations.map(|s| {
                                [s.direct, s.reflected_history, s.velocity()].map(f64::to_bits)
                            }),
                            o.velocity().to_bits(),
                        )
                    })
                });
                trace.push((n, event, g.nut.epoch_state(), g.launch_count, g.fault_count));
            }
        }
        trace
    };
    const N: usize = 6016;
    let single = render(&[1; N]);
    assert_eq!(render(&[64; N / 64]), single);
    let mut blocks = Vec::new();
    let mut random = 19u32;
    let mut n = 0;
    for end in [17, 1451, 1500, 1501, 3900, 4200, 4703, N] {
        while n < end {
            random = random.wrapping_mul(1664525).wrapping_add(1013904223);
            let step = (1 + random as usize % 257).min(end - n);
            blocks.extend([0, step]);
            n += step;
        }
    }
    assert_eq!(render(&blocks), single);
    println!(
        "O1 partition {N} timestamps, {} variable blocks including zero",
        blocks.len()
    );
}

#[test]
fn preserves_fraction_at_post_commit() {
    let mut ring = DelayRing::prepare(32000.0, Arm::Nut).unwrap();
    ring.launch(1320.0, 0.1).unwrap();
    ring.stage(1.0).unwrap().commit();
    ring.stage(0.0).unwrap().commit();
    let d = 0.5_f64.next_down();
    assert_eq!(
        ring.read_committed(d).to_bits(),
        d.to_bits(),
        "lost original fraction"
    );
}

#[test]
fn capacity_transition_edges() {
    for multiple in 493..=2953 {
        let edge = 65.0 * f64::from(multiple);
        for rate in [edge.next_down(), edge, edge.next_up()] {
            let c = (rate / 65.0).ceil() as usize + 4;
            let mut ring = DelayRing::prepare(rate, Arm::Nut).unwrap();
            assert_eq!(ring.storage().len(), c);
            ring.launch(65.0, 0.16).unwrap();
            ring.stage(-4.0).unwrap().commit();
            for p in [rate / 65.0, rate / 1320.0] {
                for s in POSITIONS {
                    for d in [(s - 0.127) * p / 2.0, (2.0 - s - 0.127) * p / 2.0] {
                        assert!(d.floor() as usize + 2 <= c);
                        assert!(ring.read_committed(d).abs() <= 4.0);
                    }
                }
            }
        }
    }
}

#[test]
fn first_high_note_uses_committed_emission() {
    let mut g = Guide::prepare(32000.0).unwrap();
    g.launch(1320.0, 0.16).unwrap();
    let p: f64 = 32000.0 / 1320.0;
    let d = (0.162 - 0.127) * p / 2.0;
    let phase = -1.0 / p + 1.0 - 0.127;
    let minus_one = -(0.16 / 0.127) * (phase - phase.floor() - 0.5);
    let (s, o) = g
        .tick_observed(1320.0, 0.16, 2.4 * 0.16 / 1.65)
        .unwrap()
        .unwrap();
    let expected = (1.0 - d) * s.contact.to_nut + d * minus_one;
    assert!(
        (o.stations[0].direct - expected).abs() <= TOL,
        "committed emission rate=32000 tick=0 Hz=1320 s=.162 d={d} indices=[0,-1] {} vs {expected}",
        o.stations[0].direct
    );
}

#[test]
#[ignore = "manual isolated timing, run alone in release mode"]
fn isolated_read_cost() {
    use std::time::Instant;
    for rate in [32000, 48000, 192000] {
        let mut g = launched(f64::from(rate));
        for _ in 0..rate {
            g.tick(440.0, 0.1, K).unwrap();
        }
        let batch = || {
            for n in 0..10 * rate {
                let p = f64::from(rate) / if n % 4096 < 2048 { 196.0 } else { 440.0 };
                black_box(Observation::read(black_box(&g.nut), black_box(p)).velocity());
            }
        };
        batch();
        for _ in 0..5 {
            let start = Instant::now();
            batch();
            println!(
                "O1 saturated history rate {rate} ns/sample {:.3}",
                start.elapsed().as_secs_f64() * 1e9 / f64::from(10 * rate)
            );
        }
    }
}
