//! Offline text adapter for differential tests of the actual instrument module.
#[path = "../../rust/dsp/src/instrument/friction.rs"]
mod friction;

use friction::{ContactInput, Direction, FrictionCurve, Mode};
use std::io::{self, BufRead, BufWriter, Write};

fn main() -> io::Result<()> {
    let curve = FrictionCurve::prepare();
    let mut out = BufWriter::new(io::stdout().lock());
    if std::env::args().nth(1).as_deref() == Some("--coefficients") {
        for mu in curve.prepared_coefficients() {
            writeln!(out, "{:016x}", mu.to_bits())?;
        }
        return out.flush();
    }
    for line in io::stdin().lock().lines() {
        let line = line?;
        let mut fields = line.split_whitespace();
        let d = fields.next().unwrap().parse().unwrap();
        let k = fields.next().unwrap().parse().unwrap();
        let previous = match fields.next().unwrap() {
            "-1" => Mode::Slide(Direction::Negative),
            "0" => Mode::Stick,
            "1" => Mode::Slide(Direction::Positive),
            _ => panic!("invalid fixture mode"),
        };
        // This encoding presents the requested d exactly, even one ULP from a fold.
        let input = ContactInput {
            bow_speed: d,
            k,
            from_bridge: 0.0,
            from_nut: 0.0,
        };
        match curve.solve(input, previous) {
            Err(_) => writeln!(out, "Fault")?,
            Ok(result) => {
                let sign = match result.mode {
                    Mode::Stick => 0,
                    Mode::Slide(Direction::Positive) => 1,
                    Mode::Slide(Direction::Negative) => -1,
                };
                writeln!(
                    out,
                    "{} {} {sign} {}",
                    result.j, result.slip, result.segment_probes
                )?;
            }
        }
    }
    out.flush()
}
