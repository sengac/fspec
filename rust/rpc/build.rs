//! Conservative compatibility fingerprint for the complete wire definitions.
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::Write,
    path::PathBuf,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut paths = vec![PathBuf::from("src/lib.rs")];
    for entry in std::fs::read_dir("../rpc-types/src")? {
        let path = entry?.path();
        if path.extension().is_some_and(|x| x == "rs") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut hash = DefaultHasher::new();
    let mut out = std::io::stdout().lock();
    for path in paths {
        writeln!(out, "cargo:rerun-if-changed={}", path.display())?;
        std::fs::read(path)?.hash(&mut hash);
    }
    writeln!(
        out,
        "cargo:rustc-env=FSPEC_RPC_SCHEMA={:016x}",
        hash.finish()
    )?;
    Ok(())
}
