//! Installed as waveguide/tests.rs in each disposable comparison build.
//! advance() is bound by the builder to the entry or observed tick.
use super::*;
use std::io::{BufWriter, Write};

#[test]
fn epoch_state_trace() {
    let path = std::env::var_os("O1_STATE_TRACE").unwrap();
    let mut trace = BufWriter::new(std::fs::File::create(path).unwrap());
    for rate in [32000.0, 192000.0] {
        let mut g = Guide::prepare(rate).unwrap();
        let capacity = g.nut.storage().len();
        for n in 0..4 * capacity {
            match n {
                1 => g.launch(1320.0, 0.16).unwrap(),
                i if i == 3 * capacity => g.invalidate(),
                i if i == 3 * capacity + 10 || i == 3 * capacity + 40 => {
                    g.launch(196.0, 0.06).unwrap()
                }
                i if i == 3 * capacity + 30 => {
                    g.edit_proposal = Some(|s| s.contact.to_nut = f64::NAN)
                }
                i if i == 3 * capacity + 31 => g.edit_proposal = None,
                _ => {}
            }
            let hz = if n == 4 * capacity - 2 {
                f64::NAN
            } else if n % 401 < 200 {
                1320.0
            } else {
                65.0
            };
            let k = if g.live && n % 509 < 409 {
                1.8 * 0.1 / 1.65
            } else {
                0.0
            };
            let result = advance(&mut g, hz, k).map(|v| {
                v.map(|s| {
                    let c = s.contact;
                    (
                        [
                            s.from_nut,
                            s.from_bridge,
                            c.to_nut,
                            c.to_bridge,
                            s.x,
                            c.j,
                            c.slip,
                            c.force,
                            c.string_velocity,
                            s.pickup,
                            s.audio,
                        ]
                        .map(f64::to_bits),
                        c.mode,
                        c.segment_probes,
                    )
                })
            });
            let arms = [&g.bridge, &g.nut].map(|ring| {
                let (cursor, count, seed) = ring.epoch_state();
                let newest = (cursor + capacity - 1) % capacity;
                (
                    cursor,
                    count,
                    seed.map(|(p, a)| (p.to_bits(), a.to_bits())),
                    ring.storage()[newest].to_bits(),
                )
            });
            let scalars = (
                g.live,
                g.launch_count,
                g.fault_count,
                g.mode,
                g.loss.map(f64::to_bits),
                g.previous_pickup.to_bits(),
                g.previous_dc.to_bits(),
            );
            writeln!(trace, "{rate} {n} {result:?} {arms:?} {scalars:?}").unwrap();
        }
    }
    trace.flush().unwrap();
}
