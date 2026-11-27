use super::*;
use Direction::{Negative, Positive};
use Mode::{Slide, Stick};
use std::hint::black_box;

fn input(d: f64, k: f64) -> ContactInput {
    ContactInput {
        from_nut: 0.0,
        from_bridge: 0.0,
        bow_speed: d,
        k,
    }
}

fn verify(curve: &FrictionCurve, input: ContactInput, previous: Mode) -> ContactResult {
    let result = curve.solve(input, previous).unwrap();
    assert_eq!(result, curve.solve(input, previous).unwrap());
    let ContactInput {
        from_nut: a,
        from_bridge: b,
        k,
        bow_speed,
    } = input;
    let d = bow_speed - a - b;
    let ContactResult {
        j,
        slip,
        mode,
        force,
        string_velocity: v,
        to_nut,
        to_bridge,
        segment_probes,
    } = result;
    let limit = k * curve.mu[0];
    assert!(segment_probes <= 9);
    assert_eq!(force, 2.0 * j);
    assert_eq!(to_nut, b + j);
    assert_eq!(to_bridge, a + j);
    assert!((v - (to_nut + a)).abs() < 1e-10 && (v - (to_bridge + b)).abs() < 1e-10);
    let flux = to_bridge * to_bridge + to_nut * to_nut - a * a - b * b;
    assert!((flux - force * v).abs() < 1e-10);
    assert!(force * slip >= -1e-12 && force.abs() <= 2.0 * limit + 2e-10);
    assert!((j + slip - d).abs() < 1e-10 && ((bow_speed - v) - slip).abs() < 1e-10);
    match mode {
        Stick if k == 0.0 => assert_eq!((j, slip), (0.0, d)),
        Stick => assert!(slip == 0.0 && (j - d).abs() < 1e-10 && j.abs() <= limit + 1e-10),
        Slide(sign) => {
            assert!(k > 0.0 && slip.abs() > SLIP_FLOOR && slip.signum() == d.signum());
            assert_eq!(sign, if d > 0.0 { Positive } else { Negative });
            assert!(previous == mode || d.abs() > limit);
            let q = slip.abs();
            let i = Q.partition_point(|&knot| knot <= q) - 1;
            let mut mu = curve.mu[i];
            if i + 1 < Q.len() {
                mu += (curve.mu[i + 1] - mu) * (q - Q[i]) / (Q[i + 1] - Q[i]);
            }
            assert!((q + k * mu - d.abs()).abs() < 1e-10);
            assert!((j - (k * mu).copysign(d)).abs() < 1e-10);
        }
    }
    result
}

#[test]
fn v02_sweeps_and_wave_flux() {
    let curve = FrictionCurve::prepare();
    let mut count = 0;
    let mut max_probes = 0;
    for k in [0.0, 0.03, 0.10, 0.23] {
        let mut ds: Vec<_> = (-2000..=2000).map(|n| f64::from(n) / 1000.0).collect();
        let limit = k * curve.mu[0];
        for h in Q
            .iter()
            .zip(curve.mu)
            .map(|(q, mu)| q + k * mu)
            .chain([limit - 1e-11, limit + 1e-11])
        {
            ds.extend([h, -h]);
        }
        let mut check = |d, previous| {
            let input = ContactInput {
                from_nut: 0.13,
                from_bridge: 0.10 - 0.13 - d,
                bow_speed: 0.10,
                k,
            };
            let result = verify(&curve, input, previous);
            max_probes = max_probes.max(result.segment_probes);
            count += 1;
            result.mode
        };
        for &d in &ds {
            for previous in [Stick, Slide(Positive), Slide(Negative)] {
                check(d, previous);
            }
        }
        ds.sort_by(f64::total_cmp);
        for _ in 0..2 {
            let mut previous = Stick;
            for &d in &ds {
                previous = check(d, previous);
            }
            ds.reverse();
        }
    }
    assert_eq!((count, max_probes), (80_500, 9));
}

#[test]
fn worked_branches_reversal_lift_and_tail() {
    let curve = FrictionCurve::prepare();
    let sample = input(0.14, 0.10);
    let static_result = verify(&curve, sample, Stick);
    assert_eq!((static_result.mode, static_result.force), (Stick, 0.28));
    let slide = verify(&curve, sample, Slide(Positive));
    assert_eq!(slide.mode, Slide(Positive));
    assert!((slide.slip - 0.08552223933509404).abs() < 1e-14);
    assert!((slide.force - 0.10895552132981196).abs() < 1e-14);
    assert_eq!(verify(&curve, input(-0.14, 0.10), slide.mode).mode, Stick);
    let reversed = verify(&curve, input(-0.20, 0.10), slide.mode);
    assert_eq!(reversed.mode, Slide(Negative));
    assert!((reversed.slip + 0.15525214545877897).abs() < 1e-14);
    let lift = verify(&curve, input(-0.14, 0.0), reversed.mode);
    assert_eq!(lift.mode, Stick);
    assert_eq!(verify(&curve, input(-0.14, 0.10), lift.mode).mode, Stick);
    let tail_start = Q[9] + 0.10 * curve.mu[9];
    for d in [
        tail_start.next_down(),
        tail_start,
        tail_start.next_up(),
        2.0,
    ] {
        for s in [-1.0, 1.0] {
            let result = verify(&curve, input(s * d, 0.10), Stick);
            assert!((result.slip.abs() - (d - 0.10 * curve.mu[9])).abs() < 1e-14);
            assert_eq!(result.segment_probes, u8::from(d < tail_start));
        }
    }
    for d in [0.0, -0.0] {
        assert_eq!(verify(&curve, input(d, 0.10), Slide(Negative)).mode, Stick);
    }
}

#[test]
fn stored_fold_loads_retain_the_upper_root() {
    let curve = FrictionCurve::prepare();
    for i in [2, 3, 4] {
        for target_width in [5e-13, 1e-14] {
            let k = (Q[i + 1] - Q[i] - target_width) / (curve.mu[i] - curve.mu[i + 1]);
            let left = Q[i] + k * curve.mu[i];
            let right = Q[i + 1] + k * curve.mu[i + 1];
            assert!(left < right && right - left < 1e-12);
            for h in [
                left,
                left.next_up(),
                (left + right) * 0.5,
                right.next_down(),
                right,
            ] {
                let q = Q[i] + (Q[i + 1] - Q[i]) * ((h - left) / (right - left));
                for (s, direction) in [(1.0, Positive), (-1.0, Negative)] {
                    let sample = input(s * h, k);
                    assert_eq!(verify(&curve, sample, Stick).mode, Stick);
                    let result = verify(&curve, sample, Slide(direction));
                    assert_eq!(result.mode, Slide(direction));
                    assert_eq!(result.slip, s * q);
                }
            }
            assert_eq!(
                verify(&curve, input(left.next_down(), k), Slide(Positive)).mode,
                Stick
            );
            assert!(verify(&curve, input(right.next_up(), k), Slide(Positive)).slip >= Q[i + 1]);
        }
    }
    let mu = curve.prepared_coefficients();
    // Preserve the two named Review 1 inputs on their original stored graph.
    if mu[3..=5].iter().map(|v| v.to_bits()).eq([
        0x3fea_abbc_5655_f763,
        0x3fe5_59cd_a364_61a3,
        0x3fe1_adf1_3325_a650,
    ]) {
        for (h, k, slip) in [
            (
                0.12026541114915246,
                0.12029965498642688,
                0.030000277554192455,
            ),
            (0.2726217499930773, 0.3486472674315129, 0.08),
        ] {
            for (s, direction) in [(1.0, Positive), (-1.0, Negative)] {
                assert_eq!(
                    verify(&curve, input(s * h, k), Slide(direction)).slip,
                    s * slip
                );
            }
        }
    }
    let center = (Q[5] - Q[4]) / (mu[4] - mu[5]);
    // An algebraic fold may have distinct rounded loads. Find a stored tie.
    let k = (center.to_bits() - 64..=center.to_bits() + 64)
        .map(f64::from_bits)
        .find(|k| Q[4] + k * mu[4] == Q[5] + k * mu[5])
        .expect("no coincident load within 64 ULP of the fold");
    let h = Q[5] + k * curve.mu[5];
    assert_eq!(Q[4] + k * curve.mu[4], h);
    for s in [-1.0, 1.0] {
        let mode = Slide(if s > 0.0 { Positive } else { Negative });
        assert_eq!(
            verify(&curve, input(s * h.next_down(), k), mode).mode,
            Stick
        );
        assert_eq!(verify(&curve, input(s * h, k), mode).slip, s * Q[5]);
        let above = verify(&curve, input(s * h.next_up(), k), mode);
        assert_eq!(above.mode, mode);
        assert!(above.slip.abs() >= Q[5]);
    }
}

#[test]
fn tiny_positive_load_preserves_the_m3b_slip_floor() {
    let curve = FrictionCurve::prepare();
    let k = f64::MIN_POSITIVE;
    // At this load h0 is negligible and the stored first span reproduces q=h.
    for direction in [Positive, Negative] {
        let sign = if direction == Positive { 1.0 } else { -1.0 };
        let at_floor = verify(&curve, input(sign * SLIP_FLOOR, k), Slide(direction));
        assert_eq!((at_floor.mode, at_floor.slip), (Stick, 0.0));
        let above = verify(
            &curve,
            input(sign * SLIP_FLOOR.next_up(), k),
            Slide(direction),
        );
        assert_eq!(above.mode, Slide(direction));
    }
}

#[test]
fn faults_carry_no_state_and_have_no_heap_cost() {
    let curve = FrictionCurve::prepare();
    let good = input(0.14, 0.1);
    let previous = Slide(Positive);
    let expected = curve.solve(good, previous).unwrap();
    let operations = crate::ffi::heap_operations(|| {
        for invalid in [f64::NAN, f64::NEG_INFINITY, f64::INFINITY] {
            for sample in [
                input(invalid, 0.0),
                input(0.0, invalid),
                ContactInput {
                    from_nut: invalid,
                    ..good
                },
                ContactInput {
                    from_bridge: invalid,
                    ..good
                },
            ] {
                assert_eq!(curve.solve(black_box(sample), previous), Err(ContactFault));
            }
        }
        for sample in [
            input(0.14, -0.01),
            input(0.0, f64::MAX), // k*mu0
            ContactInput {
                from_nut: -f64::MAX,
                ..input(f64::MAX, 0.0)
            }, // d
            ContactInput {
                from_nut: 1.7e308,
                from_bridge: -1.6e308,
                ..input(7e307, 7e307)
            }, // a+j
            input(-1e308, 1e308), // force
            ContactInput {
                from_nut: 1e308,
                from_bridge: 1e308,
                ..input(1e308, 0.0)
            }, // a+b
        ] {
            assert_eq!(curve.solve(black_box(sample), previous), Err(ContactFault));
            assert_eq!(curve.solve(black_box(good), previous), Ok(expected));
        }
        for k in [0.0, 0.03, 0.10, 0.23] {
            let mut mode = previous;
            for i in -2000..=2000 {
                mode = curve
                    .solve(black_box(input(f64::from(i) / 1000.0, k)), mode)
                    .unwrap()
                    .mode;
            }
        }
    });
    assert_eq!(operations, 0);
}
