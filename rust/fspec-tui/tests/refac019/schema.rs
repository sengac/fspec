//! Feature: spec/features/track-nested-rpc-schema-declaration-inputs.feature

use std::path::Path;
use std::process::Command;

use super::*;

fn run(binary: &Path, directory: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new(binary).current_dir(directory).output()?;
    assert!(
        output.status.success(),
        "build script failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

fn fingerprint(output: &str) -> Result<&str, Box<dyn std::error::Error>> {
    output
        .lines()
        .find(|line| line.starts_with("cargo:rustc-env=FSPEC_RPC_SCHEMA="))
        .ok_or_else(|| "missing schema fingerprint".into())
}

#[test]
fn schema_fingerprints_track_nested_declaration_inputs() -> TestResult {
    // @step Given an isolated RPC workspace with nested service and type declarations
    let temporary = codelet_test_helpers::setup_temp_directory()?;
    let rpc = temporary.path().join("rpc");
    let types = temporary.path().join("rpc-types");
    std::fs::create_dir_all(rpc.join("src/service"))?;
    std::fs::create_dir_all(types.join("src/nested"))?;
    std::fs::write(
        rpc.join("src/lib.rs"),
        "include!(\"service/methods.rs\");\n",
    )?;
    std::fs::write(
        rpc.join("src/service/methods.rs"),
        "pub trait Service { fn first(); }\n",
    )?;
    std::fs::write(types.join("src/lib.rs"), "mod nested;\n")?;
    std::fs::write(types.join("src/nested/mod.rs"), "pub struct First;\n")?;
    let binary = temporary
        .path()
        .join(format!("build{}", std::env::consts::EXE_SUFFIX));
    let compiled = Command::new("rustc")
        .arg("--edition=2021")
        .arg(workspace().join("rpc/build.rs"))
        .arg("-o")
        .arg(&binary)
        .output()?;
    assert!(
        compiled.status.success(),
        "build script compilation failed: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let initial = run(&binary, &rpc)?;
    // @step When either nested declaration changes without changing the root files
    std::fs::write(
        rpc.join("src/service/methods.rs"),
        "pub trait Service { fn second(); }\n",
    )?;
    let service_changed = run(&binary, &rpc)?;
    std::fs::write(types.join("src/nested/mod.rs"), "pub struct Second;\n")?;
    let types_changed = run(&binary, &rpc)?;
    // @step Then its schema fingerprint changes and Cargo rerun tracking names the input
    assert_ne!(
        fingerprint(&initial)?,
        fingerprint(&service_changed)?,
        "nested service declarations must participate in the schema hash"
    );
    assert_ne!(
        fingerprint(&service_changed)?,
        fingerprint(&types_changed)?,
        "nested type declarations must participate in the schema hash"
    );
    for output in [&service_changed, &types_changed] {
        assert!(output.contains("cargo:rerun-if-changed=src/service/methods.rs"));
        assert!(output.contains("cargo:rerun-if-changed=../rpc-types/src/nested/mod.rs"));
    }
    Ok(())
}
