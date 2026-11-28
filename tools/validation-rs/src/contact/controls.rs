use super::oracle::{Answer, Mu, Q, compare, graph, intersections, physics};
use crate::{Result, ensure, evidence, process, replay};
use num_rational::BigRational;
use num_traits::ToPrimitive;
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

pub fn ancillary(mu: &Mu) -> Result<Value> {
    let k = (Q[4] - Q[3] - 5e-13) / (mu[3] - mu[4]);
    let (left, right) = (Q[3] + k * mu[3], Q[4] + k * mu[4]);
    let d = (left + right) / 2.;
    let stored = graph((d, k, 1), mu).ok_or("stored fold")?;
    let exact = |v| BigRational::from_float(v).unwrap();
    let a = exact(Q[3]) + exact(k) * exact(mu[3]);
    let b = exact(Q[4]) + exact(k) * exact(mu[4]);
    let alternate = exact(Q[3]) + (exact(Q[4]) - exact(Q[3])) * (exact(d) - &a) / (b - a);
    ensure(
        stored.2 == 1 && (Q[3]..Q[4]).contains(&stored.1),
        "near-fold stored branch",
    )?;
    let heights: Vec<_> = Q.iter().zip(mu).map(|(q, m)| q + k * m).collect();
    ensure(
        d < heights[9]
            && d <= heights[0]
            && !heights
                .windows(2)
                .any(|p| p[0] <= d && d <= p[1] && p[1] - p[0] > 1e-12),
        "review fold cutoff precondition",
    )?;
    let lower = intersections(0.14, 0.1, mu)
        .iter()
        .map(|r| r.0)
        .min_by(f64::total_cmp)
        .ok_or("lower root")?;
    for (d, k, wrong) in [
        (d, k, (d, 0., 0)),
        (0.14, 0.1, (0.14, 0., 0)),
        (0.14, 0.1, (0.14 - lower, lower, 1)),
    ] {
        physics((d, k, 1), wrong, mu, false)?;
        ensure(
            compare(d, wrong, graph((d, k, 1), mu).ok_or("branch")?).is_err(),
            "negative branch control",
        )?;
    }
    let mut vector_results = vec![];
    for (input, expected) in [
        ((0.14, 0.1, 0), (0.14, 0., 0)),
        (
            (0.14, 0.1, 1),
            (0.05447776066490598, 0.08552223933509404, 1),
        ),
        ((-0.14, 0.1, 1), (-0.14, 0., 0)),
        (
            (-0.2, 0.1, 1),
            (-0.044747854541221044, -0.15525214545877897, -1),
        ),
        ((0.14, 0., 1), (0., 0.14, 0)),
    ] {
        let r = graph(input, mu).ok_or("named vector")?;
        ensure(
            (r.0 - expected.0).abs() < 1e-10
                && (r.1 - expected.1).abs() < 1e-10
                && r.2 == expected.2,
            "named contact vector",
        )?;
        vector_results.push(json!({"input":input,"actual":r}));
    }
    let mut fades = vec![];
    for (rate, frames) in [
        (32000, 160),
        (44100, 221),
        (44150, 221),
        (48000, 240),
        (192000, 960),
    ] {
        ensure(
            (0.005 * f64::from(rate) + 0.5).floor() as i32 == frames,
            "M6 fade count",
        )?;
        let ramp: Vec<_> = (0..frames)
            .map(|n| f64::from(frames - n - 1) / f64::from(frames))
            .collect();
        ensure(
            ramp[0] > 0.
                && ramp[0] < 1.
                && ramp.last() == Some(&0.)
                && ramp.windows(2).all(|w| w[1] < w[0]),
            "M6 fade countdown",
        )?;
        fades.push(json!({"rate":rate,"frames":frames,"first":ramp[0],"zero_tick":frames-1}));
    }
    let edge = 1200. / f64::MAX;
    ensure(
        !(1200. / 1e-307f64).is_finite()
            && !(1200. / edge.next_down()).is_finite()
            && (1200. / edge).is_finite()
            && (1200. / edge.next_up()).is_finite(),
        "tail quotient overflow",
    )?;
    Ok(
        json!({"counterexample":{"input":[d,k],"stored":stored,"unrounded_root":alternate.to_f64()},"branch_controls":3,"vectors":vector_results,"fade":fades,"tail_edge_bits":format!("{:016x}",edge.to_bits())}),
    )
}
pub fn mutants(out: &Path) -> Result<Value> {
    let source = fs::read_to_string("rust/dsp/src/instrument/friction.rs")?;
    let start = source
        .find("        for i in (0..last).rev() {")
        .ok_or("descending loop")?;
    let end = start
        + source[start..]
            .find("        None\n")
            .ok_or("end of loop")?;
    let ascending = "        for i in 0..last {\n            *probes += 1;\n            let left = Q[i] + k * self.mu[i];\n            let right = Q[i + 1] + k * self.mu[i + 1];\n            if left.min(right) <= h && h <= left.max(right) && left != right {\n                let fraction = (h - left) / (right - left);\n                return Some(Q[i] + (Q[i + 1] - Q[i]) * fraction);\n            }\n        }\n";
    let mut rows = vec![];
    for (name, before, after) in [
        (
            "cutoff",
            "if left <= h && h <= right {",
            "if left <= h && h <= right && right-left > 1e-12 {",
        ),
        (
            "static",
            "previous != Mode::Slide(direction) && h <= limit",
            "h <= limit",
        ),
        ("lower", &source[start..end], ascending),
    ] {
        let tmp = tempfile::tempdir_in(out)?;
        let mut candidate = source.clone();
        replay::edit(&mut candidate, before, after)?;
        fs::write(tmp.path().join("friction.rs"), candidate)?;
        let adapter = fs::read_to_string("tools/validation/contact_replay.rs")?
            .replace("../../rust/dsp/src/instrument/friction.rs", "friction.rs");
        let driver = tmp.path().join("driver.rs");
        fs::write(&driver, adapter)?;
        let exe = tmp
            .path()
            .join(format!("defect{}", std::env::consts::EXE_SUFFIX));
        process::check(
            Command::new("rustc")
                .args(["--edition", "2024", "-O", "-Awarnings"])
                .arg(driver)
                .arg("-o")
                .arg(&exe),
        )?;
        let mu = super::coefficients(&exe)?;
        let (d, k) = if name == "cutoff" {
            let k = (Q[4] - Q[3] - 5e-13) / (mu[3] - mu[4]);
            (((Q[3] + k * mu[3]) + (Q[4] + k * mu[4])) / 2., k)
        } else {
            (0.14, 0.1)
        };
        let r = replay::stream(&exe, &[], format!("{d:?} {k:?} 1\n").as_bytes())?;
        let words: Vec<_> = std::str::from_utf8(&r.stdout)?.split_whitespace().collect();
        ensure(words.len() == 4, "mutant contact response")?;
        let actual: Answer = (words[0].parse()?, words[1].parse()?, words[2].parse()?);
        let expected = graph((d, k, 1), &mu).ok_or("missing branch")?;
        physics((d, k, 1), actual, &mu, false)?;
        let why = compare(d, actual, expected)
            .err()
            .ok_or("escaped branch mutant")?
            .to_string();
        ensure(
            why == if name == "lower" {
                "slip ULP"
            } else {
                "branch mode"
            },
            "wrong contact mutant failure",
        )?;
        rows.push(json!({"name":name,"input":[d,k],"actual":actual,"expected":expected,"first_failure":why,"producer_sha256":evidence::digest(&exe)?}));
    }
    Ok(rows.into())
}
