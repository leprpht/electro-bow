mod cases;
mod oracle;
use crate::{Result, ensure, evidence};
use oracle::{Answer, Input, Mu, compare, graph, physics, ulp};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    process::{Command, Stdio},
};

pub fn run(out: &Path, reference: Option<&Path>) -> Result<Value> {
    let exe = out.join(format!("contact-replay{}", std::env::consts::EXE_SUFFIX));
    let status = Command::new("rustc")
        .args([
            "--edition",
            "2024",
            "-O",
            "-Awarnings",
            "tools/validation/contact_replay.rs",
            "-o",
        ])
        .arg(&exe)
        .status()?;
    ensure(status.success(), "compile contact replay")?;
    let prep = Command::new(&exe).arg("--coefficients").output()?;
    ensure(prep.status.success(), "prepare contact coefficients")?;
    let coeff: Vec<_> = std::str::from_utf8(&prep.stdout)?
        .split_whitespace()
        .map(|word| u64::from_str_radix(word, 16).map(f64::from_bits))
        .collect::<std::result::Result<_, _>>()?;
    let mu: Mu = coeff.try_into().map_err(|_| "expected ten coefficients")?;
    let bounds: [[f64; 2]; 10] =
        serde_json::from_str(include_str!("../../reference/mu-bounds.json"))?;
    for (m, [lo, hi]) in mu.iter().zip(bounds) {
        ensure((lo..=hi).contains(m), "80-decimal-digit exponential bound")?;
    }
    ensure(mu[0] == 1.65, "static coefficient")?;
    let cases = cases::generate(&mu);
    ensure(cases.len() == 84886, "complete contact inventory")?;
    let input = out.join("contact-input.txt");
    let mut stream = BufWriter::new(fs::File::create(&input)?);
    for (d, k, p) in &cases {
        writeln!(stream, "{d:?} {k:?} {p}")?;
    }
    stream.flush()?;
    let actual = out.join("actual.txt");
    let status = Command::new(&exe)
        .stdin(fs::File::open(input)?)
        .stdout(Stdio::from(fs::File::create(&actual)?))
        .status()?;
    ensure(status.success(), "execute contact replay")?;
    let mut records = BufReader::new(fs::File::open(actual)?).lines();
    let path = out.join("contact.f64le");
    let mut oracle_rows = BufWriter::new(fs::File::create(&path)?);
    let mut max_probes = 0;
    let mut max_ulps = 0.0f64;
    for (index, input) in cases.into_iter().enumerate() {
        let expected = graph(input, &mu).ok_or("missing finite intersection")?;
        let row = records.next().ok_or("missing contact replay row")??;
        let parts: Vec<_> = row.split_whitespace().collect();
        ensure(parts.len() == 4, "contact replay columns")?;
        let value = (parts[0].parse()?, parts[1].parse()?, parts[2].parse()?);
        let probes: u8 = parts[3].parse()?;
        let grade = || -> Result<()> {
            physics(input, expected, &mu, true)?;
            physics(input, value, &mu, false)?;
            compare(input.0, value, expected)?;
            ensure(
                probes <= 9 && 2. * value.0 * value.1 >= 0.,
                "probe count/dissipation",
            )
        };
        grade().map_err(|e| format!("contact row {index} {input:?} {e}"))?;
        max_probes = max_probes.max(probes);
        max_ulps = max_ulps.max((value.1 - expected.1).abs() / ulp(expected.1));
        write_record(&mut oracle_rows, input, expected)?;
    }
    ensure(
        records.next().is_none() && max_probes == 9,
        "final contact inventory",
    )?;
    oracle_rows.flush()?;
    let digest = evidence::digest(&path)?;
    if let Some(reference) = reference {
        let old = reference.join("contact.f64le");
        ensure(
            evidence::digest(&old)?
                == "e7076afaf470103a9b40c66547a1ed4f114b035fe9abbd89f8d2545e7be125b7",
            "entry corpus digest",
        )?;
        let (a, b) = (fs::read(old)?, fs::read(&path)?);
        if a != b {
            let first = a
                .iter()
                .zip(&b)
                .position(|(x, y)| x != y)
                .unwrap_or(a.len().min(b.len()));
            return Err(format!(
                "contact corpus first difference row {} byte {}",
                first / 34,
                first % 34
            )
            .into());
        }
    }
    for (d, k) in [
        (f64::NAN, 0.),
        (f64::INFINITY, 0.1),
        (0., f64::NAN),
        (0., -0.1),
        (0.14, f64::MAX),
        (-1e308, 1e308),
    ] {
        ensure(graph((d, k, 0), &mu).is_none(), "invalid contact graph")?;
    }
    Ok(
        json!({"rows":84886,"max_probes":max_probes,"max_slip_ulps":max_ulps,"reference_compared":reference.is_some(),"oracle_sha256":digest,"producer_sha256":evidence::digest(&exe)?}),
    )
}
fn write_record(w: &mut impl Write, (d, k, p): Input, (j, r, m): Answer) -> Result<()> {
    w.write_all(&d.to_le_bytes())?;
    w.write_all(&k.to_le_bytes())?;
    w.write_all(&p.to_le_bytes())?;
    w.write_all(&j.to_le_bytes())?;
    w.write_all(&r.to_le_bytes())?;
    w.write_all(&m.to_le_bytes())?;
    Ok(())
}
