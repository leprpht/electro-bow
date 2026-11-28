use crate::{Result, ensure, process};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

#[derive(Deserialize)]
struct Mutation {
    suite: String,
    name: String,
    test: String,
    marker: String,
    edits: Vec<Edit>,
}
#[derive(Deserialize)]
struct Edit {
    file: String,
    before: String,
    after: String,
}

pub fn run(out: &Path) -> Result<Value> {
    let cases: Vec<Mutation> =
        serde_json::from_str(include_str!("../contracts/source-mutations.json"))?;
    let scratch = tempfile::tempdir_in(out)?;
    let root = scratch.path();
    process::dsp_copy(root)?;
    let mut results = vec![];
    for (i, case) in cases.iter().enumerate() {
        for edit in &case.edits {
            let path = root.join(&edit.file);
            let text = fs::read_to_string(&path)?;
            ensure(
                text.matches(&edit.before).count() == 1,
                &format!("{} ambiguous edit in {}", case.name, edit.file),
            )?;
            fs::write(path, text.replace(&edit.before, &edit.after))?;
        }
        let mut command = Command::new("cargo");
        command
            .current_dir(root)
            .args(["test", "--lib", "--locked", "--offline"]);
        if case.suite == "o1" {
            command.arg("--release");
        }
        let r = command.arg(&case.test).args(["--", "--exact"]).output()?;
        let log = format!(
            "{}\n{}",
            String::from_utf8_lossy(&r.stdout),
            String::from_utf8_lossy(&r.stderr)
        );
        fs::write(out.join(format!("mutant-{i}.log")), &log)?;
        let output = std::str::from_utf8(&r.stdout)?;
        ensure(
            !r.status.success()
                && output.contains(&format!("test {} ... FAILED", case.test))
                && output.contains("panicked at")
                && output.contains(&case.marker),
            &format!("{}: expected {} in mutant-{i}.log", case.name, case.marker),
        )?;
        if case.suite == "guide" {
            ensure(
                output.contains("panicked at rust/dsp/src/instrument/waveguide/tests.rs:"),
                "wrong guide failure site",
            )?;
        }
        results.push(json!({"name":case.name,"test":case.test,"suite":case.suite,"exit":r.status.code(),"predicate":case.marker,"first_failure":output.lines().find(|l|l.contains(&case.marker))}));
        for edit in &case.edits {
            fs::copy(&edit.file, root.join(&edit.file))?;
        }
    }
    Ok(results.into())
}

pub fn state_parity(out: &Path) -> Result<Value> {
    let scratch = tempfile::tempdir_in(out)?;
    let root = scratch.path();
    process::dsp_copy(root)?;
    let instrument = Path::new("rust/dsp/src/instrument");
    let mut traces = Vec::new();
    for historical in [true, false] {
        for file in ["mod.rs", "friction.rs", "ring.rs", "waveguide.rs"] {
            let path = instrument.join(file);
            let data = if historical {
                process::check(Command::new("git").args([
                    "show",
                    &format!(
                        "0c98f608a0d46e29693320f36e85f173cd3faefe:{}",
                        path.to_string_lossy().replace('\\', "/")
                    ),
                ]))?
                .stdout
            } else {
                fs::read(&path)?
            };
            fs::write(root.join(path), data)?;
        }
        let tick = if historical {
            "g.tick(hz,0.1,k)"
        } else {
            "g.tick_observed(hz,0.1,k).map(|v|v.map(|(s,o)|{std::hint::black_box(o.velocity());s}))"
        };
        let probe = fs::read_to_string("tools/validation/o1_state.rs")?;
        fs::write(
            root.join(instrument).join("waveguide/tests.rs"),
            format!(
                "{probe}\nfn advance(g:&mut Guide,hz:f64,k:f64)->Result<Option<Sample>,Fault>{{{tick}}}"
            ),
        )?;
        let path = out.canonicalize()?.join(if historical {
            "entry.trace"
        } else {
            "observed.trace"
        });
        process::check(
            Command::new("cargo")
                .current_dir(root)
                .env("O1_STATE_TRACE", &path)
                .args([
                    "test",
                    "--release",
                    "--lib",
                    "--locked",
                    "--offline",
                    "instrument::waveguide::tests::epoch_state_trace",
                    "--",
                    "--exact",
                ]),
        )?;
        traces.push(fs::read(path)?);
    }
    let rows = traces[0].split_inclusive(|&b| b == b'\n').count();
    ensure(
        rows == 13820 && traces[0] == traces[1],
        "historical O1 state row mismatch",
    )?;
    Ok(json!({"rows":rows,"sha256":crate::evidence::digest(&out.join("observed.trace"))?}))
}
