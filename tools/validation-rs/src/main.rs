mod audio;
mod contact;
mod electrical;
mod evidence;
mod identity;
mod mutations;
mod numerics;
mod observation;
mod pitch;
mod process;
mod replay;
mod ring;
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string().into())
    }
}
const SUITES: &[&str] = &[
    "fixtures",
    "contact",
    "pitch",
    "pitch-compare",
    "ring",
    "mutations",
    "i1-bank-compare",
    "i1-qr",
    "i1-motion",
    "i1-stationary",
    "o2-carrier",
    "o2-bench",
    "o2-mutants",
    "o2-timing",
    "o1-state",
    "o1-replay",
    "native",
    "compare",
];
const DEFAULT_OUT: &str = "build/validation-migration/rust";

fn execute(suite: &str, dir: &Path, reference: &Path) -> Result<Value> {
    match suite {
        "fixtures" => audio::fixtures(),
        "mutations" => mutations::run(dir),
        "o1-state" => mutations::state_parity(dir),
        "native" => process::native(dir),
        "i1-qr" => identity::qr::run(dir),
        "i1-motion" => identity::motion::run(dir),
        "i1-stationary" => identity::stationary::run(dir),
        "o2-bench" => electrical::bench(dir),
        "o2-carrier" => electrical::carrier::run(dir),
        "o2-mutants" => electrical::mutants(dir),
        "o2-timing" => electrical::timing(dir),
        "i1-bank-compare" => identity::compare(reference),
        "o1-replay" => observation::run(dir),
        "contact" => contact::run(dir, None),
        "pitch" => pitch::run(dir),
        "pitch-compare" => pitch::differential(dir, reference),
        "ring" => ring::run(dir, None),
        "compare" => Ok(
            json!({"fixtures":audio::fixtures()?,"numerics":numerics::differential(reference)?,"contact":contact::run(dir,Some(reference))?,"ring":ring::run(dir,Some(reference))?}),
        ),
        _ => Err(format!("unknown suite {suite}").into()),
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let suite = args.first().map(String::as_str).unwrap_or("help");
    if suite == "i1-bank-stream" {
        ensure(args.len() == 2, "i1-bank-stream <rate>")?;
        return identity::stream(args[1].parse()?);
    }
    if matches!(suite, "list" | "help" | "--help") {
        println!(
            "{}\ninspect <suite>\ncompare [reference-dir]",
            SUITES.join("\n")
        );
        return Ok(());
    }
    let root = PathBuf::from(
        env::var_os("ELECTROBOW_VALIDATION_OUT").unwrap_or_else(|| DEFAULT_OUT.into()),
    );
    if suite == "inspect" {
        ensure(
            args.len() == 2 && SUITES.contains(&args[1].as_str()),
            "inspect requires a suite name",
        )?;
        let dir = root.join(&args[1]);
        let lock = fs::File::open(dir.join("runner.lock"))?;
        let idle = match lock.try_lock() {
            Ok(()) => true,
            Err(fs::TryLockError::WouldBlock) => false,
            Err(fs::TryLockError::Error(e)) => return Err(e.into()),
        };
        let mut report = evidence::strict(&fs::read(dir.join("current.json"))?)?;
        if report["status"] == "running" && idle {
            report["status"] = "interrupted".into();
            evidence::atomic(&dir.join("current.json"), &report)?;
        }
        println!("{}", serde_json::to_string_pretty(&report)?);
        return ensure(report["status"] == "passed", "suite lacks completed pass");
    }
    ensure(
        SUITES.contains(&suite),
        "unknown validation suite (use list)",
    )?;
    ensure(
        args.len() == 1
            || (matches!(suite, "compare" | "pitch-compare" | "i1-bank-compare")
                && args.len() == 2),
        "unexpected argument",
    )?;
    let reference = PathBuf::from(
        args.get(1)
            .map_or("build/validation-migration/reference", String::as_str),
    );
    let dir = root.join(suite);
    fs::create_dir_all(&dir)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("runner.lock"))?;
    lock.try_lock()?;
    let run_dir = tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(&dir)?
        .keep();
    let id = run_dir.file_name().unwrap().to_string_lossy();
    let current = dir.join("current.json");
    let mut receipt = json!({"schema":1,"suite":suite,"run_id":id,"status":"running"});
    evidence::atomic(&current, &receipt)?;
    let start = Instant::now();
    let result = execute(suite, &run_dir, &reference)
        .and_then(|v| evidence::atomic(&run_dir.join("evidence.json"), &v));
    receipt["elapsed_seconds"] = start.elapsed().as_secs_f64().into();
    receipt["status"] = if result.is_ok() { "passed" } else { "failed" }.into();
    if let Err(e) = &result {
        receipt["error"] = e.to_string().into();
    }
    evidence::atomic(&run_dir.join("receipt.json"), &receipt)?;
    evidence::atomic(&current, &receipt)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    result
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("validation: {e}");
            ExitCode::FAILURE
        }
    }
}
