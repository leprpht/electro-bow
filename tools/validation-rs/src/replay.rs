//! Compile actual private modules with the reviewed read-only probes in a disposable tree.
use crate::{Result, ensure, process};
use serde::Deserialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
};

#[derive(Deserialize)]
struct Audit {
    ring: Vec<[String; 2]>,
    guide: Vec<[String; 2]>,
    commits: Vec<[String; 2]>,
    ring_epoch: String,
}
pub fn edit(s: &mut String, before: &str, after: &str) -> Result<()> {
    ensure(
        s.matches(before).count() == 1,
        &format!("ambiguous replay edit {before}"),
    )?;
    *s = s.replace(before, after);
    Ok(())
}
pub fn build(out: &Path, mutation: Option<(&str, &str, &str)>) -> Result<PathBuf> {
    let audit: Audit = serde_json::from_str(include_str!("../contracts/audit.json"))?;
    let scratch = tempfile::tempdir_in(out)?;
    let root = scratch.path();
    let modules = Path::new("rust/dsp/src/instrument");
    let target = root.join(modules);
    fs::create_dir_all(&target)?;
    for name in ["ring", "waveguide", "friction", "o2"] {
        let file = format!("{name}.rs");
        let mut code = fs::read_to_string(modules.join(&file))?;
        let tests = target.join(name);
        fs::create_dir(&tests)?;
        fs::write(tests.join("tests.rs"), "")?;
        match name {
            "ring" => {
                for [a, b] in &audit.ring {
                    edit(&mut code, a, b)?;
                }
                code.push_str(&audit.ring_epoch);
            }
            "waveguide" => {
                for [a, b] in audit.guide.iter().chain(&audit.commits) {
                    edit(&mut code, a, b)?;
                }
                if let Some(("relaunch", _, _)) = mutation {
                    let changes: Vec<[String; 2]> =
                        serde_json::from_str(include_str!("../contracts/guide-relaunch.json"))?;
                    for [before, after] in changes {
                        edit(&mut code, &before, &after)?;
                    }
                }
                code.push_str(&fs::read_to_string("tools/validation/guide_epoch.rs")?);
            }
            "o2" => {
                code.push_str(&fs::read_to_string("tools/validation/o2_probes.rs")?);
                if let Some(("o2", a, b)) = mutation {
                    edit(&mut code, a, b)?;
                }
            }
            _ => {}
        }
        fs::write(target.join(file), code)?;
    }
    fs::write(target.join("tests.rs"), "")?;
    let adapter = root.join("tools/validation");
    fs::create_dir_all(&adapter)?;
    for name in ["guide_render.rs", "o2_replay.rs", "o2_lifecycle.rs"] {
        let mut code = fs::read_to_string(Path::new("tools/validation").join(name))?;
        if name == "o2_replay.rs"
            && let Some(("harness", a, b)) = mutation
        {
            edit(&mut code, a, b)?;
        }
        fs::write(adapter.join(name), code)?;
    }
    let exe = out
        .canonicalize()?
        .join(format!("renderer{}", std::env::consts::EXE_SUFFIX));
    process::check(
        Command::new("rustc")
            .args([
                "--edition",
                "2024",
                "--cfg",
                "test",
                "--cfg",
                "epoch_audit",
                "--cfg",
                "o2_audit",
                "-O",
            ])
            .arg(adapter.join("guide_render.rs"))
            .arg("-o")
            .arg(&exe),
    )?;
    Ok(exe)
}
pub fn stream(exe: &Path, args: &[String], data: &[u8]) -> Result<Output> {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("missing replay stdin")?;
    let output = thread::scope(|scope| -> Result<Output> {
        let write = scope.spawn(move || stdin.write_all(data));
        let o = child.wait_with_output()?;
        let sent = write.join().map_err(|_| "replay input writer panicked")?;
        ensure(
            o.status.success(),
            &format!("replay {args:?}: {}", String::from_utf8_lossy(&o.stderr)),
        )?;
        sent?;
        Ok(o)
    })?;
    Ok(output)
}
