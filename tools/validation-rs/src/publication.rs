use crate::{Result, ensure, evidence};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Status {
    Running,
    Failed,
    Interrupted,
    Passed,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: u32,
    suite: String,
    run_id: String,
    status: Status,
    executable_sha256: String,
    revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    elapsed_seconds: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    artifacts: Option<BTreeMap<String, String>>,
}
fn files(dir: &Path) -> Result<BTreeMap<String, String>> {
    fn walk(root: &Path, dir: &Path, entries: &mut BTreeMap<String, String>) -> Result<()> {
        for file in fs::read_dir(dir)? {
            let f = file?;
            let path = f.path();
            if f.file_type()?.is_dir() {
                walk(root, &path, entries)?;
            } else if f.file_type()?.is_file() {
                let name = path
                    .strip_prefix(root)?
                    .to_str()
                    .ok_or("artifact path encoding")?
                    .replace('\\', "/");
                if name != "receipt.json" {
                    entries.insert(name, evidence::digest(&path)?);
                }
            } else {
                return Err(format!("unexpected artifact type {}", path.display()).into());
            }
        }
        Ok(())
    }
    let mut result = BTreeMap::new();
    walk(dir, dir, &mut result)?;
    Ok(result)
}
fn sha(text: &str, digits: usize) -> bool {
    text.len() == digits
        && text
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn inspect(dir: &Path, suite: &str) -> Result<Value> {
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir.join("runner.lock"))?;
    let idle = match lock.try_lock() {
        Ok(()) => true,
        Err(fs::TryLockError::WouldBlock) => false,
        Err(fs::TryLockError::Error(e)) => return Err(e.into()),
    };
    let current = dir.join("current.json");
    let mut value = evidence::strict(&fs::read(&current)?)?;
    let r: Receipt = serde_json::from_value(value.clone())?;
    ensure(
        matches!(r.schema, 2 | 3)
            && r.suite == suite
            && r.run_id.starts_with("run-")
            && r.run_id[4..].len() == 6
            && r.run_id[4..].bytes().all(|v| v.is_ascii_alphanumeric())
            && sha(&r.executable_sha256, 64)
            && sha(&r.revision, 40),
        "invalid run identity",
    )?;
    ensure(
        idle || r.status == Status::Running,
        "publication in progress",
    )?;
    let run = dir.join(&r.run_id);
    if r.status == Status::Running && idle {
        value["status"] = "interrupted".into();
        evidence::atomic(&run.join("receipt.json"), &value)?;
        evidence::atomic(&current, &value)?;
    } else if r.status == Status::Passed {
        ensure(
            r.error.is_none() && r.elapsed_seconds.is_some_and(|s| s.is_finite() && s >= 0.),
            "invalid successful receipt",
        )?;
        ensure(
            evidence::strict(&fs::read(run.join("receipt.json"))?)? == value,
            "published run receipt",
        )?;
        let inventory = r.artifacts.ok_or("missing artifact inventory")?;
        ensure(
            inventory.contains_key("evidence.json") && files(&run)? == inventory,
            "published artifact inventory or digest mismatch",
        )?;
        ensure(
            r.schema < 3 || inventory.contains_key("provenance.json"),
            "missing source provenance",
        )?;
        if inventory.contains_key("provenance.json") {
            let source = evidence::strict(&fs::read(run.join("provenance.json"))?)?;
            ensure(
                source["run_id"] == r.run_id
                    && source["suite"] == suite
                    && source["executable_sha256"] == r.executable_sha256
                    && source["revision"] == r.revision
                    && source["source_sha256"].as_str().is_some_and(|s| sha(s, 64)),
                "source provenance disagrees with receipt",
            )?;
        }
        let payload = evidence::strict(&fs::read(run.join("evidence.json"))?)?;
        ensure(
            payload["run_id"] == r.run_id
                && payload["suite"] == suite
                && payload
                    .get("results")
                    .is_some_and(|v| v.is_object() || v.is_array()),
            "dependent evidence run/suite/schema",
        )?;
    }
    Ok(value)
}

pub fn run(suite: &str, root: &Path, body: impl FnOnce(&Path) -> Result<Value>) -> Result<()> {
    let dir = root.join(suite);
    fs::create_dir_all(&dir)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("runner.lock"))?;
    lock.try_lock()?;
    let current = dir.join("current.json");
    // Retire the public pointer before any subsequent setup step can fail.
    match fs::remove_file(&current) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let path = tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(&dir)?
        .keep();
    let id = path.file_name().unwrap().to_str().ok_or("run name")?;
    let revision =
        crate::process::check(std::process::Command::new("git").args(["rev-parse", "HEAD"]))?;
    let mut receipt = Receipt {
        schema: 3,
        suite: suite.into(),
        run_id: id.into(),
        status: Status::Running,
        executable_sha256: evidence::digest(&std::env::current_exe()?)?,
        revision: String::from_utf8(revision.stdout)?.trim().into(),
        elapsed_seconds: None,
        error: None,
        artifacts: None,
    };
    evidence::atomic(&current, &serde_json::to_value(&receipt)?)?;
    let start = Instant::now();
    let result = (|| -> Result<Value>{
        let compiled=env!("VALIDATION_SOURCE_SHA256");
        ensure(crate::source_identity::calculate(Path::new("tools/validation-rs"))?==compiled,"validation executable built from different sources")?;
        let output=crate::process::check(std::process::Command::new("git").args(["ls-files","-z","rust/dsp","tools/validation","Source","CMakeLists.txt","Cargo.toml","Cargo.lock","rust-toolchain.toml"]))?;
        let mut inputs=BTreeMap::new();
        for raw in output.stdout.split(|v|*v==0).filter(|s|!s.is_empty()){
            let name=std::str::from_utf8(raw)?;
            inputs.insert(name,evidence::digest(Path::new(name))?);
        }
        evidence::atomic(&path.join("provenance.json"),&json!({"source_sha256":compiled,"revision":receipt.revision,"executable_sha256":receipt.executable_sha256,"run_id":id,"suite":suite,"rustc":env!("VALIDATION_RUSTC"),"target":env!("VALIDATION_TARGET"),"profile":env!("VALIDATION_PROFILE"),"inputs":inputs}))?;
        body(&path)
    })().and_then(|v| {
        evidence::atomic(
            &path.join("evidence.json"),
            &json!({"suite":suite,"run_id":id,"results":v}),
        )?;
        receipt.artifacts = Some(files(&path)?);
        Ok(())
    });
    receipt.elapsed_seconds = Some(start.elapsed().as_secs_f64());
    receipt.status = if result.is_ok() {
        Status::Passed
    } else {
        Status::Failed
    };
    if let Err(e) = &result {
        receipt.error = Some(e.to_string());
    }
    let value = serde_json::to_value(receipt)?;
    evidence::atomic(&path.join("receipt.json"), &value)?;
    evidence::atomic(&current, &value)?;
    println!(
        "{suite} {} {}",
        value["status"].as_str().unwrap(),
        path.display()
    );
    result
}
pub fn current_artifact(root: &Path, suite: &str, name: &str) -> Result<PathBuf> {
    let dir = root.join(suite);
    let receipt = inspect(&dir, suite)?;
    ensure(
        receipt["status"] == "passed" && receipt["artifacts"].get(name).is_some(),
        "required successful artifact missing",
    )?;
    Ok(dir
        .join(receipt["run_id"].as_str().ok_or("run id")?)
        .join(name))
}
