//! UPD-001 — cross-platform release pipeline: version bump + workflow.
//!
//! Feature: spec/features/cross-platform-release-pipeline-v0-10-0-via-github-actions.feature
//!
//! Scenarios covered:
//!   - "Workspace version is bumped to 0.10.1"
//!   - "Release workflow builds all five targets"
//!   - "Release is published with self-updater-compatible assets"
//!   - "Release job is blocked when any build fails"
//!   - "Release binary reports the new version"
//!   - "Linux binary runs on old glibc distros"
//!   - "Windows archive is a valid zip"
//!   - "Windows ARM64 user runs the prebuilt binary"
//!
//! The workflow file is validated structurally (YAML parse + matrix
//! inspection) — the actual 5-target CI run is verified by pushing the
//! release tag (manual step, UPD-001 Definition of Done).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::process::Command;

mod common;

use common::{codelet_root, fspec_bin, project_root};

/// (target, runner OS, build tool, archive extension)
///
/// Cross-compilation pipeline: Windows targets are built with cargo-xwin
/// on ubuntu-24.04, Linux targets with cargo-zigbuild on ubuntu-24.04,
/// and the macOS target natively on macos-latest.
const EXPECTED_TARGETS: [(&str, &str, &str, &str); 5] = [
    ("x86_64-pc-windows-msvc", "ubuntu-24.04", "xwin", "zip"),
    ("aarch64-pc-windows-msvc", "ubuntu-24.04", "xwin", "zip"),
    ("x86_64-unknown-linux-gnu", "ubuntu-24.04", "zigbuild", "tar.gz"),
    ("aarch64-unknown-linux-gnu", "ubuntu-24.04", "zigbuild", "tar.gz"),
    ("aarch64-apple-darwin", "macos-latest", "native", "tar.gz"),
];

/// Extract the `version = "..."` value from the `[workspace.package]`
/// section of `rust/Cargo.toml`.
fn workspace_package_version() -> String {
    let raw = fs::read_to_string(codelet_root().join("Cargo.toml")).expect("read rust/Cargo.toml");
    let Some(start) = raw.find("[workspace.package]") else {
        panic!("[workspace.package] section missing from rust/Cargo.toml");
    };
    let section = &raw[start..];
    // The section is multi-line; find the first `version =` after the header line.
    let header_end = section.find('\n').expect("header newline");
    let body = &section[header_end..];
    let Some(ver_line) = body.lines().find(|l| l.trim_start().starts_with("version =")) else {
        panic!("no version key under [workspace.package]");
    };
    ver_line
        .split('"')
        .nth(1)
        .map(str::to_string)
        .expect("quoted version value")
}

fn workflow_yaml() -> serde_yaml::Value {
    let path = project_root().join(".github/workflows/release.yml");
    let raw = fs::read_to_string(&path).expect("read .github/workflows/release.yml");
    serde_yaml::from_str(&raw).expect("parse release.yml as YAML")
}

// =============================================================================
// Scenario: Workspace version is bumped to 0.10.1
// =============================================================================

#[test]
fn scenario_workspace_version_is_bumped_to_0_10_1() {
    // @step Given the workspace is at version 0.1.0
    // (precondition — the repo history shows 0.1.0; nothing to assert here)

    // @step When the version bump for v0.10.1 is applied
    let version = workspace_package_version();

    // @step Then rust/Cargo.toml [workspace.package] version is 0.10.1
    assert_eq!(
        version, "0.10.1",
        "[workspace.package] version must be 0.10.1 after the bump"
    );

    // @step And Cargo.lock reports 0.10.1 for all member crates
    let lock = fs::read_to_string(codelet_root().join("Cargo.lock")).expect("read Cargo.lock");
    // Every workspace member package entry must carry version 0.10.1.
    for name in ["codelet-fspec", "codelet-fspec-core", "codelet-fspec-tui", "codelet-core"] {
        let Some(start) = lock.find(&format!("\nname = \"{name}\"\n")) else {
            panic!("workspace member {name} missing from Cargo.lock");
        };
        let entry = &lock[start..start + 200];
        assert!(
            entry.contains("version = \"0.10.1\""),
            "Cargo.lock entry for {name} must report version 0.10.1, got: {entry}"
        );
    }
}

// =============================================================================
// Scenario: Release workflow builds all five targets
// =============================================================================

#[test]
fn scenario_release_workflow_builds_all_five_targets() {
    // @step Given a git tag matching v* is pushed
    let wf = workflow_yaml();
    let on = wf.get("on").expect("workflow must have an `on` trigger");
    let tags = on
        .get("push")
        .and_then(|p| p.get("tags"))
        .and_then(|t| t.as_sequence())
        .expect("`on.push.tags` sequence");
    assert!(
        tags.iter().any(|t| t.as_str() == Some("v*")),
        "workflow must trigger on tags matching v*: {tags:?}"
    );

    // @step When the release workflow runs
    let jobs = wf.get("jobs").and_then(|j| j.as_mapping()).expect("jobs mapping");
    let build = jobs.get("build").expect("a `build` job must exist");
    let matrix = build
        .get("strategy")
        .and_then(|s| s.get("matrix"))
        .and_then(|m| m.get("include"))
        .and_then(|i| i.as_sequence())
        .expect("strategy.matrix.include sequence");

    // @step Then one build job runs per target (win x86_64, win arm64, linux x86_64, linux arm64, mac arm64)
    assert_eq!(
        matrix.len(),
        5,
        "exactly 5 matrix entries (rule [0]: exactly 5 targets)"
    );
    for (target, os, tool, _archive) in EXPECTED_TARGETS {
        let entry = matrix
            .iter()
            .find(|e| e.get("target").and_then(|t| t.as_str()) == Some(target))
            .unwrap_or_else(|| panic!("matrix entry for {target} missing"));
        assert_eq!(
            entry.get("os").and_then(|o| o.as_str()),
            Some(os),
            "target {target} must build on {os} (rule [4])"
        );
        assert_eq!(
            entry.get("tool").and_then(|t| t.as_str()),
            Some(tool),
            "target {target} must use build tool {tool} (rule [4])"
        );
    }

    // @step And Windows targets are cross-compiled with cargo-xwin on ubuntu-24.04
    // @step And Linux targets are cross-compiled with cargo-zigbuild on ubuntu-24.04 with TARGET_GLIBC_VERSION=2.17
    // @step And the macOS target is built natively on macos-latest
    let raw = fs::read_to_string(project_root().join(".github/workflows/release.yml"))
        .expect("read workflow");
    assert!(
        raw.contains("cargo xwin build"),
        "Windows targets must be built with cargo-xwin (rule [4])"
    );
    assert!(
        raw.contains("cargo zigbuild"),
        "Linux targets must be built with cargo-zigbuild (rule [4])"
    );
    assert!(
        raw.contains("TARGET_GLIBC_VERSION"),
        "Linux targets must pin TARGET_GLIBC_VERSION for old-glibc compatibility (rule [4])"
    );

    // @step And every build uses the release-slim profile and the pinned 1.95.0 toolchain
    assert!(
        raw.contains("release-slim"),
        "build step must use the release-slim profile (rule [4])"
    );
    assert!(
        raw.contains("1.95.0"),
        "workflow must pin the 1.95.0 toolchain (rule [4])"
    );
}

// =============================================================================
// Scenario: Release is published with self-updater-compatible assets
// =============================================================================

#[test]
fn scenario_release_is_published_with_self_updater_compatible_assets() {
    // @step Given all five build jobs succeeded
    // (structural precondition — the release job runs after `needs: build`)

    // @step When the release job completes
    // (structural — the release job's contents are asserted below)
    let raw = fs::read_to_string(project_root().join(".github/workflows/release.yml"))
        .expect("read workflow");

    // @step Then a GitHub Release exists with exactly five assets
    assert!(
        raw.contains("softprops/action-gh-release"),
        "release job must use softprops/action-gh-release (rule [5])"
    );

    // @step And Windows assets are named fspec-<target>.zip and unix assets fspec-<target>.tar.gz
    // The packaging step must produce fspec-<target>.<ext> names (the
    // UPD-002 self-updater contract).
    for (target, _os, _tool, ext) in EXPECTED_TARGETS {
        let expected = format!("fspec-{target}.{ext}");
        assert!(
            raw.contains(&expected) || raw.contains("fspec-${{ matrix.target }}"),
            "workflow must package asset {expected} (asset-naming contract)"
        );
    }
}

// =============================================================================
// Scenario: Release job is blocked when any build fails
// =============================================================================

#[test]
fn scenario_release_job_is_blocked_when_any_build_fails() {
    // @step Given one of the five build jobs fails
    // (structural precondition — `needs` is the blocking mechanism)

    // @step When the workflow reaches the release stage
    let wf = workflow_yaml();
    let jobs = wf.get("jobs").and_then(|j| j.as_mapping()).expect("jobs mapping");
    let release = jobs.get("release").expect("a `release` job must exist");

    // @step Then no GitHub Release is created or updated
    let needs = release
        .get("needs")
        .and_then(|n| n.as_str())
        .map(str::to_string)
        .or_else(|| {
            release
                .get("needs")
                .and_then(|n| n.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                        .join(",")
                })
        })
        .expect("release job must declare `needs`");
    assert!(
        needs.contains("build"),
        "release job must `needs: build` so a failed build blocks publication (rule [4])"
    );
}

// =============================================================================
// Scenario: Windows ARM64 user runs the prebuilt binary
// =============================================================================

#[test]
fn scenario_windows_arm64_user_runs_the_prebuilt_binary() {
    // @step Given a user on Windows ARM64 downloads fspec-aarch64-pc-windows-msvc.zip
    // (structural precondition — the workflow must package the aarch64
    // Windows asset as a zip; the download itself is a manual smoke test
    // per UPD-001 Definition of Done)
    let raw = fs::read_to_string(project_root().join(".github/workflows/release.yml"))
        .expect("read workflow");

    // @step When they extract and run fspec.exe
    // The Windows packaging step must include the .exe in the zip so the
    // extracted artifact is a runnable fspec.exe.
    assert!(
        raw.contains("zip -j fspec-${{ matrix.target }}.zip fspec.exe"),
        "Windows packaging step must zip fspec.exe into fspec-<target>.zip"
    );

    // @step Then it works without installing a Rust toolchain
    // (structural assertion — the asset is a self-contained release-slim
    // cargo build; the user needs no toolchain, only the binary)
    let wf = workflow_yaml();
    let build = wf
        .get("jobs")
        .and_then(|j| j.as_mapping())
        .and_then(|j| j.get("build"))
        .expect("build job");
    let matrix = build
        .get("strategy")
        .and_then(|s| s.get("matrix"))
        .and_then(|m| m.get("include"))
        .and_then(|i| i.as_sequence())
        .expect("matrix include");
    assert!(
        matrix
            .iter()
            .any(|e| e.get("target").and_then(|t| t.as_str())
                == Some("aarch64-pc-windows-msvc")),
        "aarch64-pc-windows-msvc must be a first-class matrix target"
    );
    assert!(
        raw.contains("cargo build --profile release-slim"),
        "the shipped artifact must be a self-contained release-slim build"
    );
}

#[test]
fn scenario_release_binary_reports_the_new_version() {
    // @step Given the mac aarch64 release binary is extracted
    // (local equivalent: the built fspec binary carries the workspace
    // version via clap `#[command(version)]`)

    // @step When fspec --version is run
    let output = Command::new(fspec_bin())
        .arg("--version")
        .output()
        .expect("spawn fspec --version");

    // @step Then it prints fspec 0.10.1
    assert!(output.status.success(), "fspec --version must exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "fspec 0.10.1",
        "fspec --version must print the bumped workspace version"
    );
}

// =============================================================================
// Scenario: Linux binary runs on old glibc distros
// =============================================================================

#[test]
fn scenario_linux_binary_runs_on_old_glibc_distros() {
    // @step Given the fspec-x86_64-unknown-linux-gnu release asset is downloaded
    // (structural precondition — the CI build pins the glibc floor; the
    // actual old-distro smoke test is a manual step per UPD-001 DoD)
    let raw = fs::read_to_string(project_root().join(".github/workflows/release.yml"))
        .expect("read workflow");

    // @step When the binary is run on a system with glibc 2.17
    // The zigbuild invocation must pin TARGET_GLIBC_VERSION so the linker
    // refuses to emit references to symbols newer than the pinned floor.
    assert!(
        raw.contains("2.17"),
        "Linux cross-builds must pin TARGET_GLIBC_VERSION=2.17 (rule [4])"
    );

    // @step Then it runs without "version GLIBC_2.39 not found" or similar symbol version errors
    // (structural assertion — with the pinned floor, zig's bundled linker
    // errors at build time if any dependency requires a newer glibc)
    let wf = workflow_yaml();
    let build = wf
        .get("jobs")
        .and_then(|j| j.as_mapping())
        .and_then(|j| j.get("build"))
        .expect("build job");
    let matrix = build
        .get("strategy")
        .and_then(|s| s.get("matrix"))
        .and_then(|m| m.get("include"))
        .and_then(|i| i.as_sequence())
        .expect("matrix include");
    for target in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
        assert!(
            matrix
                .iter()
                .any(|e| e.get("target").and_then(|t| t.as_str()) == Some(target)
                    && e.get("tool").and_then(|t| t.as_str()) == Some("zigbuild")),
            "{target} must be built with cargo-zigbuild so the glibc floor applies"
        );
    }
}

// =============================================================================
// Scenario: Windows archive is a valid zip
// =============================================================================

#[test]
fn scenario_windows_archive_is_a_valid_zip() {
    // @step Given the fspec-x86_64-pc-windows-msvc release asset is downloaded
    // (structural precondition — the packaging step is asserted below)
    let raw = fs::read_to_string(project_root().join(".github/workflows/release.yml"))
        .expect("read workflow");

    // @step When unzip -t is run on the archive
    // The packaging step must use the real `zip` binary (NOT `tar -a`,
    // which produces a tar archive with a .zip extension) and verify the
    // result with `unzip -t` before uploading.
    assert!(
        raw.contains("zip -j fspec-${{ matrix.target }}.zip fspec.exe"),
        "Windows packaging must use the zip binary to create a real ZIP archive (rule [7])"
    );
    assert!(
        raw.contains("unzip -t"),
        "packaging must verify the zip archive with unzip -t before upload (rule [7])"
    );
    assert!(
        !raw.contains("tar -a -cf"),
        "tar -a must not be used to create .zip assets (it produces tar archives)"
    );

    // @step Then it reports no errors and contains fspec.exe
    // (the release job re-verifies every archive before publishing)
    assert!(
        raw.contains("Verify all archives"),
        "release job must re-verify all archives before publishing (rule [7])"
    );
}
