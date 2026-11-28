use crate::{Result, ensure};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

pub fn check(cmd: &mut Command) -> Result<Output> {
    let r = cmd.output()?;
    if !r.status.success() {
        return Err(format!(
            "{cmd:?} exited {}\n{}\n{}",
            r.status,
            String::from_utf8_lossy(&r.stdout),
            String::from_utf8_lossy(&r.stderr)
        )
        .into());
    }
    Ok(r)
}
pub fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), to.join(entry.file_name()))?;
        }
    }
    Ok(())
}
pub fn dsp_copy(root: &Path) -> Result<()> {
    for f in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
        fs::copy(f, root.join(f))?;
    }
    copy_tree(Path::new("rust/dsp"), &root.join("rust/dsp"))
}
pub fn native(out: &Path) -> Result<Value> {
    let mut steps = vec![];
    let mut step = |name: &str, cmd: &mut Command| -> Result<Vec<u8>> {
        let r = cmd.output()?;
        fs::write(out.join(format!("{name}.stdout")), &r.stdout)?;
        fs::write(out.join(format!("{name}.stderr")), &r.stderr)?;
        ensure(
            r.status.success(),
            &format!("{name} failed, see {}", out.display()),
        )?;
        steps.push(json!({"name":name,"status":r.status.code(),"argv":cmd.get_args().map(|x|x.to_string_lossy()).collect::<Vec<_>>()}));
        Ok(r.stdout)
    };
    step(
        "build",
        Command::new("cmake").args(["--build", "build", "--config", "Release", "--parallel", "6"]),
    )?;
    step(
        "ctest",
        Command::new("ctest").args([
            "--test-dir",
            "build",
            "--build-config",
            "Release",
            "--output-on-failure",
            "--no-tests=error",
        ]),
    )?;
    #[cfg(target_os = "macos")]
    {
        let bundle = Path::new("build/ElectroBow_artefacts/Release/VST3/ElectroBow.vst3");
        step(
            "signature",
            Command::new("codesign")
                .args(["--verify", "--deep", "--strict"])
                .arg(bundle),
        )?;
        let binary = bundle.join("Contents/MacOS/ElectroBow");
        let symbols = step("exports", Command::new("nm").arg("-gU").arg(&binary))?;
        let names: std::collections::BTreeSet<_> = std::str::from_utf8(&symbols)?
            .lines()
            .filter_map(|l| l.split_whitespace().last())
            .collect();
        let header = fs::read_to_string("rust/dsp/include/electrobow_dsp.h")?;
        let mut exports = vec!["_GetPluginFactory".to_string()];
        for line in header
            .lines()
            .filter(|line| !line.trim_start().starts_with('*'))
        {
            if let Some(start) = line.find("eb_")
                && let Some(end) = line[start..].find('(')
            {
                let symbol = &line[start..start + end];
                if symbol
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    exports.push(format!("_{symbol}"));
                }
            }
        }
        ensure(
            exports.len() == 20 && exports.iter().all(|n| names.contains(n.as_str())),
            "expected VST3 factory and nineteen public ABI exports",
        )?;
        Ok(
            json!({"steps":steps,"exports":exports,"plugin_sha256":crate::evidence::digest(&binary)?}),
        )
    }
    #[cfg(not(target_os = "macos"))]
    Ok(json!({"steps":steps,"signature":"macOS only"}))
}
