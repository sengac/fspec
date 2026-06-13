// Feature: spec/features/list-foundation-sections-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `list-foundation-sections` (RPC-246). Each scenario maps to exactly one
// #[test] function with @step comments mirroring the Gherkin steps verbatim.
//
// RED phase: list-foundation-sections is still a NotYetPorted stub (it is
// NOT in `PORTED_COMMANDS`), so every assertion below should fail today --
// dispatch_command returns `success=false` with the canonical
// "not yet ported" error string instead of the expected payload.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-foundation-sections".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ---------- scenarios ----------

#[test]
fn scenario_default_format_is_text() {
    // Scenario: Default format (no format key supplied) is text

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with an empty args object {}
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the DispatchResult.data starts with the exact line 'Foundation Sections (update-foundation field reference)'
    assert!(
        result
            .data
            .starts_with("Foundation Sections (update-foundation field reference)"),
        "expected text header on first line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact line '========================================================='
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "========================================================="),
        "missing exact '=' separator line; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_json_returns_exactly_seven_sections_in_canonical_order() {
    // Scenario: JSON format returns exactly seven sections in canonical order

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let data = parse_data(&result.data);
    let arr = data.as_array().expect("top-level must be an array");

    // @step And the parsed JSON is an array of length 7
    assert_eq!(arr.len(), 7, "expected 7 sections, got {arr:?}");

    // @step And the entries have name values in order projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview
    let expected = [
        "projectName",
        "projectVision",
        "projectType",
        "problemTitle",
        "problemDefinition",
        "problemImpact",
        "solutionOverview",
    ];
    for (i, name) in expected.iter().enumerate() {
        assert_eq!(
            arr[i]["name"].as_str(),
            Some(*name),
            "expected name at index {i} = {name}, got {}",
            arr[i]
        );
    }
}

#[test]
fn scenario_json_emits_canonical_jsonpath_strings() {
    // Scenario: JSON format emits the canonical jsonPath strings for every section

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);
    let arr = data.as_array().expect("array");
    let by_name = |target: &str| -> Value {
        arr.iter()
            .find(|e| e["name"].as_str() == Some(target))
            .cloned()
            .unwrap_or_else(|| panic!("missing section {target}"))
    };

    // @step And the projectName entry has jsonPath='project.name'
    assert_eq!(
        by_name("projectName")["jsonPath"].as_str(),
        Some("project.name")
    );

    // @step And the projectVision entry has jsonPath='project.vision'
    assert_eq!(
        by_name("projectVision")["jsonPath"].as_str(),
        Some("project.vision")
    );

    // @step And the projectType entry has jsonPath='project.projectType'
    assert_eq!(
        by_name("projectType")["jsonPath"].as_str(),
        Some("project.projectType")
    );

    // @step And the problemTitle entry has jsonPath='problemSpace.primaryProblem.title'
    assert_eq!(
        by_name("problemTitle")["jsonPath"].as_str(),
        Some("problemSpace.primaryProblem.title")
    );

    // @step And the problemDefinition entry has jsonPath='problemSpace.primaryProblem.description'
    assert_eq!(
        by_name("problemDefinition")["jsonPath"].as_str(),
        Some("problemSpace.primaryProblem.description")
    );

    // @step And the problemImpact entry has jsonPath='problemSpace.primaryProblem.impact'
    assert_eq!(
        by_name("problemImpact")["jsonPath"].as_str(),
        Some("problemSpace.primaryProblem.impact")
    );

    // @step And the solutionOverview entry has jsonPath='solutionSpace.overview'
    assert_eq!(
        by_name("solutionOverview")["jsonPath"].as_str(),
        Some("solutionSpace.overview")
    );
}

#[test]
fn scenario_json_emits_canonical_constraint_strings() {
    // Scenario: JSON format emits the canonical constraint strings for every section

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);
    let arr = data.as_array().expect("array");
    let by_name = |target: &str| -> Value {
        arr.iter()
            .find(|e| e["name"].as_str() == Some(target))
            .cloned()
            .unwrap_or_else(|| panic!("missing section {target}"))
    };

    // @step And the projectName entry has constraint='freeform string'
    assert_eq!(
        by_name("projectName")["constraint"].as_str(),
        Some("freeform string")
    );

    // @step And the projectVision entry has constraint='freeform string'
    assert_eq!(
        by_name("projectVision")["constraint"].as_str(),
        Some("freeform string")
    );

    // @step And the projectType entry has constraint='freeform string (1-30 characters)'
    assert_eq!(
        by_name("projectType")["constraint"].as_str(),
        Some("freeform string (1-30 characters)")
    );

    // @step And the problemTitle entry has constraint='freeform string'
    assert_eq!(
        by_name("problemTitle")["constraint"].as_str(),
        Some("freeform string")
    );

    // @step And the problemDefinition entry has constraint='freeform string'
    assert_eq!(
        by_name("problemDefinition")["constraint"].as_str(),
        Some("freeform string")
    );

    // @step And the problemImpact entry has constraint='enum: high, medium, low'
    assert_eq!(
        by_name("problemImpact")["constraint"].as_str(),
        Some("enum: high, medium, low")
    );

    // @step And the solutionOverview entry has constraint='freeform string'
    assert_eq!(
        by_name("solutionOverview")["constraint"].as_str(),
        Some("freeform string")
    );
}

#[test]
fn scenario_json_omits_examples_field_for_sections_without_examples() {
    // Scenario: JSON format omits the examples field for sections without examples

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);
    let arr = data.as_array().expect("array");
    let by_name = |target: &str| -> Value {
        arr.iter()
            .find(|e| e["name"].as_str() == Some(target))
            .cloned()
            .unwrap_or_else(|| panic!("missing section {target}"))
    };

    // @step And the projectType entry has examples=['cli-tool','web-app','saas-platform']
    let pt = by_name("projectType");
    let ex = pt["examples"]
        .as_array()
        .expect("projectType.examples array");
    assert_eq!(ex.len(), 3, "expected 3 examples, got {ex:?}");
    assert_eq!(ex[0].as_str(), Some("cli-tool"));
    assert_eq!(ex[1].as_str(), Some("web-app"));
    assert_eq!(ex[2].as_str(), Some("saas-platform"));

    // @step And the projectName entry does NOT contain a top-level 'examples' field
    assert!(
        by_name("projectName").get("examples").is_none(),
        "projectName must omit examples field"
    );

    // @step And the projectVision entry does NOT contain a top-level 'examples' field
    assert!(by_name("projectVision").get("examples").is_none());

    // @step And the problemTitle entry does NOT contain a top-level 'examples' field
    assert!(by_name("problemTitle").get("examples").is_none());

    // @step And the problemDefinition entry does NOT contain a top-level 'examples' field
    assert!(by_name("problemDefinition").get("examples").is_none());

    // @step And the problemImpact entry does NOT contain a top-level 'examples' field
    assert!(by_name("problemImpact").get("examples").is_none());

    // @step And the solutionOverview entry does NOT contain a top-level 'examples' field
    assert!(by_name("solutionOverview").get("examples").is_none());
}

#[test]
fn scenario_json_uses_two_space_indented_pretty_print() {
    // Scenario: JSON format uses two-space indented pretty-printed payload

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the DispatchResult.data starts with the exact string "[\n  {\n    \"name\": \"projectName\""
    assert!(
        result
            .data
            .starts_with("[\n  {\n    \"name\": \"projectName\""),
        "expected 2-space indented JSON opener; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact substring "\"jsonPath\": \"project.name\""
    assert!(
        result.data.contains("\"jsonPath\": \"project.name\""),
        "missing jsonPath substring; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_text_renders_header_separator_and_seven_section_bullets() {
    // Scenario: Text format renders the header, separator, blank line, and seven section blocks

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the DispatchResult.data contains the exact line 'Foundation Sections (update-foundation field reference)'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "Foundation Sections (update-foundation field reference)"),
        "missing header line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact line '========================================================='
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "========================================================="),
        "missing separator line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact line '• projectName'
    assert!(result.data.lines().any(|l| l == "\u{2022} projectName"));

    // @step And the DispatchResult.data contains the exact line '• projectVision'
    assert!(result.data.lines().any(|l| l == "\u{2022} projectVision"));

    // @step And the DispatchResult.data contains the exact line '• projectType'
    assert!(result.data.lines().any(|l| l == "\u{2022} projectType"));

    // @step And the DispatchResult.data contains the exact line '• problemTitle'
    assert!(result.data.lines().any(|l| l == "\u{2022} problemTitle"));

    // @step And the DispatchResult.data contains the exact line '• problemDefinition'
    assert!(result
        .data
        .lines()
        .any(|l| l == "\u{2022} problemDefinition"));

    // @step And the DispatchResult.data contains the exact line '• problemImpact'
    assert!(result.data.lines().any(|l| l == "\u{2022} problemImpact"));

    // @step And the DispatchResult.data contains the exact line '• solutionOverview'
    assert!(result
        .data
        .lines()
        .any(|l| l == "\u{2022} solutionOverview"));

    // @step And the substring '• projectName' appears before '• projectVision' in the output
    let a = result
        .data
        .find("\u{2022} projectName")
        .expect("projectName present");
    let b = result
        .data
        .find("\u{2022} projectVision")
        .expect("projectVision present");
    assert!(a < b, "projectName must precede projectVision; a={a} b={b}");

    // @step And the substring '• problemImpact' appears before '• solutionOverview' in the output
    let c = result
        .data
        .find("\u{2022} problemImpact")
        .expect("problemImpact present");
    let d = result
        .data
        .find("\u{2022} solutionOverview")
        .expect("solutionOverview present");
    assert!(
        c < d,
        "problemImpact must precede solutionOverview; c={c} d={d}"
    );
}

#[test]
fn scenario_text_renders_path_constraint_and_about_lines() {
    // Scenario: Text format renders path, constraint, and about lines for each section row

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the DispatchResult.data contains the exact line '    path:       project.name'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "    path:       project.name"),
        "missing path row for project.name; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact line '    constraint: freeform string'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "    constraint: freeform string"),
        "missing 'constraint: freeform string' line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact line '    about:      Project name'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "    about:      Project name"),
        "missing 'about: Project name' line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact line '    path:       problemSpace.primaryProblem.impact'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "    path:       problemSpace.primaryProblem.impact"),
        "missing impact path; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact line '    constraint: enum: high, medium, low'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "    constraint: enum: high, medium, low"),
        "missing impact constraint; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact line '    about:      How critical the problem is'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "    about:      How critical the problem is"),
        "missing impact about; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_text_renders_examples_line_only_for_project_type() {
    // Scenario: Text format renders the examples line only for projectType

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the DispatchResult.data contains the exact line '    examples:   cli-tool, web-app, saas-platform'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "    examples:   cli-tool, web-app, saas-platform"),
        "missing projectType examples line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains exactly one line starting with '    examples:'
    let count = result
        .data
        .lines()
        .filter(|l| l.starts_with("    examples:"))
        .count();
    assert_eq!(
        count, 1,
        "expected exactly one '    examples:' line, got {count}\n{}",
        result.data
    );
}

#[test]
fn scenario_text_ends_with_two_line_footer_about_dedicated_commands() {
    // Scenario: Text format ends with the two-line footer note about dedicated commands

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-foundation-sections with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the DispatchResult.data contains the exact line 'Note: capabilities and personas are managed via dedicated commands'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "Note: capabilities and personas are managed via dedicated commands"),
        "missing footer first line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the exact line '      (add-capability, add-persona) and cannot be updated via update-foundation.'
    assert!(
        result.data.lines().any(|l| {
            l == "      (add-capability, add-persona) and cannot be updated via update-foundation."
        }),
        "missing footer second line; got:\n{}",
        result.data
    );

    // @step And the substring 'Note: capabilities and personas' appears after '• solutionOverview' in the output
    let solution_pos = result
        .data
        .find("\u{2022} solutionOverview")
        .expect("solutionOverview present");
    let note_pos = result
        .data
        .find("Note: capabilities and personas")
        .expect("footer present");
    assert!(
        solution_pos < note_pos,
        "footer must appear AFTER solutionOverview bullet; solution={solution_pos} note={note_pos}\n{}",
        result.data
    );
}

#[test]
fn scenario_dispatch_ignores_project_root_path_entirely() {
    // Scenario: Dispatch ignores the project_root path entirely

    // @step Given a project root directory containing a populated spec/ with arbitrary contents
    // NOTE: list-foundation-sections performs NO filesystem reads, so we only
    // need to prove the output is invariant under different project_root states.
    // We create a populated `spec/` subdirectory with arbitrary unrelated files.
    let tmp = TempDir::new().expect("tempdir");
    let spec = tmp.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("unrelated.txt"), "arbitrary contents").expect("write unrelated");

    // @step When I dispatch list-foundation-sections with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);
    let arr = data.as_array().expect("array");

    // @step And the parsed JSON is an array of length 7
    assert_eq!(arr.len(), 7, "expected static 7-section list; got {arr:?}");

    // @step And the entries have name values in order projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview
    let expected = [
        "projectName",
        "projectVision",
        "projectType",
        "problemTitle",
        "problemDefinition",
        "problemImpact",
        "solutionOverview",
    ];
    for (i, name) in expected.iter().enumerate() {
        assert_eq!(arr[i]["name"].as_str(), Some(*name));
    }
}
