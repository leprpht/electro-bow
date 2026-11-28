//! Build-time and run-time identity of the non-shipping program and its embedded contracts.
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
pub const INPUTS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "build.rs",
    "source_identity.rs",
    "src",
    "contracts",
    "native",
    "reference/acoustic.json",
    "reference/mu-bounds.json",
    "../validation/fixtures/o1_sha256.tsv",
    "../validation/fixtures/o2_corner_peaks.tsv",
    "../validation/fixtures/o2_replay_sha256.tsv",
    "../validation/fixtures/o2_timing_terminals.json",
    "../validation/fixtures/o2_negative_predicates.json",
    "../validation/fixtures/stage07_audio_sha256.tsv",
];
pub fn calculate(root: &Path) -> io::Result<String> {
    fn collect(root: &Path, path: PathBuf, files: &mut Vec<PathBuf>) -> io::Result<()> {
        if root.join(&path).is_dir() {
            for entry in fs::read_dir(root.join(&path))? {
                collect(root, path.join(entry?.file_name()), files)?;
            }
        } else {
            files.push(path);
        }
        Ok(())
    }
    let mut files = Vec::new();
    for name in INPUTS {
        collect(root, PathBuf::from(name), &mut files)?;
    }
    files.sort();
    let mut total = Sha256::new();
    for file in files {
        let name = file.to_string_lossy().replace('\\', "/");
        total.update((name.len() as u64).to_le_bytes());
        total.update(name.as_bytes());
        total.update(Sha256::digest(fs::read(root.join(file))?));
    }
    Ok(format!("{:x}", total.finalize()))
}
