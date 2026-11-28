// Exercise the public entry point so a stale passing invocation cannot hide a failure.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn cmd(out: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_electrobow-validation"));
    cmd.current_dir(root())
        .args(args)
        .env("ELECTROBOW_VALIDATION_OUT", out);
    cmd
}
fn current(out: &Path, suite: &str) -> Value {
    serde_json::from_slice(&fs::read(out.join(suite).join("current.json")).unwrap()).unwrap()
}
#[test]
fn a_new_failure_supersedes_success_and_dependent_readers_reject_damage() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path();
    assert!(cmd(out, &["fixtures"]).output().unwrap().status.success());
    assert!(
        cmd(out, &["inspect", "fixtures"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let before = current(out, "fixtures");
    let dir = out
        .join("fixtures")
        .join(before["run_id"].as_str().unwrap());
    let evidence = dir.join("evidence.json");
    let saved = fs::read(&evidence).unwrap();
    for invalid in [br#"{"nested":{"v":0,"v":1}}"#.as_slice(), b"[1e999]", b"{}"] {
        fs::write(&evidence, invalid).unwrap();
        assert!(
            !cmd(out, &["artifact", "fixtures", "evidence.json"])
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    fs::write(&evidence, saved).unwrap();
    assert!(
        cmd(out, &["artifact", "fixtures", "evidence.json"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let bad_working_dir = root().join("tools/validation-rs");
    assert!(
        !cmd(out, &["fixtures"])
            .current_dir(bad_working_dir)
            .output()
            .unwrap()
            .status
            .success()
    );
    let failed = current(out, "fixtures");
    assert_eq!(failed["status"], "failed");
    assert_ne!(failed["run_id"], before["run_id"]);
    assert!(
        !cmd(out, &["inspect", "fixtures"])
            .output()
            .unwrap()
            .status
            .success()
    );
}
#[test]
fn interrupted_writer_and_concurrent_attempt() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path();
    let suite = "o1-replay";
    let mut writer = cmd(out, &[suite])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while !out.join(suite).join("current.json").exists() {
        assert!(
            started.elapsed() < Duration::from_secs(30) && writer.try_wait().unwrap().is_none()
        );
        thread::sleep(Duration::from_millis(5));
    }
    let running = current(out, suite);
    assert_eq!(running["status"], "running");
    assert!(!cmd(out, &[suite]).output().unwrap().status.success());
    assert_eq!(current(out, suite), running);
    writer.kill().unwrap();
    writer.wait().unwrap();
    assert!(
        !cmd(out, &["inspect", suite])
            .output()
            .unwrap()
            .status
            .success()
    );
    let interrupted = current(out, suite);
    assert_eq!(interrupted["run_id"], running["run_id"]);
    assert_eq!(interrupted["status"], "interrupted");
    let terminal = out
        .join(suite)
        .join(running["run_id"].as_str().unwrap())
        .join("receipt.json");
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(terminal).unwrap()).unwrap(),
        interrupted
    );
}

#[test]
fn inspect_rejects_coherently_rehashed_incomplete_provenance() {
    use sha2::{Digest, Sha256};
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path();
    assert!(cmd(out, &["fixtures"]).output().unwrap().status.success());
    let original = current(out, "fixtures");
    let copy = out.join("copy");
    let suite = copy.join("fixtures");
    let id = original["run_id"].as_str().unwrap();
    let run = suite.join(id);
    fs::create_dir_all(&run).unwrap();
    fs::copy(out.join("fixtures/runner.lock"), suite.join("runner.lock")).unwrap();
    for file in fs::read_dir(out.join("fixtures").join(id)).unwrap() {
        let file = file.unwrap();
        fs::copy(file.path(), run.join(file.file_name())).unwrap();
    }
    let path = run.join("provenance.json");
    let valid: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for missing in [
        &["inputs", "rustc", "target", "profile"][..],
        &["inputs"],
        &["rustc"],
        &["target"],
        &["profile"],
    ] {
        let mut source = valid.clone();
        for key in missing {
            source.as_object_mut().unwrap().remove(*key);
        }
        let bytes = serde_json::to_vec(&source).unwrap();
        let mut receipt = original.clone();
        receipt["artifacts"]["provenance.json"] = format!("{:x}", Sha256::digest(&bytes)).into();
        fs::write(&path, bytes).unwrap();
        let receipt = serde_json::to_vec(&receipt).unwrap();
        fs::write(run.join("receipt.json"), &receipt).unwrap();
        fs::write(suite.join("current.json"), receipt).unwrap();
        let result = cmd(&copy, &["inspect", "fixtures"]).output().unwrap();
        assert!(!result.status.success());
        let reason = String::from_utf8(result.stderr).unwrap();
        assert!(
            reason.contains("rustc identity")
                || reason.contains("compilation")
                || reason.contains("runtime input inventory"),
            "{reason}"
        );
    }
}

#[test]
fn dependent_layout_is_strict_after_coherent_rehash() {
    use sha2::{Digest, Sha256};

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path();
    let suite = "i1-layout";
    assert!(cmd(out, &[suite]).output().unwrap().status.success());
    let receipt = current(out, suite);
    let run = out.join(suite).join(receipt["run_id"].as_str().unwrap());
    let report = run.join("evidence.json");
    let original = fs::read_to_string(&report).unwrap();
    let mut wrong_type: Value = serde_json::from_str(&original).unwrap();
    assert!(wrong_type["results"]["measured_allocations"].is_null());
    wrong_type["results"]["maximum"]["analysis_bytes"] = "61376".into();
    let mut inputs = vec![serde_json::to_string(&wrong_type).unwrap()];
    for value in ["NaN", "Infinity", "-Infinity", "1e999", "?"] {
        inputs.push(original.replacen(
            "\"results\": {",
            &format!("\"results\": {{\"extension\":{value},"),
            1,
        ));
    }
    inputs.push(original.replacen("\"results\": {", "\"results\": {}, \"results\": {", 1));
    for bytes in inputs {
        assert_ne!(bytes, original);
        let mut coherent = receipt.clone();
        coherent["artifacts"]["evidence.json"] = format!("{:x}", Sha256::digest(&bytes)).into();
        fs::write(&report, bytes).unwrap();
        let receipt_bytes = serde_json::to_vec(&coherent).unwrap();
        fs::write(run.join("receipt.json"), &receipt_bytes).unwrap();
        fs::write(out.join(suite).join("current.json"), receipt_bytes).unwrap();
        let result = cmd(out, &["artifact", suite, "evidence.json"])
            .output()
            .unwrap();
        assert!(!result.status.success());
        let reason = String::from_utf8(result.stderr).unwrap();
        assert!(!reason.contains("digest mismatch"), "{reason}");
    }
}
