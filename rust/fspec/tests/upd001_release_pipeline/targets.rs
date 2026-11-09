//! Feature: spec/features/cross-platform-release-pipeline-v0-10-0-via-github-actions.feature
//!
//! BUG-293: build-matrix scenarios — the five release targets and the
//! static-musl x86_64 Linux asset, asserted on .github/workflows/release.yml.

use super::*;

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
    let matrix = build_matrix(&wf);

    // @step Then one build job runs per target (win x86_64, win arm64, linux x86_64, linux arm64, mac arm64)
    assert_eq!(
        matrix.len(),
        5,
        "exactly 5 matrix entries (rule [0]: exactly 5 targets)"
    );
    for (target, os, tool, _archive) in EXPECTED_TARGETS {
        let entry = matrix_entry(&matrix, target);
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

    // @step And x86_64 Windows is cross-compiled with cargo-xwin on ubuntu-24.04 and arm64 Windows builds natively on windows-11-arm
    // @step And x86_64 Linux is built as a static musl binary on ubuntu-24.04 and arm64 Linux builds natively on ubuntu-24.04-arm
    // @step And the macOS target is built natively on macos-latest
    let raw = workflow_raw();
    assert!(
        raw.contains("cargo xwin build"),
        "x86_64 Windows must be built with cargo-xwin (rule [4])"
    );
    assert!(
        raw.contains("x86_64-unknown-linux-musl"),
        "x86_64 Linux must build the static musl target (rule [4])"
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
// Scenario: x86_64 Linux release asset is a static musl build with no glibc
// dependency
// =============================================================================

#[test]
fn scenario_x86_64_linux_release_asset_is_a_static_musl_build() {
    // @step Given the release workflow packages the fspec-x86_64-unknown-linux-gnu asset
    let wf = workflow_yaml();
    let matrix = build_matrix(&wf);
    let x64_linux = matrix_entry(&matrix, "x86_64-unknown-linux-gnu");
    assert_eq!(
        x64_linux.get("archive").and_then(|a| a.as_str()),
        Some("tar.gz"),
        "the x86_64 Linux asset is fspec-x86_64-unknown-linux-gnu.tar.gz"
    );

    // @step When the x86_64 Linux build job is inspected
    let field = |key: &str| x64_linux.get(key).and_then(|v| v.as_str());

    // @step Then it builds the x86_64-unknown-linux-musl target with the musl tool on ubuntu-24.04
    assert_eq!(field("build_target"), Some("x86_64-unknown-linux-musl"));
    assert_eq!(field("tool"), Some("musl"));
    assert_eq!(field("os"), Some("ubuntu-24.04"));

    // @step And the C runtime is linked statically with crt-static so the binary needs no glibc symbol versions
    let steps = wf
        .get("jobs")
        .and_then(|j| j.get("build"))
        .and_then(|b| b.get("steps"))
        .and_then(|s| s.as_sequence())
        .expect("build job steps");
    let build_step = steps
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Build (release-slim)"))
        .expect("`Build (release-slim)` step");
    let rustflags = build_step
        .get("env")
        .and_then(|e| e.get("RUSTFLAGS"))
        .and_then(|r| r.as_str())
        .expect("build step RUSTFLAGS");
    assert!(
        rustflags.contains("matrix.tool == 'musl'") && rustflags.contains("+crt-static"),
        "musl builds must link the C runtime statically, got RUSTFLAGS: {rustflags}"
    );
}
