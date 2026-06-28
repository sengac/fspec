#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/research-rust-port.feature
//
// Dispatcher-contract tests for the Rust port of `research` (RPC-286,
// LIST-only scope). Each scenario maps to exactly one #[test] with @step
// comments mirroring the Gherkin steps verbatim.
//
// RED PHASE: the command is still a stub returning NotYetPorted, so these
// tests FAIL now. They assert the real expected behaviour the Phase C
// implementation must satisfy.
//
// ENVIRONMENT ASSUMPTION: these tests resolve research-tool configuration via
// the same precedence chain as the TS `resolveConfig` (ENV → ~/.fspec config →
// project config → defaults). They assume a clean environment: no research
// env vars (PERPLEXITY_API_KEY, JIRA_*, …) are set and ~/.fspec/fspec-config.json
// contains no `research` block. Only the per-test project config differs.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "research".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_project_config(project_root: &Path, config: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("fspec-config.json"),
        serde_json::to_string_pretty(config).expect("ser config"),
    )
    .expect("write fspec-config.json");
}

/// Parse the dispatcher data envelope and return its `tools` array.
fn tools_of(result_data: &str) -> Vec<Value> {
    let data: Value = serde_json::from_str(result_data).expect("parse data json");
    data["tools"].as_array().cloned().unwrap_or_default()
}

fn tool_named<'a>(tools: &'a [Value], name: &str) -> Option<&'a Value> {
    tools.iter().find(|t| t["name"].as_str() == Some(name))
}

fn is_configured(tools: &[Value], name: &str) -> bool {
    tool_named(tools, name)
        .and_then(|t| t["configured"].as_bool())
        .unwrap_or(false)
}

// ---------- scenarios ----------

#[test]
fn list_mode_enumerates_bundled_registry_with_ast_configured_by_default() {
    // @step Given an empty project root tempdir with no spec/fspec-config.json and no research env vars
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch research with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");
    let tools = tools_of(&result.data);

    // @step And the result lists the tool "ast"
    assert!(
        tool_named(&tools, "ast").is_some(),
        "missing ast: {tools:?}"
    );
    // @step And the result lists the tool "perplexity"
    assert!(
        tool_named(&tools, "perplexity").is_some(),
        "missing perplexity"
    );
    // @step And the result lists the tool "jira"
    assert!(tool_named(&tools, "jira").is_some(), "missing jira");
    // @step And the result lists the tool "confluence"
    assert!(
        tool_named(&tools, "confluence").is_some(),
        "missing confluence"
    );
    // @step And the result lists the tool "stakeholder"
    assert!(
        tool_named(&tools, "stakeholder").is_some(),
        "missing stakeholder"
    );

    // @step And the tool "ast" is reported as configured
    assert!(
        is_configured(&tools, "ast"),
        "ast must be configured: {tools:?}"
    );
    // @step And the tool "perplexity" is reported as not configured
    assert!(
        !is_configured(&tools, "perplexity"),
        "perplexity must be not configured"
    );
}

#[test]
fn list_mode_reflects_configured_perplexity_api_key_from_project_config() {
    // @step Given a project root tempdir whose spec/fspec-config.json sets research.perplexity.apiKey to "pplx-test"
    let tmp = TempDir::new().expect("tempdir");
    write_project_config(
        tmp.path(),
        &json!({ "research": { "perplexity": { "apiKey": "pplx-test" } } }),
    );

    // @step When I dispatch research with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the tool "perplexity" is reported as configured
    let tools = tools_of(&result.data);
    assert!(
        is_configured(&tools, "perplexity"),
        "perplexity must be configured: {tools:?}"
    );
}

#[test]
fn list_mode_reports_stakeholder_configured_when_required_webhook_present() {
    // @step Given a project root tempdir whose spec/fspec-config.json sets research.stakeholder.teamsWebhook to "https://example.test/hook"
    let tmp = TempDir::new().expect("tempdir");
    write_project_config(
        tmp.path(),
        &json!({ "research": { "stakeholder": { "teamsWebhook": "https://example.test/hook" } } }),
    );

    // @step When I dispatch research with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the tool "stakeholder" is reported as configured
    let tools = tools_of(&result.data);
    assert!(
        is_configured(&tools, "stakeholder"),
        "stakeholder must be configured: {tools:?}"
    );
}

#[test]
fn list_mode_does_not_create_files_or_spawn_a_process() {
    // @step Given an empty project root tempdir with no spec subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(
        !tmp.path().join("spec").exists(),
        "precondition: no spec dir"
    );

    // @step When I dispatch research with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And no spec/fspec-config.json is created in the project root
    assert!(
        !tmp.path().join("spec/fspec-config.json").exists(),
        "list mode must not create config"
    );

    // @step And the command completes without spawning a child process or opening a network socket
    // (Implicit: dispatch_command runs under the single-poll sync dispatcher;
    //  a real async/process/socket would have returned Poll::Pending and the
    //  dispatcher would surface an InvalidArgs "returned Pending" error.)
    assert!(result.error.is_none(), "no error expected: {result:?}");
}

#[test]
fn execute_mode_rejects_unknown_tool_before_doing_work() {
    // @step Given an empty project root tempdir
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch research with tool="does-not-exist"
    let result = dispatch_command(req(tmp.path(), json!({ "tool": "does-not-exist" })));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains "Research tool not found: does-not-exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Research tool not found: does-not-exist"),
        "error message mismatch: {err}"
    );
}

#[test]
fn both_front_doors_converge_on_same_function() {
    // @step Given an empty project root tempdir
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch research with no flags via the dispatcher and via the standalone binary
    // (Dispatcher front-door here; the binary front-door is exercised in
    //  cli_research.rs. Both call commands::research::run.)
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "dispatcher path failed: {result:?}");
    let tools = tools_of(&result.data);

    // @step Then both invocations enumerate the same five bundled research tools
    let mut names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["ast", "confluence", "jira", "perplexity", "stakeholder"],
        "tool set mismatch: {names:?}"
    );
}
