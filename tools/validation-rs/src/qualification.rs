//! Aggregate only receipts generated as children of this invocation.
use crate::{Result, ensure, evidence, publication};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};
const REGRESSION: &[&str] = &[
    "fixtures",
    "contact",
    "contact-mutants",
    "ring",
    "guide",
    "mutations",
    "o1-replay",
    "o1-state",
    "o2-bench",
    "o2-carrier",
    "o2-mutants",
    "i1-stationary",
    "i1-motion",
    "i1-qr",
    "i1-review82",
    "pitch",
    "native",
    "package",
    "o2-timing",
];
const COMPARISONS: &[&str] = &[
    "compare",
    "guide-compare",
    "pitch-compare",
    "i1-bank-compare",
];
pub fn run(dir: &Path, reference: &Path, complete: bool, historical: bool) -> Result<Value> {
    let exe = std::env::current_exe()?;
    let children = dir.canonicalize()?.join("children");
    let mut reports = BTreeMap::new();
    let cases = REGRESSION
        .iter()
        .chain(COMPARISONS.iter().filter(|_| historical));
    for suite in cases {
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
        ensure(receipt["status"] == "passed", "unsuccessful child receipt")?;
        reports.insert(suite,json!({"id":receipt["run_id"],"executable":receipt["executable_sha256"],"revision":receipt["revision"],"elapsed_seconds":receipt["elapsed_seconds"],"evidence":receipt["artifacts"]["evidence.json"]}));
        evidence::atomic(&dir.join("inventory.json"), &json!(reports))?;
    }
    if complete {
        let pending = [
            "complete O2 acoustic, listening and evidence-control inventory",
            "joined I1 capture, signed A4, cached stereo lifecycle and behavioral mutants",
            "controlled recording protocol and source discrimination scorer",
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
