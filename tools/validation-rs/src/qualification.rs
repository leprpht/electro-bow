//! Aggregate only receipts generated as children of this invocation.
use crate::{Result, ensure, evidence, publication};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};
const REGRESSION: &[&str] = &[
    "i1-stereo",
    "o2-records",
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
    "i1-study-controls",
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
    let mut differences = BTreeMap::new();
    if !electrical {
        let audit = evidence::strict(&fs::read(publication::current_artifact(
            &children,
            "i1-artifacts",
            "evidence.json",
        )?)?)?;
        for (name, row) in audit["results"]["artifacts"]
            .as_object()
            .ok_or("missing measured artifact inventory")?
        {
            if row["entry"] != row["actual"]
                && row["equivalence"] != "same measured fields, invocation-local renderer identity"
            {
                differences.insert(name.clone(), row.clone());
            }
        }
    }
    if historical {
        let comparisons: BTreeMap<_, _> = COMPARISONS
            .iter()
            .filter_map(|suite| reports.get(suite).map(|r| (*suite, r)))
            .collect();
        ensure(
            comparisons.len() == COMPARISONS.len(),
            "missing differential evidence",
        )?;
        evidence::atomic(
            &dir.join("comparisons.json"),
            &json!({"completed_comparisons":comparisons,"unresolved_artifact_differences":differences}),
        )?;
    }
    if complete && !differences.is_empty() {
        evidence::atomic(
            &dir.join("unqualified.json"),
            &json!({"review":"8.2","qualification_passed":false,"unresolved_artifact_differences":differences,"children":reports}),
        )?;
        return Err(format!(
            "complete qualification: unresolved artifact differences in {}",
            differences.keys().cloned().collect::<Vec<_>>().join(", ")
        )
        .into());
    }
    Ok(
        json!({"suites":reports,"historical_comparisons":historical,"unresolved_artifact_differences":differences}),
    )
}
