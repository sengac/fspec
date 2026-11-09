//! Feature: spec/features/cross-platform-release-pipeline-v0-10-0-via-github-actions.feature
//!
//! BUG-293: version scenarios — every workspace member inherits the
//! `[workspace.package]` version (asserted against each member manifest and
//! Cargo.lock), and the binary the release jobs build prints that version.

use super::*;

/// `[workspace] members` of `rust/Cargo.toml`, in declaration order.
fn workspace_members(manifest: &str) -> Vec<String> {
    let start = manifest
        .find("members = [")
        .expect("[workspace] members list in rust/Cargo.toml");
    manifest[start..]
        .lines()
        .skip(1)
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .take_while(|line| !line.starts_with(']'))
        .filter_map(|line| line.strip_prefix('"')?.split('"').next())
        .map(str::to_string)
        .collect()
}

/// The `[package]` table of a member manifest (up to the next table).
fn package_section(manifest: &str) -> &str {
    let start = manifest.find("[package]").expect("[package] section");
    let body = &manifest[start + "[package]".len()..];
    body.find("\n[").map_or(body, |end| &body[..end])
}

// =============================================================================
// Scenario: Every workspace crate reports the workspace version
// =============================================================================

#[test]
fn scenario_every_workspace_crate_reports_the_workspace_version() {
    // @step Given rust/Cargo.toml declares a [workspace.package] version
    let version = workspace_package_version();
    assert!(
        !version.is_empty(),
        "[workspace.package] version must be set"
    );
    let root = codelet_root();
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("read rust/Cargo.toml");
    let members = workspace_members(&manifest);
    assert!(
        !members.is_empty(),
        "rust/Cargo.toml must list workspace members"
    );

    // @step When Cargo.lock is inspected
    let lock = fs::read_to_string(root.join("Cargo.lock")).expect("read Cargo.lock");

    // @step Then every workspace member crate reports the workspace version
    for member in &members {
        let crate_manifest = fs::read_to_string(root.join(member).join("Cargo.toml"))
            .unwrap_or_else(|e| panic!("read rust/{member}/Cargo.toml: {e}"));
        let package = package_section(&crate_manifest);
        assert!(
            package
                .lines()
                .any(|line| line.trim() == "version.workspace = true"),
            "rust/{member} must inherit the workspace version"
        );
        let name = package
            .lines()
            .find_map(|line| line.trim().strip_prefix("name = "))
            .map(|value| value.trim_matches('"'))
            .unwrap_or_else(|| panic!("rust/{member}/Cargo.toml must name its package"));
        let entry = format!("\nname = \"{name}\"\nversion = \"{version}\"\n");
        assert!(
            lock.contains(&entry),
            "Cargo.lock must record {name} (rust/{member}) at workspace version {version}"
        );
    }
}

// =============================================================================
// Scenario: Binary built by the release jobs reports the workspace version
// =============================================================================

#[test]
fn scenario_binary_built_by_the_release_jobs_reports_the_workspace_version() {
    // @step Given every release build job builds the fspec binary from the codelet-fspec package
    let raw = workflow_raw();
    for build in ["cargo build", "cargo xwin build"] {
        let command =
            format!("{build} --profile release-slim --target ${{{{ matrix.build_target }}}} -p codelet-fspec");
        assert!(
            raw.contains(&command),
            "release build step must run `{command}`"
        );
    }

    // @step When fspec --version is run on a binary built from that package
    let output = Command::new(fspec_bin())
        .arg("--version")
        .output()
        .expect("spawn fspec --version");

    // @step Then it prints fspec followed by the workspace version
    assert!(output.status.success(), "fspec --version must exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("fspec {}", workspace_package_version()),
        "fspec --version must print the workspace version"
    );
}
