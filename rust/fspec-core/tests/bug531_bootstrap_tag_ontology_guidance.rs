//! BUG-531 — the bootstrap tag ontology guidance names real commands.
//!
//! The Foundation Event Storm section of the bootstrap text told agents to run
//! `fspec derive-tags-from-foundation`, which never existed. It now gives the
//! register-tag + generate-tags-md workflow.
//!
//! Feature: spec/features/bootstrap-tag-ontology-guidance.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::json;

const SECTION: &str = "### Integration with Tag Ontology (EXMAP-004)";

fn bootstrap(project_root: &Path) -> String {
    let response = dispatch_command(DispatchRequest {
        command: "bootstrap".to_string(),
        args_json: "{}".to_string(),
        project_root: project_root.to_path_buf(),
    });
    assert!(response.success, "bootstrap failed: {response:?}");
    response.data
}

/// The text from the section heading up to the next `### ` heading.
fn section(output: &str) -> &str {
    let start = output.find(SECTION).expect("tag ontology section present");
    let body = &output[start + SECTION.len()..];
    let end = body.find("\n### ").unwrap_or(body.len());
    &body[..end]
}

#[test]
fn the_tag_ontology_section_gives_the_register_tag_and_generate_tags_md_workflow() {
    // @step Given a project whose Foundation Event Storm has bounded contexts
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(root.path().join("spec")).expect("mkdir spec");
    let foundation = json!({
        "eventStorm": {"level": "big_picture", "items": [
            {"id": 1, "type": "bounded_context", "text": "Work Management", "deleted": false},
            {"id": 2, "type": "bounded_context", "text": "Specification", "deleted": false}
        ], "nextItemId": 3}
    });
    std::fs::write(
        root.path().join("spec/foundation.json"),
        serde_json::to_string_pretty(&foundation).expect("serialize foundation"),
    )
    .expect("write foundation.json");

    // @step When the agent runs fspec bootstrap and reads the section "Integration with Tag Ontology (EXMAP-004)"
    let output = bootstrap(root.path());
    let text = section(&output);

    // @step Then the section tells it to run fspec register-tag with the "Component Tags" category for each bounded context
    assert!(
        text.contains("fspec register-tag @work-management \"Component Tags\""),
        "register-tag per bounded context missing:\n{text}"
    );
    assert!(
        text.contains("bounded context"),
        "the section must tie the tags to bounded contexts:\n{text}"
    );

    // @step And the section tells it to run fspec generate-tags-md
    assert!(
        text.contains("fspec generate-tags-md"),
        "generate-tags-md missing:\n{text}"
    );
}

#[test]
fn the_bootstrap_output_never_names_derive_tags_from_foundation() {
    // @step Given any project
    let root = tempfile::tempdir().expect("tempdir");

    // @step When the agent runs fspec bootstrap
    let output = bootstrap(root.path());

    // @step Then the output does not contain "derive-tags-from-foundation"
    assert!(
        !output.contains("derive-tags-from-foundation"),
        "bootstrap still names the phantom command"
    );
}
