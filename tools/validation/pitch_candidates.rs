// Offline comparison on externally supplied direct-sum differences. No plugin linkage.
// Protocol: 3 little-endian u32s (frame count, lowest candidate, largest lag),
// then count rows of K+1 little-endian f64s. Each output row has three (start Hz/Fa,
// follow Hz/Fa, Q) triples. Zero frequency means rejected. Range is checked by caller.
use std::io::{self, Read, Write};

fn offset(left: f64, mid: f64, right: f64) -> f64 {
    let curvature = left - 2.0 * mid + right;
    if curvature > 1e-12 {
        (0.5 * (left - right) / curvature).clamp(-0.5, 0.5)
    } else {
        0.0
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Selection {
    IntegerFirst,
    FittedFirst,
    FittedDeepest,
}

fn choose(d: &[f64], cm: &[f64], low: usize, rule: Selection) -> [f64; 3] {
    let mut best = None;
    for t in low..cm.len() - 1 {
        let (a, b, c) = (cm[t - 1], cm[t], cm[t + 1]);
        if b > a || b >= c {
            continue;
        }
        let u = if rule != Selection::IntegerFirst {
            offset(a, b, c)
        } else {
            0.0
        };
        let depth = (b + 0.5 * u * ((c - a) + u * (a - 2.0 * b + c))).clamp(0.0, 1.0);
        if best.is_none_or(|(_, old)| depth < old) {
            best = Some((t, depth));
        }
        if depth < 0.15 && rule != Selection::FittedDeepest {
            break;
        }
    }
    if let Some((t, depth)) = best {
        if depth <= 0.26 {
            let u = offset(d[t - 1], d[t], d[t + 1]);
            let f = 1.0 / (t as f64 + u);
            return [if depth < 0.15 { f } else { 0.0 }, f, 1.0 - depth];
        }
    }
    [0.0; 3]
}

fn main() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::BufWriter::new(io::stdout().lock());
    loop {
        let mut header = [0u8; 12];
        if input.read(&mut header[..1])? == 0 {
            return Ok(());
        }
        input.read_exact(&mut header[1..])?;
        let word = |i| u32::from_le_bytes(header[i..i + 4].try_into().unwrap()) as usize;
        let (rows, low, k) = (word(0), word(4), word(8));
        let mut bytes = vec![0; rows * (k + 1) * 8];
        input.read_exact(&mut bytes)?;
        for row in bytes.chunks_exact((k + 1) * 8) {
            let d: Vec<_> = row
                .chunks_exact(8)
                .map(|b| f64::from_le_bytes(b.try_into().unwrap()).max(0.0))
                .collect();
            let mut sum = 0.0;
            let mut cm = vec![1.0; k + 1];
            for lag in 1..=k {
                sum += d[lag];
                if sum > 1e-12 {
                    cm[lag] = lag as f64 * d[lag] / sum;
                }
            }
            for rule in [
                Selection::IntegerFirst,
                Selection::FittedFirst,
                Selection::FittedDeepest,
            ] {
                for v in choose(&d, &cm, low, rule) {
                    output.write_all(&v.to_le_bytes())?;
                }
            }
        }
        output.flush()?;
    }
}
