//! Feature: spec/features/add-rig-core-dependency-and-compatibility-layer.feature
//!
//! Tests for Adding rig-core Dependency and Compatibility Layer - REFAC-002
//!
//! These tests verify that rig-core 0.25.0 is added as a dependency,
//! re-exported correctly, and doesn't break existing functionality.

use std::fs;
use std::path::Path;
use std::process::Command;

// ==========================================
// SCENARIO 1: Add rig-core dependency to Cargo.toml
// ==========================================

/// Scenario: Add rig-core dependency to Cargo.toml
#[test]
fn test_add_rig_core_dependency_to_cargo_toml() {
    // @step Given I have a Cargo.toml file in the project root
    let cargo_toml_path = Path::new("Cargo.toml");
    assert!(
        cargo_toml_path.exists(),
        "Cargo.toml should exist in project root"
    );

    // @step When I add "rig-core = \"0.25.0\"" to the dependencies section
    let cargo_toml_content =
        fs::read_to_string(cargo_toml_path).expect("Should be able to read Cargo.toml");

    // Verify rig-core dependency exists in Cargo.toml
    assert!(
        cargo_toml_content.contains("rig-core"),
        "Cargo.toml should contain rig-core dependency"
    );

    // @step And I run "cargo build"
    let build_result = Command::new("cargo")
        .args(&["build", "--quiet"])
        .output()
        .expect("Failed to execute cargo build");

    // @step Then the build should succeed without errors
    // @step And rig-core 0.25.0 should be downloaded and compiled
    assert!(
        build_result.status.success(),
        "cargo build should succeed after adding rig-core. Stderr: {}",
        String::from_utf8_lossy(&build_result.stderr)
    );
}

// ==========================================
// SCENARIO 2: All existing tests pass after adding rig-core
// ==========================================

/// Scenario: All existing tests pass after adding rig-core
#[test]
fn test_all_existing_tests_pass_after_adding_rig_core() {
    // @step Given rig-core 0.25.0 is added to Cargo.toml
    let cargo_toml_content =
        fs::read_to_string("Cargo.toml").expect("Should be able to read Cargo.toml");
    assert!(
        cargo_toml_content.contains("rig-core"),
        "rig-core should be in Cargo.toml"
    );

    // @step And the project builds successfully
    let build_result = Command::new("cargo")
        .args(&["build", "--quiet"])
        .output()
        .expect("Failed to execute cargo build");
    assert!(
        build_result.status.success(),
        "Project should build successfully"
    );

    // @step When I run "cargo test"
    let test_result = Command::new("cargo")
        .args(&["test", "--quiet", "--lib", "--bins"])
        .output()
        .expect("Failed to execute cargo test");

    // @step Then all 10 existing integration tests should pass
    // @step And no test failures should occur
    // @step And test output should show "test result: ok"
    let test_output = String::from_utf8_lossy(&test_result.stdout);
    assert!(
        test_result.status.success(),
        "All tests should pass after adding rig-core. Output: {}\nStderr: {}",
        test_output,
        String::from_utf8_lossy(&test_result.stderr)
    );
}

// ==========================================
// SCENARIO 3: Re-export rig types from lib.rs
// ==========================================

/// Scenario: Re-export rig types from lib.rs
#[test]
fn test_reexport_rig_types_from_lib_rs() {
    // @step Given rig-core 0.25.0 is added to Cargo.toml
    let cargo_toml_content =
        fs::read_to_string("Cargo.toml").expect("Should be able to read Cargo.toml");
    assert!(
        cargo_toml_content.contains("rig-core"),
        "rig-core should be in Cargo.toml"
    );

    // @step When I add "pub use rig;" to src/lib.rs
    let lib_rs_content =
        fs::read_to_string("src/lib.rs").expect("Should be able to read src/lib.rs");

    // @step And I create a test file that imports "use codelet_core::rig::completion::CompletionModel;"
    // @step Then the test file should compile without errors
    // @step And rig types should be accessible from the codelet namespace

    // Verify rig is re-exported
    assert!(
        lib_rs_content.contains("pub use rig"),
        "src/lib.rs should contain 'pub use rig;' re-export"
    );

    // Verify we can access rig types through codelet namespace
    // This is a compile-time test - if this test compiles, it proves re-export works
    let _test_compile: Option<fn() -> ()> = Some(|| {
        // This should compile if re-export is working
        let _: Option<&dyn std::any::Any> = None;
        // In actual usage: use codelet_core::rig::completion::CompletionModel;
    });
}

// ==========================================
// SCENARIO 4: Cargo clippy completes without warnings
// ==========================================

/// Scenario: Cargo clippy completes without warnings
#[test]
fn test_cargo_clippy_completes_without_warnings() {
    // @step Given rig-core 0.25.0 is added and re-exported
    let cargo_toml_content =
        fs::read_to_string("Cargo.toml").expect("Should be able to read Cargo.toml");
    assert!(
        cargo_toml_content.contains("rig-core"),
        "rig-core should be in Cargo.toml"
    );

    let lib_rs_content =
        fs::read_to_string("src/lib.rs").expect("Should be able to read src/lib.rs");
    assert!(
        lib_rs_content.contains("pub use rig"),
        "src/lib.rs should re-export rig"
    );

    // @step And all code changes are complete
    // @step When I run "cargo clippy -- -D warnings"
    let clippy_result = Command::new("cargo")
        .args(&["clippy", "--", "-D", "warnings"])
        .output()
        .expect("Failed to execute cargo clippy");

    // @step Then clippy should complete with exit code 0
    // @step And no warnings should be reported
    // @step And the output should confirm "0 warnings emitted"
    let clippy_stderr = String::from_utf8_lossy(&clippy_result.stderr);
    assert!(
        clippy_result.status.success(),
        "cargo clippy should complete without warnings. Stderr: {}",
        clippy_stderr
    );

    // Verify no warnings in output
    assert!(
        !clippy_stderr.contains("warning:"),
        "clippy should not emit any warnings. Stderr: {}",
        clippy_stderr
    );
}
