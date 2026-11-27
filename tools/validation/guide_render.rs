//! Offline binary adapter around the same private Rust sources as the instrument.
//! stdin is little-endian binary64 (Hz,V,k) frames, stdout is binary64 telemetry.
#![forbid(unsafe_code)]
#[allow(dead_code)]
#[path = "../../rust/dsp/src/instrument/friction.rs"]
mod friction;
#[path = "../../rust/dsp/src/instrument/ring.rs"]
mod ring;
#[path = "../../rust/dsp/src/instrument/waveguide.rs"]
mod waveguide;

use friction::{Direction, Mode};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

fn render(rate: f64, observed: bool, input: impl Read, output: impl Write) -> io::Result<()> {
    let mut guide = waveguide::Guide::prepare(rate).expect("supported rate");
    let mut input = BufReader::new(input);
    let mut output = BufWriter::new(output);
    let mut tick = 0;
    #[cfg(epoch_audit)]
    let mut seed = (0.0, 0.0);
    while !input.fill_buf()?.is_empty() {
        let mut controls = [0.0; 3];
        for value in &mut controls {
            let mut word = [0; 8];
            input.read_exact(&mut word)?;
            *value = f64::from_le_bytes(word);
        }
        let [hz, speed, k] = controls;
        if tick == 0 {
            guide.launch(hz, speed).expect("valid launch");
            #[cfg(epoch_audit)]
            {
                seed = (rate / hz, speed / ring::BETA);
                guide.check_epoch(0, seed);
            }
        }
        let result = if observed {
            guide
                .tick_observed(hz, speed, k)
                .map(|v| v.map(|(s, o)| (s, Some(o))))
        } else {
            guide.tick(hz, speed, k).map(|v| v.map(|s| (s, None)))
        };
        let (sample, observation) = result
            .unwrap_or_else(|_| panic!("guide fault on tick {tick} at {controls:?}"))
            .expect("live epoch");
        #[cfg(epoch_audit)]
        guide.check_epoch(tick + 1, seed);
        let c = sample.contact;
        let branch = match c.mode {
            Mode::Stick => 0.0,
            Mode::Slide(Direction::Positive) => 1.0,
            Mode::Slide(Direction::Negative) => -1.0,
        };
        for value in [
            sample.from_nut,
            sample.from_bridge,
            c.to_bridge,
            c.to_nut,
            sample.x,
            sample.pickup,
            c.j,
            c.slip,
            c.force,
            c.string_velocity,
            branch,
            sample.audio,
        ] {
            output.write_all(&value.to_le_bytes())?;
        }
        if let Some(o) = observation {
            for station in o.stations {
                for value in [
                    station.direct,
                    station.reflected_history,
                    station.velocity(),
                ] {
                    output.write_all(&value.to_le_bytes())?;
                }
            }
            output.write_all(&o.velocity().to_le_bytes())?;
        }
        tick += 1;
    }
    output.flush()?;
    #[cfg(epoch_audit)]
    {
        let (launches, faults, invalidations) = guide.lifecycle();
        eprintln!("{tick} {launches} {faults} {invalidations}");
    }
    #[cfg(not(epoch_audit))]
    eprintln!("{tick} ticks (standalone render without lifecycle audit)");
    Ok(())
}

fn benchmark(observed: bool) {
    use std::{hint::black_box, time::Instant};
    for rate in [32000_u32, 48000, 192000] {
        let mut guide = waveguide::Guide::prepare(f64::from(rate)).unwrap();
        let capacity = (f64::from(rate) / 65.0).ceil() as usize + 4;
        let bytes = size_of::<waveguide::Guide>() + 2 * capacity * 8;
        guide.launch(440.0, 0.1).unwrap();
        let k = 1.8 * 0.1 / 1.65;
        let tick = |guide: &mut waveguide::Guide, hz| {
            if observed {
                let (s, o) = guide.tick_observed(hz, 0.1, k).unwrap().unwrap();
                black_box((s, o.velocity()));
            } else {
                black_box(guide.tick(hz, 0.1, k).unwrap().unwrap());
            }
        };
        for _ in 0..rate {
            tick(&mut guide, 440.0);
        }
        for _ in 0..5 {
            let start = Instant::now();
            for i in 0..10 * rate {
                if i % rate == 0 {
                    guide.launch(440.0, 0.1).unwrap();
                }
                let hz = if i % 4096 < 2048 { 196.0 } else { 440.0 };
                tick(&mut guide, black_box(hz));
            }
            println!(
                "rate {rate} bytes {bytes} ns/tick {:.3}",
                start.elapsed().as_secs_f64() * 1e9 / f64::from(10 * rate)
            );
        }
    }
}

/// Ten minutes of prescribed per-lane work. Xorshift32 chooses the next
/// physical playing point. The per-sample Hz ramp is bounded at 4 octaves/s.
fn stress() {
    let rate = 48000;
    let mut guide = waveguide::Guide::prepare(f64::from(rate)).unwrap();
    let mut state = 1_u32;
    let mut uniform = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        f64::from(state) / f64::from(u32::MAX)
    };
    let mut peaks = [0.0_f64; 10];
    for note in 0..600 {
        let start = 65.0 + 1255.0 * uniform();
        let target = 65.0 + 1255.0 * uniform();
        let speed = [0.06, 0.16, 0.06 + 0.10 * uniform()][note % 3];
        let ratio = [1.2, 2.4, 1.2 + 1.2 * uniform()][note % 3];
        let multiplier =
            ((target / start).log2().clamp(-2.0, 2.0) / (0.5 * f64::from(rate))).exp2();
        guide.launch(start, speed).unwrap();
        let mut hz = start;
        for n in 0..rate {
            if n >= rate / 4 && n < 3 * rate / 4 {
                hz = (hz * multiplier).clamp(start.min(target), start.max(target));
            }
            let k = if n >= 7 * rate / 8 {
                0.0
            } else {
                ratio * speed / 1.65
            };
            let s = guide.tick(hz, speed, k).unwrap().unwrap();
            let c = s.contact;
            for (peak, value) in peaks.iter_mut().zip([
                s.from_nut,
                s.from_bridge,
                c.to_bridge,
                c.to_nut,
                s.pickup,
                s.x,
                c.j,
                c.slip,
                c.force,
                c.string_velocity,
            ]) {
                *peak = peak.max(value.abs());
            }
        }
        guide.invalidate();
    }
    #[cfg(epoch_audit)]
    {
        let (launches, faults, invalidations) = guide.lifecycle();
        assert_eq!((launches, faults, invalidations), (600, 0, 1200));
        println!("launches {launches} faults {faults} invalidations {invalidations}");
    }
    println!("ticks {} peaks a b B N p x j r F v {peaks:?}", 600 * rate);
}

fn main() -> io::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--stress") {
        stress();
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("--benchmark") {
        benchmark(std::env::args().nth(2).as_deref() == Some("--o1"));
        return Ok(());
    }
    let rate = std::env::args()
        .nth(1)
        .expect("guide_render SAMPLE_RATE")
        .parse()
        .unwrap();
    render(
        rate,
        std::env::args().nth(2).as_deref() == Some("--o1"),
        io::stdin().lock(),
        io::stdout().lock(),
    )
}
