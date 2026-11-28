use super::*;
use crate::instrument::waveguide::Guide;

fn bits(lane: &O2Lane) -> [u64; 4] {
    [lane.v, lane.h, lane.z1, lane.z2].map(f64::to_bits)
}

#[test]
fn preparation_and_transaction() {
    for fs in [
        f64::NAN,
        f64::INFINITY,
        32_000.0_f64.next_down(),
        192_000.0_f64.next_up(),
    ] {
        assert!(O2Coefficients::prepare(fs).is_err());
    }
    let c = O2Coefficients::prepare(48_000.0).unwrap();
    let mut lane = O2Lane::default();
    assert_eq!(bits(&lane), [0; 4]);
    for (v, depth) in [(-8.0, 0.0), (8.0, 1.0)] {
        lane.tick(&c, v, depth).unwrap();
    }
    for (v, d) in [
        (8.0_f64.next_up(), 1.0),
        ((-8.0_f64).next_down(), 0.0),
        (f64::NAN, 0.5),
        (f64::INFINITY, 0.5),
        (0.2, f64::NAN),
        (0.2, -0.1),
        (0.2, 1.0_f64.next_up()),
    ] {
        let old = bits(&lane);
        assert_eq!(lane.tick(&c, v, d), Err(OutputFault));
        assert_eq!(bits(&lane), old);
    }
    for index in 0..4 {
        lane.reset();
        *[&mut lane.v, &mut lane.h, &mut lane.z1, &mut lane.z2][index] = f64::NAN;
        let old = bits(&lane);
        assert_eq!(lane.tick(&c, 0.3, 0.5), Err(OutputFault));
        assert_eq!(bits(&lane), old);
    }
    lane.reset();
    lane.z1 = f64::MAX;
    let old = bits(&lane);
    assert_eq!(lane.tick(&c, 1.0, 0.5), Err(OutputFault));
    assert_eq!(bits(&lane), old, "prospective fault published state");
    for _ in 0..2 {
        lane.reset();
        assert_eq!(bits(&lane), [0; 4], "dirty reset");
    }
}

#[test]
fn isolated_epoch_heap_operations() {
    for rate in [32_000.0, 192_000.0] {
        let coeff = O2Coefficients::prepare(rate).unwrap();
        let mut lane = O2Lane::default();
        let mut guide = Guide::prepare(rate).unwrap();
        let operations = crate::ffi::heap_operations(|| {
            for _ in 0..12 {
                guide.launch(196.0, 0.1).unwrap();
                lane.reset();
                for i in 0..(5.0 * rate) as usize {
                    let k = if i < (4.0 * rate) as usize {
                        1.8 * 0.1 / 1.65
                    } else {
                        0.0
                    };
                    let (_, o) = guide
                        .tick_observed(196.0 + i as f64 / rate, 0.1, k)
                        .unwrap()
                        .unwrap();
                    std::hint::black_box(
                        lane.tick(&coeff, o.velocity(), (i % 3) as f64 * 0.5)
                            .unwrap(),
                    );
                }
                let (_, o) = guide.tick_observed(200.0, 0.1, 0.0).unwrap().unwrap();
                assert_eq!(lane.tick(&coeff, o.velocity(), f64::NAN), Err(OutputFault));
                guide.invalidate();
                lane.reset();
                assert!(guide.tick_observed(196.0, 0.0, 0.0).unwrap().is_none());
                guide.launch(196.0, 0.1).unwrap();
                lane.reset();
                assert!(guide.tick_observed(f64::NAN, 0.1, 0.0).is_err());
                lane.reset();
            }
            for i in 0..(30.0 * rate) as usize {
                let tiny = if i % 2 == 0 { 1e-310 } else { -1e-310 };
                std::hint::black_box(lane.tick(&coeff, tiny, 0.5).unwrap());
            }
        });
        assert_eq!(operations, 0);
    }
}
