//! Prescribed events in absolute sample time. No articulation decisions.
use super::*;

const END: usize = 1536;
const EDGES: [usize; 15] = [
    0, 2, 193, 320, 447, 512, 555, 600, 700, 701, 800, 923, 1100, 1300, END,
];

#[derive(Debug, PartialEq)]
struct Record {
    time: usize,
    kind: u8,
    delivered: Option<u64>,
    m5: Option<u64>,
    state: [u64; 4],
    commits: usize,
    calls: usize,
    epoch: (usize, usize, usize),
}

fn block(h: &mut Harness, rows: &mut Vec<Record>, frames: usize) {
    for _ in 0..frames {
        let n = rows.len();
        if [2, 600, 800, 1100, 1300].contains(&n) {
            h.launch(196.0, 0.1);
            assert_eq!(h.lane.state().map(f64::to_bits), [0; 4], "launch state");
        }
        if n == 555 {
            h.dispose();
        }
        // 193..320 retunes continuously. Compatible continuation at 320 keeps
        // the same epoch. Lift at 447; harness-only release interval 512..555.
        let hz = if n == 700 {
            f64::NAN
        } else {
            196.0 + n.saturating_sub(193).min(127) as f64 * 0.015
        };
        let k = if (2..447).contains(&n)
            || (600..700).contains(&n)
            || (800..=923).contains(&n)
            || n >= 1100
        {
            1.8 * 0.1 / 1.65
        } else {
            0.0
        };
        let depth = if n == 923 {
            f64::NAN
        } else if n < 193 {
            0.0
        } else if n < 320 {
            1.0
        } else {
            0.5
        };
        let before = (h.guide.commits, h.calls, h.guide.lifecycle());
        let (kind, q, m5) = match h.tick([hz, 0.1, k], depth) {
            Frame::Live(s, o, q) => {
                assert_eq!(
                    h.lane.state()[0].to_bits(),
                    o.velocity().to_bits(),
                    "same-tick velocity"
                );
                (1, Some(q), Some(s.audio))
            }
            Frame::Idle(q) => {
                assert_eq!(q.to_bits(), 0);
                (0, Some(q), None)
            }
            Frame::GuideFault => (2, None, None),
            Frame::OutputFault(s) => (3, None, Some(s.audio)),
        };
        let expected = match n {
            0..=1 | 555..=599 | 701..=799 | 924..=1099 => 0,
            700 => 2,
            923 => 3,
            _ => 1,
        };
        assert_eq!(kind, expected);
        let advanced = usize::from(matches!(kind, 1 | 3));
        assert_eq!(
            h.guide.commits,
            before.0 + advanced,
            "physical commit count"
        );
        assert_eq!(h.calls, before.1 + advanced, "O2 calls");
        let (launch, faults, clear) = before.2;
        assert_eq!(
            h.guide.lifecycle(),
            (
                launch,
                faults + usize::from(kind == 2),
                clear + usize::from(kind >= 2)
            ),
            "fault invalidation count"
        );
        if kind != 1 {
            assert_eq!(h.lane.state().map(f64::to_bits), [0; 4], "dirty reset");
        }
        rows.push(Record {
            time: n,
            kind,
            delivered: q.map(f64::to_bits),
            m5: m5.map(f64::to_bits),
            state: h.lane.state().map(f64::to_bits),
            commits: h.guide.commits,
            calls: h.calls,
            epoch: h.guide.lifecycle(),
        });
    }
}

pub(super) fn check(fs: f64) {
    let mut reference = Vec::with_capacity(END);
    let mut h = Harness::new(fs);
    for _ in 0..END {
        block(&mut h, &mut reference, 1);
    }
    for schedule in ["fixed", "random", "events"] {
        let mut h = Harness::new(fs);
        let mut rows = Vec::with_capacity(END);
        let mut seed = 603u32;
        while rows.len() < END {
            let n = rows.len();
            let before = (
                h.guide.lifecycle(),
                h.guide.commits,
                h.calls,
                h.lane.state(),
                n,
            );
            block(&mut h, &mut rows, 0);
            assert_eq!(
                before,
                (
                    h.guide.lifecycle(),
                    h.guide.commits,
                    h.calls,
                    h.lane.state(),
                    rows.len()
                )
            );
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let size = match schedule {
                "fixed" => 127,
                "random" => 1 + (seed % 91) as usize,
                _ => EDGES.iter().find(|&&edge| edge > n).unwrap() - n,
            };
            block(&mut h, &mut rows, size.min(END - n));
        }
        assert_eq!(rows, reference, "partition {schedule}");
        println!("partition-{schedule} {END}");
    }
    for (name, n) in [
        ("cleared-idle", 0),
        ("launch", 2),
        ("sustain", 150),
        ("retune", 193),
        ("continuation", 320),
        ("physical-lift", 447),
        ("harness-release", 512),
        ("disposal", 555),
        ("physical-fault", 700),
        ("post-commit-fault", 923),
        ("fresh-epoch", 1100),
        ("dirty-relaunch", 1300),
    ] {
        let row = &reference[n];
        println!(
            "{name} {n} {} {} {} {} {} {}",
            row.kind, row.commits, row.calls, row.epoch.0, row.epoch.1, row.epoch.2
        );
    }
    crate::o2::verify_faults(fs);
}
