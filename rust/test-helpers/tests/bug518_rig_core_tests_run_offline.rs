//! BUG-518 — the patched rig-core's tests run offline with a documented,
//! lock-stable command.
//!
//! Feature: spec/features/patched-rig-core-tests-run-with-the-workspace-commands.feature

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn workspace() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The `[workspace] members = [...]` entries of the root manifest.
fn workspace_members(manifest: &str) -> Vec<String> {
    let Some(start) = manifest.find("members = [") else {
        return Vec::new();
    };
    let list = &manifest[start..];
    let list = &list[..list.find(']').unwrap_or(list.len())];
    list.lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter_map(|entry| entry.strip_prefix('"'))
        .filter_map(|entry| entry.split('"').next())
        .map(str::to_string)
        .collect()
}

#[test]
fn the_patched_rig_core_stays_a_standalone_package_outside_the_workspace() -> TestResult {
    // @step Given the workspace manifest and the patched rig-core manifest
    let workspace = workspace();
    let patch = workspace.join("patches").join("rig-core");

    // @step When the workspace members and the patch's own files are read
    let members = workspace_members(&std::fs::read_to_string(workspace.join("Cargo.toml"))?);
    let patch_manifest = std::fs::read_to_string(patch.join("Cargo.toml"))?;

    // @step Then the patches/rig-core directory is not a workspace member
    assert!(!members.is_empty(), "the workspace lists its members");
    assert!(
        !members.iter().any(|member| member.starts_with("patches/")),
        "patches stay outside the workspace (UPD-001 versions every member at the workspace version): {members:?}"
    );

    // @step And the patch declares its own workspace and keeps its own Cargo.lock
    assert!(
        patch_manifest
            .lines()
            .any(|line| line.trim() == "[workspace]"),
        "patches/rig-core/Cargo.toml must declare its own [workspace]"
    );
    assert!(
        patch.join("Cargo.lock").is_file(),
        "patches/rig-core keeps its own Cargo.lock"
    );
    Ok(())
}

#[test]
fn the_patched_rig_core_lock_resolves_offline_without_being_rewritten() -> TestResult {
    // @step Given the patched rig-core package and its Cargo.lock
    let patch = workspace().join("patches").join("rig-core");
    let lock_before = std::fs::read(patch.join("Cargo.lock"))?;

    // @step When cargo resolves the package with --locked and --offline
    let cargo =
        std::env::var("CARGO").map_err(|error| format!("cargo is not available: {error}"))?;
    let output = std::process::Command::new(cargo)
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(patch.join("Cargo.toml"))
        .output()?;

    // @step Then resolution succeeds
    assert!(
        output.status.success(),
        "rig-core's Cargo.lock is stale or needs the network:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // @step And the lock file is unchanged
    assert_eq!(std::fs::read(patch.join("Cargo.lock"))?, lock_before);
    Ok(())
}
