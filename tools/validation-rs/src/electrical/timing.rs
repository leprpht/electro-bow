use crate::{Result, ensure, evidence, process, replay};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

// The terminal fixture stores Python float.hex(). Recover its integer significand exactly.
fn terminal(s: &str) -> Result<f64> {
    let (m, e) = s
        .trim_start_matches('-')
        .strip_prefix("0x")
        .ok_or("terminal hex prefix")?
        .split_once('p')
        .ok_or("terminal exponent")?;
    let (whole, fraction) = m.split_once('.').ok_or("terminal significand")?;
    ensure(
        fraction.len() <= 13 && matches!(whole, "0" | "1"),
        "binary64 terminal format",
    )?;
    let significand = u64::from_str_radix(&format!("{whole}{fraction:0<13}"), 16)?;
    let exponent: i32 = e.parse()?;
    let sign = if s.starts_with('-') { 1u64 << 63 } else { 0 };
    if significand == 0 {
        return Ok(f64::from_bits(sign));
    }
    if whole == "0" {
        ensure(exponent == -1022, "subnormal exponent")?;
        return Ok(f64::from_bits(sign | significand));
    }
    ensure((-1022..=1023).contains(&exponent), "normal exponent")?;
    Ok(f64::from_bits(
        sign | ((exponent + 1023) as u64) << 52 | (significand & ((1u64 << 52) - 1)),
    ))
}
pub fn run(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let binary = evidence::digest(&exe)?;
    let size = process::check(Command::new(&exe).args(["--o2-size", "48000"]))?;
    ensure(
        std::str::from_utf8(&size.stdout)?.trim() == "48 32",
        "coefficient/lane size",
    )?;
    let id = out
        .file_name()
        .ok_or("missing run id")?
        .to_str()
        .ok_or("run id encoding")?;
    let terminals: BTreeMap<String, Vec<String>> = serde_json::from_value(evidence::strict(
        include_bytes!("../../../validation/fixtures/o2_timing_terminals.json"),
    )?)?;
    let workloads = ["silence", "near-denormal", "o2", "guide-o1", "guide-o1-o2"];
    let mut rates = vec![];
    for fs in [32000, 48000, 192000] {
        let rate = fs.to_string();
        let r = process::check(Command::new(&exe).args(["--o2-timing", &rate, id]))?;
        let path = out.join(format!("timing-{fs}.stdout"));
        fs::write(&path, &r.stdout)?;
        let expected = &terminals[&rate];
        let mut records = vec![];
        let mut medians = Vec::new();
        let mut times = Vec::new();
        for (i, line) in std::str::from_utf8(&r.stdout)?.lines().enumerate() {
            let t: Vec<_> = line.split_whitespace().collect();
            ensure(
                i < 25
                    && t.len() == 7
                    && t[0] == id
                    && t[1] == rate
                    && t[2] == workloads[i / 5]
                    && t[3].parse::<usize>()? == i % 5 + 1,
                "timing invocation and batch",
            )?;
            let frames = t[4].parse::<u64>()?;
            let ns = t[5].parse::<u64>()?;
            let h = t[6].parse::<f64>()?;
            ensure(
                frames == 30 * fs && ns > 0 && h.to_bits() == terminal(&expected[i])?.to_bits(),
                "timing frame count/terminal",
            )?;
            ensure(i >= 5 || h == 0., "silence terminal")?;
            ensure(
                !(5..10).contains(&i) || h.is_subnormal(),
                "active subnormal",
            )?;
            times.push(ns as f64 / frames as f64);
            records.push(
                json!({"workload":t[2],"batch":i%5+1,"frames":frames,"elapsed_ns":ns,"terminal":h}),
            );
            if i % 5 == 4 {
                times.sort_by(f64::total_cmp);
                medians.push(times[2]);
                times.clear();
            }
        }
        ensure(
            records.len() == 25 && expected.len() == 25,
            "timing workload inventory",
        )?;
        let ratio = medians[1] / medians[0];
        ensure(ratio <= 2., "median denormal/silence bound")?;
        rates.push(json!({"rate":fs,"records":records,"near_denormal_ratio":ratio,"stdout_sha256":evidence::digest(&path)?,"argv":["--o2-timing",rate,id]}));
    }
    Ok(
        json!({"run_id":id,"executable_sha256":binary,"rates":rates,"coefficient_bytes":48,"lane_bytes":32}),
    )
}
