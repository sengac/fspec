#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-diagram-fails-with-screen-is-not-defined-error-for-valid-c4-diagrams.feature
//
// BUG-082: valid C4Context diagrams must pass add-diagram's Mermaid
// validation. Each test dispatches add-diagram with a C4Context body, checks
// the diagram is stored in spec/foundation.json unchanged, and checks the
// FOUNDATION.md regeneration (which validates every diagram again) shows it.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use tempfile::TempDir;

fn complete_foundation() -> Value {
    json!({
        "version": "2.0.0",
        "project": {"name": "Bank", "vision": "Online banking", "projectType": "web-app"},
        "problemSpace": {"primaryProblem": {
            "title": "Primary Problem", "description": "Problem description", "impact": "high"}},
        "solutionSpace": {"overview": "Solution overview", "capabilities": [
            {"name": "Core Capability", "description": "What the system does"}]},
        "architectureDiagrams": []
    })
}

fn project_with_foundation() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    let spec = tmp.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(&complete_foundation()).expect("ser foundation"),
    )
    .expect("write foundation.json");
    tmp
}

fn add_diagram(project_root: &Path, title: &str, code: &str) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "add-diagram".to_string(),
        args_json: json!({"section": "Architecture", "title": title, "code": code}).to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn stored_code(project_root: &Path, title: &str) -> Option<String> {
    let raw =
        fs::read_to_string(project_root.join("spec/foundation.json")).expect("foundation.json");
    let data: Value = serde_json::from_str(&raw).expect("parse foundation.json");
    data["architectureDiagrams"]
        .as_array()?
        .iter()
        .find(|d| d["title"] == title)
        .and_then(|d| d["mermaidCode"].as_str())
        .map(str::to_string)
}

fn foundation_md(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/FOUNDATION.md")).expect("FOUNDATION.md")
}

const COMPLEX_C4: &str = "C4Context
  title System Context diagram for Internet Banking System
  Person(customer, \"Personal Banking Customer\", \"A customer of the bank\")
  System(banking_system, \"Internet Banking System\", \"Allows customers to check balances\")
  System_Ext(email_system, \"E-mail system\", \"The internal Microsoft Exchange system\")
  Rel(customer, banking_system, \"Uses\")
  Rel(banking_system, email_system, \"Sends e-mails\", \"SMTP\")";

#[test]
fn add_c4context_diagram_with_complex_example() {
    // Scenario: Add C4Context diagram with complex example

    // @step Given I have a project with foundation.json
    let tmp = project_with_foundation();

    // @step When I run add-diagram with a C4Context diagram containing Person, System, System_Ext, and Rel elements
    let result = add_diagram(tmp.path(), "Internet Banking Context", COMPLEX_C4);

    // @step Then add-diagram succeeds
    assert!(result.success, "expected success, got {result:?}");

    // @step And the diagram should be added to foundation.json
    assert_eq!(
        stored_code(tmp.path(), "Internet Banking Context").as_deref(),
        Some(COMPLEX_C4)
    );

    // @step And spec/FOUNDATION.md shows the diagram in a mermaid code block
    assert!(
        foundation_md(tmp.path()).contains(&format!(
            "### Internet Banking Context\n\n```mermaid\n{COMPLEX_C4}\n```"
        )),
        "{}",
        foundation_md(tmp.path())
    );
}

/// The diagram from GitHub issue #5, with `BiRel` in place of the issue's
/// `Rel_Neighbor`, which is not a Mermaid C4 keyword.
const ISSUE_5_C4: &str = "C4Context
  title System Context diagram for Internet Banking System
  Person(customer, \"Personal Banking Customer\", \"A customer of the bank, with personal bank accounts.\")
  System(banking_system, \"Internet Banking System\", \"Allows customers to view information about their bank accounts, and make payments.\")
  System_Ext(mail_system, \"E-mail system\", \"The internal Microsoft Exchange e-mail system.\")
  Rel(customer, banking_system, \"Uses\")
  Rel_Back(customer, mail_system, \"Sends e-mails to\")
  BiRel(banking_system, mail_system, \"Sends e-mails\", \"SMTP\")";

#[test]
fn add_c4context_diagram_from_reported_issue() {
    // Scenario: Add C4Context diagram from reported issue

    // @step Given I have a project with foundation.json
    let tmp = project_with_foundation();

    // @step When I run add-diagram with the exact C4Context code from GitHub issue #5
    let result = add_diagram(tmp.path(), "Banking System Context", ISSUE_5_C4);

    // @step Then add-diagram succeeds
    assert!(result.success, "expected success, got {result:?}");

    // @step And the diagram should be added to foundation.json
    assert_eq!(
        stored_code(tmp.path(), "Banking System Context").as_deref(),
        Some(ISSUE_5_C4)
    );

    // @step And the result carries no error
    assert_eq!(result.error, None);
}

const SIMPLE_C4: &str = "C4Context
  title Simple System Context
  Person(user, \"User\", \"A user of the system\")
  System(system, \"System\", \"The main system\")
  Rel(user, system, \"Uses\")";

#[test]
fn add_simple_c4context_diagram() {
    // Scenario: Add simple C4Context diagram

    // @step Given I have a project with foundation.json
    let tmp = project_with_foundation();

    // @step When I run add-diagram with a simple C4Context diagram with one Person and one System
    let result = add_diagram(tmp.path(), "Simple Context", SIMPLE_C4);

    // @step Then add-diagram succeeds
    assert!(result.success, "expected success, got {result:?}");

    // @step And the diagram should be added to foundation.json architectureDiagrams array
    assert_eq!(
        stored_code(tmp.path(), "Simple Context").as_deref(),
        Some(SIMPLE_C4)
    );

    // @step And spec/FOUNDATION.md shows the diagram in a mermaid code block
    assert!(
        foundation_md(tmp.path()).contains(&format!(
            "### Simple Context\n\n```mermaid\n{SIMPLE_C4}\n```"
        )),
        "{}",
        foundation_md(tmp.path())
    );
}
