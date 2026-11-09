//! BUG-382 — the Rust workspace is rustfmt-formatted and checked.
//!
//! Feature: spec/features/workspace-is-rustfmt-formatted.feature

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_workspace_passes_the_format_check() -> TestResult {
    // @step Given the rust workspace
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    // @step When cargo fmt --all -- --check runs
    let cargo =
        std::env::var("CARGO").map_err(|error| format!("cargo is not available: {error}"))?;
    let output = std::process::Command::new(cargo)
        .args(["fmt", "--all", "--", "--check", "-l"])
        .current_dir(&workspace)
        .output()?;
    // @step Then it reports no unformatted file
    let unformatted = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && unformatted.trim().is_empty(),
        "unformatted files:\n{unformatted}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
