mod source_identity;
use std::{env, path::Path, process::Command};
fn main() {
    for input in source_identity::INPUTS {
        println!("cargo:rerun-if-changed={input}");
    }
    let digest = source_identity::calculate(Path::new(".")).expect("validation source manifest");
    let toolchain = Command::new(env::var_os("RUSTC").unwrap())
        .arg("-V")
        .output()
        .expect("Rust compiler");
    assert!(toolchain.status.success());
    println!("cargo:rustc-env=VALIDATION_SOURCE_SHA256={digest}");
    println!(
        "cargo:rustc-env=VALIDATION_RUSTC={}",
        String::from_utf8(toolchain.stdout).unwrap().trim()
    );
    println!(
        "cargo:rustc-env=VALIDATION_TARGET={}",
        env::var("TARGET").unwrap()
    );
    println!(
        "cargo:rustc-env=VALIDATION_PROFILE={}",
        env::var("PROFILE").unwrap()
    );
}
