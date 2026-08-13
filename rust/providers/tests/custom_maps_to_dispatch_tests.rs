#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-maps-to-dispatch-non-file-categories.feature
//!
//! Integration tests for PROV-069: extend the custom-provider adapter's
//! `default_to_internal` dispatch to cover every `maps_to` category
//! documented in `KNOWN_MAPS_TO` — not just the three `file:*` targets.
//!
//! These tests exercise `codelet_providers::custom::tool_dispatch`,
//! which does not exist yet — the file fails to compile in the red
//! phase.

use serde_json::json;

use codelet_providers::custom::tool_dispatch::{default_to_internal, DispatchedToolParams};
use codelet_tools::facade::{
    InternalBashParams, InternalBridgeParams, InternalExecParams, InternalHitlParams,
    InternalLsParams, InternalSearchParams, InternalWebSearchParams,
};

// =========================================================================
// Scenario: Dispatch bash maps_to to InternalBashParams::Execute
// =========================================================================
#[test]
fn dispatch_bash_maps_to_internal_bash_params_execute() {
    // @step Given a params JSON object with command set to "ls"
    let params = json!({ "command": "ls" });

    // @step When I call default_to_internal with maps_to "bash"
    let dispatched = default_to_internal("bash", &params).expect("dispatch succeeds");

    // @step Then the result is an InternalBashParams::Execute whose command is "ls"
    match dispatched {
        DispatchedToolParams::Bash(InternalBashParams::Execute { command, .. }) => {
            assert_eq!(command, "ls");
        }
        other => panic!("expected DispatchedToolParams::Bash(Execute), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Dispatch search:grep maps_to to InternalSearchParams::Grep
// =========================================================================
#[test]
fn dispatch_search_grep_maps_to_internal_search_params_grep() {
    // @step Given a params JSON object with pattern "foo" and path "src"
    let params = json!({ "pattern": "foo", "path": "src" });

    // @step When I call default_to_internal with maps_to "search:grep"
    let dispatched = default_to_internal("search:grep", &params).expect("dispatch succeeds");

    // @step Then the result is an InternalSearchParams::Grep whose pattern is "foo" and path is Some("src")
    match dispatched {
        DispatchedToolParams::Search(InternalSearchParams::Grep { pattern, path, .. }) => {
            assert_eq!(pattern, "foo");
            assert_eq!(path, Some("src".to_string()));
        }
        other => panic!("expected DispatchedToolParams::Search(Grep), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Dispatch search:glob maps_to to InternalSearchParams::Glob
// =========================================================================
#[test]
fn dispatch_search_glob_maps_to_internal_search_params_glob() {
    // @step Given a params JSON object with pattern "*.rs"
    let params = json!({ "pattern": "*.rs" });

    // @step When I call default_to_internal with maps_to "search:glob"
    let dispatched = default_to_internal("search:glob", &params).expect("dispatch succeeds");

    // @step Then the result is an InternalSearchParams::Glob whose pattern is "*.rs"
    match dispatched {
        DispatchedToolParams::Search(InternalSearchParams::Glob { pattern, .. }) => {
            assert_eq!(pattern, "*.rs");
        }
        other => panic!("expected DispatchedToolParams::Search(Glob), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Dispatch ls maps_to to InternalLsParams::List
// =========================================================================
#[test]
fn dispatch_ls_maps_to_internal_ls_params_list() {
    // @step Given a params JSON object with path "/tmp"
    let params = json!({ "path": "/tmp" });

    // @step When I call default_to_internal with maps_to "ls"
    let dispatched = default_to_internal("ls", &params).expect("dispatch succeeds");

    // @step Then the result is an InternalLsParams::List whose path is Some("/tmp")
    match dispatched {
        DispatchedToolParams::Ls(InternalLsParams::List { path, .. }) => {
            assert_eq!(path, Some("/tmp".to_string()));
        }
        other => panic!("expected DispatchedToolParams::Ls(List), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Dispatch web_search:search maps_to to InternalWebSearchParams::Search
// =========================================================================
#[test]
fn dispatch_web_search_search_maps_to_internal_web_search_params_search() {
    // @step Given a params JSON object with query "rust"
    let params = json!({ "query": "rust" });

    // @step When I call default_to_internal with maps_to "web_search:search"
    let dispatched = default_to_internal("web_search:search", &params).expect("dispatch succeeds");

    // @step Then the result is an InternalWebSearchParams::Search whose query is "rust"
    match dispatched {
        DispatchedToolParams::WebSearch(InternalWebSearchParams::Search { query }) => {
            assert_eq!(query, "rust");
        }
        other => panic!("expected DispatchedToolParams::WebSearch(Search), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Dispatch fspec maps_to to InternalFspecParams
// =========================================================================
#[test]
fn dispatch_fspec_maps_to_internal_fspec_params() {
    // @step Given a params JSON object with command "board", args "{}" and project_root "."
    let params = json!({
        "command": "board",
        "args": "{}",
        "project_root": ".",
    });

    // @step When I call default_to_internal with maps_to "fspec"
    let dispatched = default_to_internal("fspec", &params).expect("dispatch succeeds");

    // @step Then the result is an InternalFspecParams with command "board"
    match dispatched {
        DispatchedToolParams::Fspec(p) => {
            assert_eq!(p.command, "board");
            assert_eq!(p.args, "{}");
            assert_eq!(p.project_root, ".");
        }
        other => panic!("expected DispatchedToolParams::Fspec, got {other:?}"),
    }
}

// =========================================================================
// Scenario: Dispatch bridge maps_to to InternalBridgeParams::List
// =========================================================================
#[test]
fn dispatch_bridge_maps_to_internal_bridge_params_list() {
    // @step Given a params JSON object with action.type set to "list"
    let params = json!({ "action": { "type": "list" } });

    // @step When I call default_to_internal with maps_to "bridge"
    let dispatched = default_to_internal("bridge", &params).expect("dispatch succeeds");

    // @step Then the result is an InternalBridgeParams::List variant
    match dispatched {
        DispatchedToolParams::Bridge(InternalBridgeParams::List) => {}
        other => panic!("expected DispatchedToolParams::Bridge(List), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Dispatch exec:run maps_to to InternalExecParams::Run
// =========================================================================
#[test]
fn dispatch_exec_run_maps_to_internal_exec_params_run() {
    // @step Given a params JSON object with command "ls"
    let params = json!({ "command": "ls" });

    // @step When I call default_to_internal with maps_to "exec:run"
    let dispatched = default_to_internal("exec:run", &params).expect("dispatch succeeds");

    // @step Then the result is an InternalExecParams::Run whose command equals the input command
    match dispatched {
        DispatchedToolParams::Exec(InternalExecParams::Run { command, .. }) => {
            assert_eq!(command, json!("ls"));
        }
        other => panic!("expected DispatchedToolParams::Exec(Run), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Dispatch hitl maps_to to InternalHitlParams::Request
// =========================================================================
#[test]
fn dispatch_hitl_maps_to_internal_hitl_params_request() {
    // @step Given a params JSON object with a questions array containing one valid HitlQuestion
    let params = json!({
        "questions": [
            {
                "id": "confirm_action",
                "header": "Confirm",
                "question": "Proceed?",
                "options": null,
            }
        ]
    });

    // @step When I call default_to_internal with maps_to "hitl"
    let dispatched = default_to_internal("hitl", &params).expect("dispatch succeeds");

    // @step Then the result is an InternalHitlParams::Request whose questions vec has length 1
    match dispatched {
        DispatchedToolParams::Hitl(InternalHitlParams::Request { questions }) => {
            assert_eq!(questions.len(), 1);
            assert_eq!(questions[0].id, "confirm_action");
        }
        other => panic!("expected DispatchedToolParams::Hitl(Request), got {other:?}"),
    }
}

// =========================================================================
// Scenario: Unknown maps_to value returns error listing valid identifiers
// =========================================================================
#[test]
fn unknown_maps_to_value_returns_error_listing_valid_identifiers() {
    // @step Given any params JSON object
    let params = json!({});

    // @step When I call default_to_internal with maps_to "mystery:foo"
    let result = default_to_internal("mystery:foo", &params);

    // @step Then I receive a CustomProviderError whose message contains "mystery:foo" and "bash"
    let err = result.expect_err("dispatch should fail for unknown maps_to");
    let msg = format!("{err}");
    assert!(
        msg.contains("mystery:foo"),
        "error should mention offending maps_to, got: {msg}"
    );
    assert!(
        msg.contains("bash"),
        "error should list valid identifiers like 'bash', got: {msg}"
    );
}
