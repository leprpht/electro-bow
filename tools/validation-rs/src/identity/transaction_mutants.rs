//! Rebuild actual Rust transaction/record readers after one coherent source edit at a time.
use crate::{Result, ensure, evidence, process, replay};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};
struct Mutant<'a> {
    name: &'a str,
    file: &'a str,
    old: &'a str,
    new: &'a str,
    suite: &'a str,
    predicate: &'a str,
}
fn cases() -> Vec<Mutant<'static>> {
    let mut cases = vec![
        Mutant {
            name: "expiry bypass",
            file: "identity/clock.rs",
            old: "t - anchor >= acquire",
            new: "t - anchor > acquire",
            suite: "i1-events",
            predicate: "elapsed acquisition eligibility",
        },
        Mutant {
            name: "duplicate gesture acceptance",
            file: "identity/clock.rs",
            old: "if !seen.insert(k) {",
            new: "if !seen.insert(k) && k != \"hold\" {",
            suite: "i1-events",
            predicate: "duplicate event rejection",
        },
        Mutant {
            name: "stale publication",
            file: "identity/clock.rs",
            old: "Some(\"hold\") if owner.is_some() && owner == live => {\n                    job = None;",
            new: "Some(\"hold\") if owner.is_some() && owner == live => {",
            suite: "i1-events",
            predicate: "stale feature publication",
        },
        Mutant {
            name: "wrong physical epoch",
            file: "identity/clock.rs",
            old: "epoch = Some(next);",
            new: "epoch = Some(next+1);",
            suite: "i1-events",
            predicate: "physical epoch ownership",
        },
        Mutant {
            name: "stereo abort",
            file: "identity/stereo.rs",
            old: "self.failed_epoch = self.owner;",
            new: "return Err(error);",
            suite: "i1-stereo",
            predicate: "epoch fault site=3 offset=0",
        },
        Mutant {
            name: "double cache envelope",
            file: "identity/stereo.rs",
            old: "row[10] = self.held[1] * w;",
            new: "row[10] = self.held[0] * self.held[1] * w;",
            suite: "i1-stereo",
            predicate: "conversion joined sample column 10",
        },
        Mutant {
            name: "wrong M5 denominator",
            file: "electrical/records.rs",
            old: "ensure(&actual == taps, \"canonical Rust tap schema\")",
            new: "Ok(())",
            suite: "o2-records",
            predicate: "surviving substituted original M5",
        },
        Mutant {
            name: "shifted wet pair",
            file: "electrical/records.rs",
            old: "digest(off.samples.iter().copied()) == truth.delivered[0]\n            && digest(on.samples.iter().copied()) == truth.delivered[1]",
            new: "true",
            suite: "o2-records",
            predicate: "surviving rolled wet valid tags",
        },
        Mutant {
            name: "Review 8.2 half product",
            file: "identity/stereo.rs",
            old: "row[10] = row[9] * (signed * row[8]);",
            new: "row[10] = 0.5 * row[9] * (signed * row[8]);",
            suite: "i1-stereo",
            predicate: "bank joined sample column 10",
        },
    ];
    for (name, old, new, predicate) in [
        (
            "candidate half product",
            "let product = energy * signed;",
            "let product = 0.5 * energy * signed;",
            "P == E*(signed_gain*wet)",
        ),
        (
            "missing captured gain",
            "let signed = gain * normalized;",
            "let signed = normalized;",
            "signed gain times normalized wet",
        ),
        (
            "wrong polarity",
            "let signed = gain * normalized;",
            "let signed = gain.abs() * normalized;",
            "signed gain times normalized wet",
        ),
        (
            "incorrect Character weighting",
            "+ self.character * self.product;",
            "+ self.product;",
            "live A4 mix",
        ),
        (
            "double live envelope",
            "let product = energy * signed;",
            "let product = energy * energy * signed;",
            "P == E*(signed_gain*wet)",
        ),
        (
            "incorrect fade mix",
            "sample.convert()?;\n        Ok(sample)\n    }\n}",
            "sample.convert()?; sample.mix -= character*(1.-pair[0])*pair[1]; sample.output=sample.mix as f32;\n        Ok(sample)\n    }\n}",
            "cached-fade A4 mix",
        ),
    ] {
        cases.push(Mutant {
            name,
            file: "identity/delivery.rs",
            old,
            new,
            suite: "i1-contract",
            predicate,
        });
    }
    cases.push(Mutant{name:"late-neutral candidate",file:"identity/candidate.rs",old:"let scale = self.slew.advance(self.target, n);",new:"let step=1./f64::from(n); self.slew.value+=(self.target-self.slew.value).clamp(-step,step); let scale=self.slew.value;",suite:"i1-contract",predicate:"exact neutral on Nth decrement"});
    cases
}
pub fn run(out: &Path) -> Result<Value> {
    ensure(
        std::env::var_os("ELECTROBOW_EXPECT_EARLY_MUTANT_REJECTION").is_none(),
        "complete mutant escaped its nominated behavioral boundary",
    )?;
    let original = Path::new("tools/validation-rs");
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let dest = root.join(original);
    fs::create_dir_all(&dest)?;
    for name in ["src", "contracts", "native", "reference"] {
        process::copy_tree(&original.join(name), &dest.join(name))?;
    }
    for name in ["Cargo.toml", "Cargo.lock", "build.rs", "source_identity.rs"] {
        fs::copy(original.join(name), dest.join(name))?;
    }
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "CMakeLists.txt",
    ] {
        fs::copy(name, root.join(name))?;
    }
    for name in ["rust/dsp", "tools/validation", "Source"] {
        process::copy_tree(Path::new(name), &root.join(name))?;
    }
    // Use the actual tracked file index for runtime provenance while reading the temporary source tree.
    let git = process::check(Command::new("git").args(["rev-parse", "--absolute-git-dir"]))?;
    let git = std::str::from_utf8(&git.stdout)?.trim();
    let mut measured = BTreeMap::new();
    for variant in cases() {
        let source = dest.join("src").join(variant.file);
        let before = fs::read_to_string(&source)?;
        let mut modified = before.clone();
        replay::edit(&mut modified, variant.old, variant.new)?;
        fs::write(&source, modified)?;
        let build = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--locked",
                "--offline",
                "--manifest-path",
                "tools/validation-rs/Cargo.toml",
            ])
            .current_dir(root)
            .env("CARGO_TARGET_DIR", root.join("target"))
            .output()?;
        ensure(
            build.status.success(),
            &format!(
                "{} compile: {}",
                variant.name,
                String::from_utf8_lossy(&build.stderr)
            ),
        )?;
        let binary = root.join("target/release").join(format!(
            "electrobow-validation{}",
            std::env::consts::EXE_SUFFIX
        ));
        let output = root.join("receipts");
        let complete = [
            "candidate half product",
            "late-neutral candidate",
            "Review 8.2 half product",
            "wrong M5 denominator",
            "shifted wet pair",
        ]
        .contains(&variant.name);
        let entry = if complete {
            "qualification"
        } else {
            variant.suite
        };
        let mut child = Command::new(&binary);
        child
            .arg(entry)
            .current_dir(root)
            .env("GIT_DIR", git)
            .env("GIT_WORK_TREE", root)
            .env("ELECTROBOW_VALIDATION_OUT", &output);
        if complete {
            child.env("ELECTROBOW_EXPECT_EARLY_MUTANT_REJECTION", "1");
        }
        let command = child.output()?;
        let parent = evidence::strict(&fs::read(output.join(entry).join("current.json"))?)?;
        let receipt = if complete {
            ensure(
                parent["status"] == "failed"
                    && parent["error"]
                        .as_str()
                        .is_some_and(|s| s.starts_with(&format!("{}:", variant.suite))),
                "complete mutant exited outside the producing suite",
            )?;
            let invocation = parent["run_id"].as_str().ok_or("failed aggregate run")?;
            let child = output
                .join(entry)
                .join(invocation)
                .join("children")
                .join(variant.suite)
                .join("current.json");
            evidence::strict(&fs::read(child)?)?
        } else {
            parent.clone()
        };
        ensure(
            command.status.code() == Some(1)
                && receipt["status"] == "failed"
                && receipt["error"] == variant.predicate,
            &format!(
                "{} wrong first failure: {}",
                variant.name,
                String::from_utf8_lossy(&command.stderr)
            ),
        )?;
        measured.insert(variant.name,json!({"source":variant.file,"before":variant.old,"after":variant.new,"source_sha256":evidence::digest(&source)?,"executable_sha256":evidence::digest(&binary)?,"suite":variant.suite,"entry_point":entry,"entry_receipt":parent,"expected":variant.predicate,"actual":receipt,"exit":command.status.code()}));
        evidence::atomic(&out.join("mutations.json"), &json!(measured))?;
        fs::write(source, before)?;
    }
    Ok(json!({"variants":measured}))
}
