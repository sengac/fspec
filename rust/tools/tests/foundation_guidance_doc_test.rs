#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/foundation-discovery-agent-guidance.feature
//
// Tests for the Phase-0 FOUNDATION section of the injected workflow guidance
// doc (rust/tools/src/fspec_workflow_guidance.rs). Each scenario maps to
// exactly one #[test] with @step comments mirroring the Gherkin steps
// verbatim.
//
// RED PHASE: the doc still documents the WRONG arg names, so these tests
// FAIL now.

use codelet_tools::get_fspec_workflow_guidance;

fn foundation_section() -> String {
    let guidance = get_fspec_workflow_guidance();
    // The Phase-0 FOUNDATION section runs from its header up to the next
    // top-level '## Phase 0: DISCOVERY' header.
    let start = guidance
        .find("## Phase 0: FOUNDATION")
        .expect("FOUNDATION section header present");
    let end = guidance
        .find("## Phase 0: DISCOVERY")
        .expect("DISCOVERY section header present");
    guidance[start..end].to_string()
}

#[test]
fn the_workflow_guidance_doc_documents_correct_foundation_argument_names() {
    // @step Given the injected workflow guidance constant
    let section = foundation_section();

    // @step When I inspect the Phase-0 FOUNDATION section
    // @step Then it documents update-foundation with section and content keys
    assert!(
        section.contains("update-foundation\"), args: {\"section\"")
            || section.contains("update-foundation\", args: {\"section\"")
            || (section.contains("\"section\":") && section.contains("\"content\":")),
        "update-foundation must use {{section, content}} keys:\n{section}"
    );

    // @step And it documents add-capability with a name key and NOT a capability key
    assert!(
        section.contains("add-capability\", args: {\"name\""),
        "add-capability must use the name key:\n{section}"
    );
    assert!(
        !section.contains("add-capability\", args: {\"capability\""),
        "add-capability must NOT use the phantom capability key:\n{section}"
    );

    // @step And it documents add-persona with name, description, and goals keys
    assert!(
        section.contains("add-persona\", args: {\"name\"") && section.contains("\"goals\""),
        "add-persona must use {{name, description, goals[]}}:\n{section}"
    );
    assert!(
        !section.contains("\"goal\":"),
        "add-persona must NOT use the singular goal key:\n{section}"
    );

    // @step And it documents add-foundation-bounded-context with a text key
    assert!(
        section.contains("add-foundation-bounded-context\", args: {\"text\""),
        "add-foundation-bounded-context must use the text key:\n{section}"
    );

    // @step And it mentions foundation-status and generate-tags-md
    assert!(
        section.contains("foundation-status"),
        "foundation-status must be documented:\n{section}"
    );
    assert!(
        section.contains("generate-tags-md"),
        "generate-tags-md must replace the phantom command:\n{section}"
    );

    // @step And it does NOT contain 'derive-tags-from-foundation'
    assert!(
        !section.contains("derive-tags-from-foundation"),
        "phantom command must be removed:\n{section}"
    );

    // @step And it does NOT document update-foundation with key and value argument names
    assert!(
        !section.contains("\"key\":") && !section.contains("\"value\":"),
        "update-foundation must NOT use {{key, value}} keys:\n{section}"
    );
}
