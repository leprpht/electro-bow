//! Aggregate only receipts generated as children of this invocation.
use crate::{Result, ensure, evidence, publication};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};
const REGRESSION: &[&str] = &[
    "fixtures",
    "recording-controls",
    "contact",
    "contact-mutants",
    "ring",
    "guide",
    "guide-negative",
    "research-observation",
    "mutations",
    "o1-replay",
    "o1-state",
    "o2-bench",
    "o2-acoustic",
    "o2-sweep",
    "o2-records",
    "o2-carrier",
    "o2-mutants",
    "i1-stationary",
    "i1-motion",
    "i1-qr",
    "i1-path-mutants",
    "i1-layout",
    "i1-clock",
    "i1-events",
    "i1-transaction-mutants",
    "i1-stereo",
    "i1-sources",
    "i1-listening",
    "i1-capture",
    "i1-review82",
    "i1-evidence",
    "i1-retirement",
    "i1-joined-motion",
    "i1-uneven",
    "i1-faults",
    "i1-study",
    "i1-integrity",
    "i1-artifacts",
    "pitch",
    "native",
    "package",
    "o2-timing",
];
const COMPARISONS: &[&str] = &[
    "compare",
    "guide-compare",
    "research-observation-compare",
    "i1-clock-compare",
    "i1-study-compare",
    "i1-stream-compare",
    "i1-motion-compare",
    "i1-stationary-compare",
    "o1-compare",
    "o2-acoustic-compare",
    "o2-electrical-compare",
    "signed-compare",
    "pitch-compare",
    "i1-bank-compare",
];
pub fn run(
    dir: &Path,
    reference: &Path,
    complete: bool,
    historical: bool,
    electrical: bool,
) -> Result<Value> {
    let exe = dir
        .canonicalize()?
        .join(format!("runner{}", std::env::consts::EXE_SUFFIX));
    fs::copy(std::env::current_exe()?, &exe)?;
    let expected_hash = evidence::digest(&exe)?;
    let revision = crate::process::check(Command::new("git").args(["rev-parse", "HEAD"]))?;
    let revision = std::str::from_utf8(&revision.stdout)?.trim();
    let children = dir.canonicalize()?.join("children");
    let mut reports = BTreeMap::new();
    let cases = REGRESSION
        .iter()
        .chain(COMPARISONS.iter().filter(|_| historical));
    for suite in cases.filter(|s| !electrical || s.starts_with("o2-")) {
        let mut cmd = Command::new(&exe);
        cmd.arg(suite).env("ELECTROBOW_VALIDATION_OUT", &children);
        if COMPARISONS.contains(suite) {
            cmd.arg(reference.canonicalize()?);
        }
        let run = cmd.output()?;
        fs::write(dir.join(format!("{suite}.stdout")), run.stdout)?;
        fs::write(dir.join(format!("{suite}.stderr")), &run.stderr)?;
        ensure(
            run.status.success(),
            &format!("{suite}: {}", String::from_utf8_lossy(&run.stderr)),
        )?;
        let receipt = publication::inspect(&children.join(suite), suite)?;
        ensure(
            receipt["status"] == "passed"
                && receipt["executable_sha256"] == expected_hash
                && receipt["revision"] == revision,
            "unsuccessful child receipt",
        )?;
        reports.insert(suite,json!({"id":receipt["run_id"],"executable":receipt["executable_sha256"],"revision":receipt["revision"],"elapsed_seconds":receipt["elapsed_seconds"],"evidence":receipt["artifacts"]["evidence.json"]}));
        evidence::atomic(&dir.join("inventory.json"), &json!(reports))?;
    }
    let electrical_report = crate::electrical::aggregate::run(&children)?;
    evidence::atomic(&dir.join("electrical.json"), &electrical_report)?;
    if complete {
        let pending = [
            "independent disposition of the legacy audition gain serialization difference",
            "remaining complete I1 dependent report fields, including motion prerequisites, lifecycle and study control interpretation",
        ];
        evidence::atomic(
            &dir.join("unqualified.json"),
            &json!({"owners":pending,"review":"8.2","qualification_passed":false}),
        )?;
        return Err(format!(
            "complete qualification remains open: {}",
            pending.join(", ")
        )
        .into());
    }
    Ok(json!({"suites":reports,"historical_comparisons":historical}))
}
