//! Feature: spec/features/harness-workflow-guidance-references-astgrep-tool.feature
//!
//! CLI-015 test, moved out of fspec_workflow_guidance.rs's inline tests so each
//! test file maps one feature (COV-075).

use super::*;

#[test]
fn test_harness_guidance_references_astgrep_not_research_ast() {
    // @step Given the harness-only FSPEC workflow guidance constant
    // @step When I inspect the guidance text
    let guidance = get_fspec_workflow_guidance();
    // @step Then it references `AstGrep` for code search during discovery
    assert!(guidance.contains("**Research first** using the AstGrep tool:"));
    // @step And it does not reference `research` with the `ast` tool
    assert!(!guidance.contains("\"tool\": \"ast\""));
    assert!(!guidance.contains("--tool=ast"));
}
