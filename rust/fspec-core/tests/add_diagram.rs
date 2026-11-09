#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-diagram-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-diagram`
// (RPC-178). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-diagram".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_foundation(project_root: &Path, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(value).expect("ser foundation"),
    )
    .expect("write foundation.json");
}

fn read_foundation(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/foundation.json"))
        .expect("read foundation.json");
    serde_json::from_str(&raw).expect("parse foundation.json")
}

fn empty_foundation() -> Value {
    json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "architectureDiagrams": []
    })
}

// ---------- scenarios ----------

#[test]
fn dispatcher_appends_a_new_diagram_when_architecture_diagrams_is_empty() {
    // Scenario: Dispatcher appends a new diagram when architectureDiagrams is empty

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='Component Diagram' code='graph TD\n  A-->B'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "section": "Architecture",
            "title": "Component Diagram",
            "code": "graph TD\n  A-->B"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/foundation.json architectureDiagrams contains exactly one entry
    let data = read_foundation(tmp.path());
    let diagrams = data["architectureDiagrams"]
        .as_array()
        .expect("architectureDiagrams must be an array");
    assert_eq!(diagrams.len(), 1, "expected 1 diagram, got {diagrams:?}");

    // @step And that entry has title='Component Diagram' and mermaidCode='graph TD\n  A-->B'
    let d = &diagrams[0];
    assert_eq!(d["title"].as_str(), Some("Component Diagram"));
    assert_eq!(d["mermaidCode"].as_str(), Some("graph TD\n  A-->B"));

    // @step And that entry has no 'section' field
    assert!(
        d.get("section").is_none(),
        "diagram entry must NOT include a 'section' field; got: {d}"
    );
}

#[test]
fn dispatcher_replaces_existing_diagram_with_same_title_in_place() {
    // Scenario: Dispatcher replaces an existing diagram with the same title in place

    // @step Given spec/foundation.json contains a diagram titled 'System Overview' with mermaidCode='graph LR\n  Old-->Content'
    let tmp = TempDir::new().expect("tempdir");
    let mut f = empty_foundation();
    f["architectureDiagrams"] = json!([
        {"title": "System Overview", "mermaidCode": "graph LR\n  Old-->Content"}
    ]);
    write_foundation(tmp.path(), &f);

    // @step When I dispatch add-diagram with section='Architecture' title='System Overview' code='graph LR\n  New-->Diagram'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "section": "Architecture",
            "title": "System Overview",
            "code": "graph LR\n  New-->Diagram"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json architectureDiagrams length is unchanged
    let data = read_foundation(tmp.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 1, "length must stay at 1, got {diagrams:?}");

    // @step And the diagram titled 'System Overview' now has mermaidCode='graph LR\n  New-->Diagram'
    assert_eq!(diagrams[0]["title"].as_str(), Some("System Overview"));
    assert_eq!(
        diagrams[0]["mermaidCode"].as_str(),
        Some("graph LR\n  New-->Diagram")
    );
}

#[test]
fn dispatcher_appends_new_diagram_alongside_existing_one() {
    // Scenario: Dispatcher appends a new diagram alongside an existing one

    // @step Given spec/foundation.json contains a diagram titled 'Diagram 1' with mermaidCode='graph TD\n  A-->B'
    let tmp = TempDir::new().expect("tempdir");
    let mut f = empty_foundation();
    f["architectureDiagrams"] = json!([
        {"title": "Diagram 1", "mermaidCode": "graph TD\n  A-->B"}
    ]);
    write_foundation(tmp.path(), &f);

    // @step When I dispatch add-diagram with section='Architecture' title='Diagram 2' code='graph TD\n  C-->D'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "section": "Architecture",
            "title": "Diagram 2",
            "code": "graph TD\n  C-->D"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json architectureDiagrams has length 2
    let data = read_foundation(tmp.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 2);

    // @step And the order is ['Diagram 1', 'Diagram 2']
    assert_eq!(diagrams[0]["title"].as_str(), Some("Diagram 1"));
    assert_eq!(diagrams[1]["title"].as_str(), Some("Diagram 2"));
}

#[test]
fn dispatcher_auto_creates_foundation_json_when_missing() {
    // Scenario: Dispatcher auto-creates spec/foundation.json when missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-diagram with section='Architecture' title='Initial' code='graph TD\n  Start-->End'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "section": "Architecture",
            "title": "Initial",
            "code": "graph TD\n  Start-->End"
        }),
    ));

    // @step Then the file spec/foundation.json exists
    assert!(
        tmp.path().join("spec/foundation.json").exists(),
        "spec/foundation.json must be auto-created by ensure_foundation_file"
    );

    // @step And the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json architectureDiagrams contains exactly one entry titled 'Initial'
    let data = read_foundation(tmp.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 1);
    assert_eq!(diagrams[0]["title"].as_str(), Some("Initial"));
}

#[test]
fn dispatcher_persists_optional_description_when_provided() {
    // Scenario: Dispatcher persists optional description when provided

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='With Notes' code='graph TD\n  A-->B' description='A flowchart'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "section": "Architecture",
            "title": "With Notes",
            "code": "graph TD\n  A-->B",
            "description": "A flowchart"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the diagram titled 'With Notes' has description='A flowchart'
    let data = read_foundation(tmp.path());
    let d = data["architectureDiagrams"]
        .as_array()
        .expect("array")
        .iter()
        .find(|d| d["title"].as_str() == Some("With Notes"))
        .expect("With Notes must exist");
    assert_eq!(d["description"].as_str(), Some("A flowchart"));
}

#[test]
fn dispatcher_rejects_empty_section_argument() {
    // Scenario: Dispatcher rejects empty section argument

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='' title='X' code='graph TD\n  A-->B'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "", "title": "X", "code": "graph TD\n  A-->B"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Section name cannot be empty'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Section name cannot be empty"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_empty_title_argument() {
    // Scenario: Dispatcher rejects empty title argument

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='' code='graph TD\n  A-->B'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "", "code": "graph TD\n  A-->B"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Diagram title cannot be empty'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Diagram title cannot be empty"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_empty_code_argument() {
    // Scenario: Dispatcher rejects empty code argument

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='X' code=''
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "X", "code": ""}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Diagram code cannot be empty'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Diagram code cannot be empty"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_diagram_with_quoted_subgraph_title() {
    // Scenario: Dispatcher rejects diagram with quoted subgraph title (merman pre-check)

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='Bad' code='graph TB\n  subgraph "Quoted"\n  end'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "section": "Architecture",
            "title": "Bad",
            "code": "graph TB\n  subgraph \"Quoted\"\n  end"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Quoted subgraph titles are not supported'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Quoted subgraph titles are not supported"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_diagram_with_invalid_subgraph_identifier() {
    // Scenario: Dispatcher rejects diagram with invalid subgraph identifier (merman pre-check)

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='Bad' code='graph TB\n  subgraph Id!!!\n  end'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "section": "Architecture",
            "title": "Bad",
            "code": "graph TB\n  subgraph Id!!!\n  end"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Invalid subgraph identifier 'Id!!!'"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid subgraph identifier 'Id!!!'"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_preserves_unknown_top_level_fields_on_write() {
    // Scenario: Dispatcher preserves unknown top-level fields on write

    // @step Given spec/foundation.json contains a top-level 'project' object and a custom 'experiments' key
    let tmp = TempDir::new().expect("tempdir");
    let f = json!({
        "version": "2.0.0",
        "project": {"name": "fspec", "vision": "v", "projectType": "cli-tool"},
        "architectureDiagrams": [],
        "experiments": {"alpha": true, "beta": [1, 2, 3]}
    });
    write_foundation(tmp.path(), &f);

    // @step When I dispatch add-diagram with section='Architecture' title='X' code='graph TD\n  A-->B'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "X", "code": "graph TD\n  A-->B"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json still contains the 'experiments' key with its original value
    let data = read_foundation(tmp.path());
    assert_eq!(data["experiments"]["alpha"].as_bool(), Some(true));
    assert_eq!(data["experiments"]["beta"][0].as_u64(), Some(1));

    // @step And spec/foundation.json still contains the 'project' object
    assert_eq!(data["project"]["name"].as_str(), Some("fspec"));
}

#[test]
fn dispatcher_rejects_diagram_with_a_genuine_merman_syntax_error() {
    // Scenario: Dispatcher rejects diagram with a genuine merman syntax error

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='Bad' code='flowchart TD\n  A[Start --> B[Done'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "section": "Architecture",
            "title": "Bad",
            "code": "flowchart TD\n  A[Start --> B[Done"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And no entry is appended to architectureDiagrams
    let data = read_foundation(tmp.path());
    assert_eq!(
        data["architectureDiagrams"]
            .as_array()
            .expect("array")
            .len(),
        0,
        "no diagram should be persisted on invalid mermaid"
    );
}

#[test]
fn dispatcher_rejects_code_that_matches_no_mermaid_diagram_type() {
    // Scenario: Dispatcher rejects code that matches no mermaid diagram type

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='Bad' code='this is not a diagram at all'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "section": "Architecture",
            "title": "Bad",
            "code": "this is not a diagram at all"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And no entry is appended to architectureDiagrams
    let data = read_foundation(tmp.path());
    assert_eq!(
        data["architectureDiagrams"]
            .as_array()
            .expect("array")
            .len(),
        0,
        "no diagram should be persisted on undetectable mermaid"
    );
}

/// BUG-535: a schema-valid generic v2.0.0 foundation, so generate-foundation-md accepts it.
fn complete_foundation() -> Value {
    json!({
        "version": "2.0.0",
        "project": {"name": "Original Name", "vision": "Original Vision", "projectType": "cli-tool"},
        "problemSpace": {"primaryProblem": {
            "title": "Primary Problem", "description": "Problem description", "impact": "high"}},
        "solutionSpace": {"overview": "Solution overview", "capabilities": [
            {"name": "Core Capability", "description": "What the system does"}]},
        "architectureDiagrams": []
    })
}

#[test]
fn dispatcher_regenerates_foundation_md_after_adding_a_diagram() {
    // Scenario: Dispatcher regenerates FOUNDATION.md after adding a diagram (BUG-535)

    // @step Given spec/foundation.json is a complete foundation with vision 'Original Vision' and spec/FOUNDATION.md does not exist
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &complete_foundation());
    let foundation_md = tmp.path().join("spec/FOUNDATION.md");
    assert!(!foundation_md.exists());

    // @step When I dispatch add-diagram with section='Architecture' title='System Context' code='graph TD\n  A-->B'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "System Context", "code": "graph TD\n  A-->B"}),
    ));
    assert!(result.success, "expected success, got {result:?}");

    // @step Then spec/FOUNDATION.md exists
    let markdown = fs::read_to_string(&foundation_md).expect("FOUNDATION.md regenerated");

    // @step And it shows 'Original Vision', the heading '### System Context' and the diagram as a mermaid code block
    assert!(markdown.contains("Original Vision"), "{markdown}");
    assert!(
        markdown.contains("### System Context\n\n```mermaid\ngraph TD\n  A-->B\n```"),
        "{markdown}"
    );
}

#[test]
fn a_rejected_add_diagram_leaves_foundation_md_unchanged() {
    // Scenario: A rejected add-diagram leaves FOUNDATION.md unchanged (BUG-535)

    // @step Given spec/foundation.json is a complete foundation and spec/FOUNDATION.md has been generated from it
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &complete_foundation());
    let generated = dispatch_command(DispatchRequest {
        command: "generate-foundation-md".to_string(),
        args_json: "{}".to_string(),
        project_root: tmp.path().to_path_buf(),
    });
    assert!(
        generated.success,
        "generate-foundation-md failed: {generated:?}"
    );
    let foundation_md = tmp.path().join("spec/FOUNDATION.md");
    let before = fs::read(&foundation_md).expect("read FOUNDATION.md");

    // @step When I dispatch add-diagram with title='Broken' and a body that matches no mermaid diagram type
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "Broken", "code": "this is not a diagram at all"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And spec/FOUNDATION.md is byte-for-byte unchanged
    assert_eq!(
        fs::read(&foundation_md).expect("read FOUNDATION.md"),
        before
    );
}

#[test]
fn dispatcher_accepts_flowchart_sequence_and_class_diagrams() {
    // Scenario: Dispatcher accepts flowchart, sequence and class diagrams (COV-071, from add-diagram)

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with title='Valid Flow' code='flowchart TD\n  A-->B\n  B-->C', title='Valid Sequence' code='sequenceDiagram\n  Alice->>Bob: Hello\n  Bob-->>Alice: Hi' and title='Domain Model' code='classDiagram\n  Class01 <|-- Class02'
    let diagrams = [
        ("Valid Flow", "flowchart TD\n  A-->B\n  B-->C"),
        (
            "Valid Sequence",
            "sequenceDiagram\n  Alice->>Bob: Hello\n  Bob-->>Alice: Hi",
        ),
        ("Domain Model", "classDiagram\n  Class01 <|-- Class02"),
    ];
    let results: Vec<_> = diagrams
        .iter()
        .map(|(title, code)| {
            dispatch_command(req(
                tmp.path(),
                json!({"section": "Architecture", "title": title, "code": code}),
            ))
        })
        .collect();

    // @step Then each dispatch returns success=true
    for result in &results {
        assert!(result.success, "expected success, got {result:?}");
    }

    // @step And spec/foundation.json architectureDiagrams holds the three diagrams in that order with their mermaidCode unchanged
    let data = read_foundation(tmp.path());
    let stored: Vec<(&str, &str)> = data["architectureDiagrams"]
        .as_array()
        .expect("array")
        .iter()
        .map(|d| {
            (
                d["title"].as_str().unwrap_or(""),
                d["mermaidCode"].as_str().unwrap_or(""),
            )
        })
        .collect();
    assert_eq!(stored, diagrams.to_vec());
}

#[test]
fn dispatcher_keeps_every_line_of_a_multi_line_diagram() {
    // Scenario: Dispatcher keeps every line of a multi-line diagram (COV-071, from add-diagram)

    // @step Given spec/foundation.json is a complete foundation
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &complete_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='Complex' code='graph TD\n  A-->B\n  B-->C\n  C-->D'
    let code = "graph TD\n  A-->B\n  B-->C\n  C-->D";
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "Complex", "code": code}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success, got {result:?}");

    // @step And the diagram titled 'Complex' has mermaidCode='graph TD\n  A-->B\n  B-->C\n  C-->D'
    let data = read_foundation(tmp.path());
    let diagram = data["architectureDiagrams"]
        .as_array()
        .expect("array")
        .iter()
        .find(|d| d["title"] == "Complex")
        .expect("diagram titled Complex");
    assert_eq!(diagram["mermaidCode"].as_str(), Some(code));

    // @step And spec/FOUNDATION.md shows the four lines with their indentation inside one mermaid code block under '### Complex'
    let markdown =
        fs::read_to_string(tmp.path().join("spec/FOUNDATION.md")).expect("FOUNDATION.md");
    assert!(
        markdown.contains("### Complex\n\n```mermaid\ngraph TD\n  A-->B\n  B-->C\n  C-->D\n```"),
        "{markdown}"
    );
}

#[test]
fn dispatcher_error_names_the_parse_problem_in_the_diagram() {
    // Scenario: Dispatcher error names the parse problem in the diagram (COV-071, from add-diagram)

    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-diagram with section='Architecture' title='Error Test' code='graph TD\n  A[Missing bracket'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "Error Test", "code": "graph TD\n  A[Missing bracket"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Unterminated node label (missing `]`)'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Unterminated node label (missing `]`)"),
        "error does not name the parse problem; got: {msg}"
    );

    // @step And no entry is appended to architectureDiagrams
    let data = read_foundation(tmp.path());
    assert_eq!(
        data["architectureDiagrams"]
            .as_array()
            .expect("array")
            .len(),
        0
    );
}
