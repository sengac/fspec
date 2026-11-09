//! COV-061: every Rust source under rust/ belongs to a Cargo package.
//!
//! Feature: spec/features/workspace-sources-belong-to-cargo-packages.feature

use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|name| name == "target" || name == ".git")
            {
                continue;
            }
            rust_sources(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn declares_package(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml"))
        .is_ok_and(|manifest| manifest.lines().any(|line| line.trim() == "[package]"))
}

/// The `members = [...]` of the root manifest, plus `patches/` (crates that
/// the workspace builds through `[patch.crates-io]`).
fn built_package_dirs(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))?;
    let members = manifest
        .split("members = [")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .ok_or("rust/Cargo.toml has no `members = [...]` list")?;
    let mut dirs: Vec<PathBuf> = members
        .lines()
        .filter_map(|line| line.split('#').next())
        .map(str::trim)
        .filter_map(|line| line.strip_prefix('"')?.split('"').next())
        .map(|member| root.join(member))
        .collect();
    dirs.push(root.join("patches"));
    Ok(dirs)
}

#[test]
fn no_rust_source_lies_outside_a_cargo_package() -> TestResult {
    // @step Given the rust/ workspace whose root manifest is a virtual workspace
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("common crate has no parent directory")?
        .to_path_buf();
    assert!(
        !declares_package(&root),
        "rust/Cargo.toml must stay a virtual workspace"
    );

    // @step When every .rs file outside rust/target is checked for an enclosing Cargo package
    let mut sources = Vec::new();
    rust_sources(&root, &mut sources)?;
    let built = built_package_dirs(&root)?;
    let orphans: Vec<&PathBuf> = sources
        .iter()
        .filter(|source| {
            let in_package = source
                .ancestors()
                .skip(1)
                .take_while(|dir| *dir != root.as_path())
                .any(declares_package);
            !in_package || !built.iter().any(|dir| source.starts_with(dir))
        })
        .collect();

    // @step Then each file lies in a workspace member package or a patched crate
    assert!(orphans.is_empty(), "unbuilt Rust sources: {orphans:?}");

    // @step And rust/tests and rust/examples contain no Rust sources
    let stray = sources
        .iter()
        .filter(|source| {
            source.starts_with(root.join("tests")) || source.starts_with(root.join("examples"))
        })
        .count();
    assert_eq!(
        stray, 0,
        "rust/tests and rust/examples must hold no Rust sources"
    );
    Ok(())
}
