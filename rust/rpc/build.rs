//! Conservative compatibility fingerprint for the complete wire definitions.
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
};

fn collect_inputs(
    directory: &Path,
    paths: &mut Vec<PathBuf>,
    out: &mut impl Write,
) -> std::io::Result<()> {
    writeln!(out, "cargo:rerun-if-changed={}", directory.display())?;
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_inputs(&path, paths, out)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            paths.push(path);
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout().lock();
    let mut paths = Vec::new();
    collect_inputs(Path::new("src"), &mut paths, &mut out)?;
    collect_inputs(Path::new("../rpc-types/src"), &mut paths, &mut out)?;
    paths.sort();
    let mut hash = DefaultHasher::new();
    for path in paths {
        writeln!(out, "cargo:rerun-if-changed={}", path.display())?;
        path.hash(&mut hash);
        std::fs::read(path)?.hash(&mut hash);
    }
    writeln!(
        out,
        "cargo:rustc-env=FSPEC_RPC_SCHEMA={:016x}",
        hash.finish()
    )?;
    Ok(())
}
