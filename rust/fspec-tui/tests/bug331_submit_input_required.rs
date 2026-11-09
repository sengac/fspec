//! BUG-331: every TUI backend states how it delivers each input mode.
//!
//! Feature: spec/features/fspec-backend-submit-input-is-required.feature

use codelet_test_helpers::TestResult;
use std::path::Path;

#[test]
fn the_fspec_backend_trait_has_no_default_submit_input() -> TestResult {
    // @step Given the FspecBackend trait declaration
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/transport/backend/subscriptions.rs"),
    )?;

    // @step When its submit_input method is read
    let start = source
        .find("async fn submit_input(")
        .ok_or("submit_input is not declared")?;
    let declaration = &source[start..];
    let body = declaration.find('{').unwrap_or(usize::MAX);
    let end = declaration
        .find(';')
        .ok_or("the declaration does not end")?;

    // @step Then it is a required method without a default body
    assert!(
        end < body,
        "submit_input has a default body: {declaration:.200}"
    );
    Ok(())
}
