mod audio;
mod contact;
mod electrical;
mod evidence;
mod guide;
mod identity;
mod mutations;
mod numerics;
mod observation;
mod package;
mod pitch;
mod process;
mod publication;
mod qualification;
mod recordings;
mod replay;
mod ring;
use serde_json::{Value, json};
use std::{
    env,
    path::{Path, PathBuf},
    process::ExitCode,
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
    "recordings",
    "repeatability",
    "recording-controls",
    "contact",
    "pitch",
    "pitch-compare",
    "ring",
    "guide",
    "guide-negative",
    "guide-compare",
    "regression",
    "differential",
    "qualification",
    "mutations",
    "contact-mutants",
    "i1-bank-compare",
    "i1-qr",
    "i1-layout",
    "i1-stereo",
    "i1-sources",
    "i1-listening",
    "i1-path-mutants",
    "i1-capture",
    "i1-review82",
    "signed-compare",
    "i1-motion",
    "i1-stationary",
    "o2-carrier",
    "o2-bench",
    "o2-acoustic",
    "o2-acoustic-compare",
    "o2-sweep",
    "o2-records",
    "o2-qualification",
    "o2-mutants",
    "o2-timing",
    "o1-state",
    "o1-replay",
    "o1-compare",
    "native",
    "package",
    "compare",
];
const DEFAULT_OUT: &str = "build/validation-migration/rust";

fn execute(suite: &str, dir: &Path, reference: &Path) -> Result<Value> {
    match suite {
        "repeatability" => recordings::scoring::run(dir, reference),
        "recordings" => recordings::preflight(dir, reference),
        "recording-controls" => recordings::controls(dir),
        "fixtures" => audio::fixtures(),
        "mutations" => mutations::run(dir),
        "contact-mutants" => contact::controls::mutants(dir),
        "o1-state" => mutations::state_parity(dir),
        "native" => process::native(dir),
        "package" => package::run(dir),
        "signed-compare" => identity::deadlines::signed(reference),
        "i1-review82" => identity::deadlines::run(dir),
        "i1-capture" => identity::capture::run(),
        "i1-path-mutants" => identity::paths::run(dir),
        "i1-listening" => identity::listening::run(
            dir,
            dir.parent().and_then(Path::parent).ok_or("suite root")?,
        ),
        "i1-sources" => identity::sources::run(dir),
        "i1-stereo" => identity::stereo::run(dir),
        "i1-layout" => identity::layout::run(),
        "i1-qr" => identity::qr::run(dir),
        "i1-motion" => identity::motion::run(dir),
        "i1-stationary" => identity::stationary::run(dir),
        "o2-records" => electrical::records::run(dir),
        "o2-sweep" => electrical::sweep::run(dir),
        "o2-acoustic-compare" => electrical::acoustic::differential(
            reference,
            dir.parent()
                .and_then(Path::parent)
                .ok_or("suite output root")?,
        ),
        "o2-acoustic" => electrical::acoustic::run(dir),
        "o2-bench" => electrical::bench(dir),
        "o2-carrier" => electrical::carrier::run(dir),
        "o2-mutants" => electrical::mutants(dir),
        "o2-timing" => electrical::timing(dir),
        "i1-bank-compare" => identity::compare(reference),
        "o1-compare" => observation::measures::differential(
            reference,
            dir.parent().and_then(Path::parent).ok_or("suite root")?,
        ),
        "o1-replay" => observation::run(dir),
        "contact" => contact::run(dir, None),
        "pitch" => pitch::run(dir),
        "pitch-compare" => pitch::differential(dir, reference),
        "guide-compare" => guide::differential(
            reference,
            dir.parent()
                .and_then(Path::parent)
                .ok_or("suite output root")?,
        ),
        "regression" | "qualification" | "differential" | "o2-qualification" => qualification::run(
            dir,
            reference,
            suite == "qualification",
            suite == "differential",
            suite == "o2-qualification",
        ),
        "guide-negative" => guide::negative(dir),
        "guide" => guide::run(dir),
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
    if suite == "artifact" {
        ensure(
            args.len() == 3 && SUITES.contains(&args[1].as_str()),
            "artifact <suite> <relative filename>",
        )?;
        println!(
            "{}",
            publication::current_artifact(&root, &args[1], &args[2])?.display()
        );
        return Ok(());
    }
    if suite == "inspect" {
        ensure(
            args.len() == 2 && SUITES.contains(&args[1].as_str()),
            "inspect requires a suite name",
        )?;
        let report = publication::inspect(&root.join(&args[1]), &args[1])?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return ensure(report["status"] == "passed", "suite lacks completed pass");
    }
    ensure(
        SUITES.contains(&suite),
        "unknown validation suite (use list)",
    )?;
    ensure(
        (args.len() == 1 && !matches!(suite, "recordings" | "repeatability"))
            || (matches!(
                suite,
                "compare"
                    | "pitch-compare"
                    | "recordings"
                    | "repeatability"
                    | "o1-compare"
                    | "guide-compare"
                    | "o2-acoustic-compare"
                    | "signed-compare"
                    | "i1-bank-compare"
                    | "differential"
            ) && args.len() == 2),
        "unexpected argument",
    )?;
    let reference = PathBuf::from(
        args.get(1)
            .map_or("build/validation-migration/reference", String::as_str),
    );
    publication::run(suite, &root, |dir| execute(suite, dir, &reference))
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
