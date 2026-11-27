//! Standalone validation adapter. Binary streams are little-endian f64.
use super::o2::{O2Coefficients, O2Lane};
use super::waveguide::{Guide, Observation, Sample};
use std::io::{self, BufRead, Read, Write};
use std::{hint::black_box, time::Instant};

#[path = "o2_lifecycle.rs"]
mod lifecycle;

enum Frame {
    Live(Sample, Observation, f64),
    Idle(f64),
    GuideFault,
    OutputFault(Sample),
}

struct Harness {
    guide: Guide,
    lane: O2Lane,
    coeff: O2Coefficients,
    calls: usize,
}

impl Harness {
    fn new(fs: f64) -> Self {
        Self {
            guide: Guide::prepare(fs).unwrap(),
            lane: O2Lane::default(),
            coeff: O2Coefficients::prepare(fs).unwrap(),
            calls: 0,
        }
    }

    fn launch(&mut self, hz: f64, speed: f64) {
        self.guide.launch(hz, speed).unwrap();
        self.lane.reset();
    }

    fn dispose(&mut self) {
        self.guide.invalidate();
        self.lane.reset();
    }

    fn tick(&mut self, [hz, speed, k]: [f64; 3], detail: f64) -> Frame {
        match self.guide.tick_observed(hz, speed, k) {
            Ok(Some((s, o))) => {
                self.calls += 1;
                match self.lane.tick(&self.coeff, o.velocity(), detail) {
                    Ok(q) => Frame::Live(s, o, q),
                    Err(_) => {
                        // Both guide arms already committed. Preserve the physical sample
                        // as evidence and invalidate once without a replacement tick.
                        self.dispose();
                        Frame::OutputFault(s)
                    }
                }
            }
            Ok(None) => Frame::Idle(0.0),
            Err(_) => {
                self.lane.reset(); // Guide already invalidated its own epoch.
                Frame::GuideFault
            }
        }
    }
}

fn read_frame<const N: usize>(input: &mut impl Read) -> io::Result<[f64; N]> {
    let mut frame = [0.0; N];
    for value in &mut frame {
        let mut bytes = [0; 8];
        input.read_exact(&mut bytes)?;
        *value = f64::from_le_bytes(bytes);
    }
    Ok(frame)
}

fn stream(fs: f64, observed: bool) -> io::Result<()> {
    let mut input = io::BufReader::new(io::stdin().lock());
    let mut out = io::BufWriter::new(io::stdout().lock());
    let mut harness = Harness::new(fs);
    let mut factory = O2Lane::default();
    let mut factory_return = O2Lane::default();
    let mut full = O2Lane::default();
    let mut seed = (0.0, 0.0);
    let mut n = 0;
    while !input.fill_buf()?.is_empty() {
        if observed {
            let controls = read_frame(&mut input)?;
            if n == 0 {
                harness.launch(controls[0], controls[1]);
                factory.reset();
                full.reset();
                seed = (fs / controls[0], controls[1] / super::ring::BETA);
            }
            let Frame::Live(s, o, off) = harness.tick(controls, 0.0) else {
                panic!("guide/O2 fault at {n}");
            };
            harness.guide.check_epoch(n + 1, seed);
            super::write_sample(&mut out, s, Some(o))?;
            let v = o.velocity();
            let [h, a, l, tap] = factory.taps(&harness.coeff, v, 0.5);
            let mid = factory_return.tick(&harness.coeff, v, 0.5).unwrap();
            assert_eq!(mid.to_bits(), tap.to_bits(), "factory return at {n}");
            let high = full.tick(&harness.coeff, v, 1.0).unwrap();
            let [_, _, z1, z2] = factory.state();
            for x in [h, a, l, off, mid, high, z1, z2] {
                out.write_all(&x.to_le_bytes())?;
            }
        } else {
            let [v, d] = read_frame(&mut input)?;
            let taps = factory.taps(&harness.coeff, v, d);
            let q = full.tick(&harness.coeff, v, d).unwrap();
            assert_eq!(q.to_bits(), taps[3].to_bits(), "return tap at {n}");
            for x in taps.into_iter().chain(factory.state()) {
                out.write_all(&x.to_le_bytes())?;
            }
        }
        n += 1;
    }
    if observed {
        let (launches, faults, clears) = harness.guide.lifecycle();
        eprintln!(
            "{n} {launches} {faults} {clears} {} {}",
            harness.guide.commits, harness.calls
        );
    }
    out.flush()
}

fn long_step(fs: f64, depth: f64) {
    let coeff = O2Coefficients::prepare(fs).unwrap();
    let mut lane = O2Lane::default();
    let mut tail = 0.0_f64;
    let mut peak = 0.0_f64;
    let n = (60.0 * fs).ceil() as usize;
    let mut first = 0.0;
    for _ in 0..1024 {
        assert_eq!(lane.tick(&coeff, 0.0, depth).unwrap().to_bits(), 0);
    }
    for i in 0..n {
        let q = lane.tick(&coeff, black_box(1.0), depth).unwrap();
        if i == 0 {
            first = q;
        }
        peak = peak.max(q.abs());
        if i >= fs.ceil() as usize {
            tail = tail.max(q.abs());
        }
    }
    let state = lane.state();
    assert!(state[0] == 1.0 && tail < 1e-10 && state[1..].iter().all(|v| v.abs() < 1e-10));
    lane.reset();
    assert_eq!(lane.state().map(f64::to_bits), [0; 4], "dirty reset");
    assert_eq!(lane.tick(&coeff, 1.0, depth).unwrap(), first);
    println!(
        "{n} {first} {peak} {tail} {} {} {} {}",
        state[0], state[1], state[2], state[3]
    );
}

fn timing(fs: f64) {
    for workload in ["silence", "near-denormal", "o2", "guide-o1", "guide-o1-o2"] {
        let mut h = Harness::new(fs);
        h.launch(196.0, 0.1);
        for batch in 0..6 {
            let frames = fs as usize * if batch == 0 { 1 } else { 30 };
            let start = Instant::now();
            for n in 0..frames {
                let sign = if n & 1 == 0 { 1.0 } else { -1.0 };
                let d = (n % 3) as f64 * 0.5;
                let v = match workload {
                    "guide-o1" | "guide-o1-o2" => {
                        let (s, o) = h
                            .guide
                            .tick_observed(black_box(196.0), 0.1, 1.8 * 0.1 / 1.65)
                            .unwrap()
                            .unwrap();
                        black_box(s);
                        black_box(o.velocity())
                    }
                    "silence" => 0.0,
                    "near-denormal" => sign * 1e-310,
                    _ => sign * 0.1,
                };
                if workload != "guide-o1" {
                    black_box(h.lane.tick(&h.coeff, black_box(v), black_box(d)).unwrap());
                }
            }
            let ns = start.elapsed().as_secs_f64() * 1e9 / frames as f64;
            if batch > 0 {
                println!("{workload} {batch} {ns:.4} {:e}", h.lane.state()[1]);
            }
        }
    }
}

pub(super) fn run(mode: &str) -> io::Result<()> {
    let fs = std::env::args().nth(2).unwrap().parse().unwrap();
    match mode {
        "--o2-guide" => return stream(fs, true),
        "--o2-taps" => return stream(fs, false),
        "--o2-coeff" => println!("{:?}", O2Coefficients::prepare(fs).unwrap().values()),
        "--o2-step" => long_step(fs, std::env::args().nth(3).unwrap().parse().unwrap()),
        "--o2-lifecycle" => lifecycle::check(fs),
        "--o2-timing" => timing(fs),
        "--o2-size" => println!("{} {}", size_of::<O2Coefficients>(), size_of::<O2Lane>()),
        _ => panic!("unknown replay mode"),
    }
    Ok(())
}
