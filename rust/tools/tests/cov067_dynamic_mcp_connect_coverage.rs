//! COV-067 — dynamic-mcp-connect coverage maps the tests that carry each
//! scenario.
//!
//! Feature: spec/features/dynamic-mcp-connect-coverage-is-verified.feature

use codelet_test_helpers::coverage_steps::unmatched_steps;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn every_dynamic_mcp_connect_scenario_maps_tests_carrying_all_of_its_steps() -> TestResult {
    // @step Given the dynamic-mcp-connect feature and its coverage file
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or("the repository root is above rust/tools")?;
    let feature = repository.join("spec/features/dynamic-mcp-connect.feature");
    // @step When each scenario's mapped test ranges are read
    let unmatched = unmatched_steps(repository, &feature)?;
    // @step Then every step of the scenario appears as an @step comment in those ranges
    assert!(unmatched.is_empty(), "unmatched steps: {unmatched:#?}");
    Ok(())
}
