//! Feature: spec/features/research-registry-drops-ast-tool.feature
//!
//! Moved out of research.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use std::path::Path;
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "research".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

#[test]
fn execute_mode_rejects_ast_tool_and_points_to_fspec_astgrep() {
    // @step Given an empty project root tempdir
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch research with tool="ast"
    let result = dispatch_command(req(tmp.path(), json!({ "tool": "ast" })));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains "Research tool not found: ast"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Research tool not found: ast"),
        "error message mismatch: {err}"
    );
    // @step And the error message lists "perplexity, jira, confluence, stakeholder" as the available bundled tools
    assert!(
        err.contains("Available bundled tools: perplexity, jira, confluence, stakeholder"),
        "bundled-tools list mismatch: {err}"
    );
    // @step And the error points at the `fspec astgrep` command for AST code search
    assert!(
        err.contains("For AST code search, use the `fspec astgrep` command instead"),
        "astgrep pointer missing: {err}"
    );
}
