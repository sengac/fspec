#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/post-deserialization-arg-errors-include-recovery-guidance-tool-023-follow-up.feature
//
// TOOL-024 (TOOL-023 follow-up): argument-validation failures raised
// AFTER successful deserialization — empty/missing-value checks inside
// the tool's own call()/execute() — must be self-contained recovery
// surfaces: registered tool name + bad parameter + accepted-parameter
// reference + canonical example call. The tool_usage renderer must also
// handle union schemas (oneOf non-object variants, anyOf properties)
// without empty-object placeholders or bare "(any)" labels.
//
// Every Gherkin step is mirrored by an @step comment.

use codelet_common::tool_usage::render_parameter_reference;
use codelet_tools::astgrep::AstGrepTool;
use codelet_tools::astgrep_refactor::AstGrepRefactorTool;
use codelet_tools::bash::BashTool;
use codelet_tools::deep_search::DeepSearchTool;
use codelet_tools::done::DoneTool;
use codelet_tools::grep::GrepTool;
use codelet_tools::request_user_input::{
    clear_all_hitl_handlers, set_hitl_handler, HitlHandler, HitlResponse, RequestUserInputTool,
};
use codelet_tools::schedule::types::ScheduleRequest;
use codelet_tools::schedule::{
    clear_all_schedule_handlers, set_schedule_handler, ScheduleHandler, ScheduleResult,
    ScheduleTool,
};
use codelet_tools::ToolError;
use rig::tool::ToolSet;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// A one-tool ToolSet (the ToolSet registry is case-insensitive on
/// registration/lookup, so tools stay distinct per name).
fn single_toolset(tool: impl rig::tool::ToolDyn + 'static) -> ToolSet {
    ToolSet::builder().static_tool(tool).build()
}

/// The LLM-visible error text for a failed ToolSet call.
async fn toolset_error(toolset: &ToolSet, name: &str, args_json: &str) -> String {
    toolset
        .call(name, args_json.to_string())
        .await
        .expect_err("expected an argument-validation error, got success")
        .to_string()
}

/// Downcast a ToolSet error to the inner codelet ToolError (the boxed
/// error carried by ToolSetError::ToolCallError).
fn inner_tool_error(rig_err: &rig::tool::ToolSetError) -> &ToolError {
    match rig_err {
        rig::tool::ToolSetError::ToolCallError(rig::tool::ToolError::ToolCallError(boxed)) => boxed
            .downcast_ref::<ToolError>()
            .expect("the boxed error must be the codelet ToolError"),
        other => panic!("unexpected rig ToolSetError shape: {other:?}"),
    }
}

/// Scenario: Bash empty command error names the tool and shows full usage
#[tokio::test]
async fn scenario_bash_empty_command_error_includes_recovery() {
    // @step Given I register the Bash direct tool with a schema declaring required 'command' (string) and optional 'cwd'
    let tool = BashTool::new(Uuid::new_v4());
    let toolset = single_toolset(tool);
    assert!(
        toolset.contains("Bash"),
        "BashTool must be registered under its registered name 'Bash'"
    );

    // @step When I call Bash with command set to the empty string
    let msg = toolset_error(&toolset, "Bash", r#"{"command":""}"#).await;

    // @step Then the error contains the substring 'command parameter is required'
    assert!(
        msg.contains("command parameter is required"),
        "original message must stay intact: {msg}"
    );

    // @step And the error names the tool 'Bash' with its registered name, not a lowercase alias
    assert!(
        msg.contains("Accepted parameters for Bash"),
        "error must name the registered tool 'Bash': {msg}"
    );

    // @step And the error lists the accepted parameters (command string required, cwd optional with type)
    assert!(
        msg.contains("command (string, required)"),
        "required 'command' parameter must be listed with its type: {msg}"
    );
    assert!(
        msg.contains("cwd (string") && msg.contains("null"),
        "optional 'cwd' parameter with its type must be listed: {msg}"
    );

    // @step And the error shows a complete example call with a <command> placeholder
    assert!(
        msg.contains("Example call:") && msg.contains("<command>"),
        "example call must exercise 'command' with a placeholder: {msg}"
    );
}

/// Scenario: Grep and AstGrep empty pattern errors name the tool and show full usage
#[tokio::test]
async fn scenario_grep_and_astgrep_empty_pattern_errors_include_recovery() {
    // @step Given I register the Grep and AstGrep direct tools
    let session = Uuid::new_v4();
    let grep_toolset = single_toolset(GrepTool::new(session));
    let astgrep_toolset = single_toolset(AstGrepTool::new(session));
    assert!(
        grep_toolset.contains("Grep"),
        "Grep must be registered under its registered name"
    );
    assert!(
        astgrep_toolset.contains("AstGrep"),
        "AstGrep must be registered under its registered name"
    );

    // @step When I call Grep with pattern set to the empty string
    let msg = toolset_error(&grep_toolset, "Grep", r#"{"pattern":""}"#).await;

    // @step Then the error contains the substring 'pattern parameter is required'
    assert!(
        msg.contains("pattern parameter is required"),
        "original message must stay intact: {msg}"
    );

    // @step And the error names the tool 'Grep'
    assert!(
        msg.contains("Accepted parameters for Grep"),
        "error must name the registered tool 'Grep': {msg}"
    );

    // @step And the error lists accepted parameters (pattern required; path, output_mode, glob, limit optional with types)
    assert!(
        msg.contains("pattern (string, required)"),
        "required 'pattern' parameter must be listed: {msg}"
    );
    for param in ["path", "output_mode", "glob", "limit"] {
        assert!(
            msg.contains(param),
            "optional parameter '{param}' must be listed: {msg}"
        );
    }

    // @step And the error shows a complete example call
    assert!(
        msg.contains("Example call:") && msg.contains("<pattern>"),
        "example call must exercise 'pattern': {msg}"
    );

    // @step When I call AstGrep with pattern set to the empty string
    let msg = toolset_error(
        &astgrep_toolset,
        "AstGrep",
        r#"{"pattern":"","language":"rust"}"#,
    )
    .await;

    // @step Then the error contains the substring 'pattern parameter is required'
    assert!(
        msg.contains("pattern parameter is required"),
        "original message must stay intact: {msg}"
    );

    // @step And the error names the tool 'AstGrep' and lists its accepted parameters and a complete example call
    assert!(
        msg.contains("Accepted parameters for AstGrep"),
        "error must name the registered tool 'AstGrep': {msg}"
    );
    assert!(
        msg.contains("pattern (string, required)") && msg.contains("language (string, required)"),
        "required AstGrep parameters must be listed: {msg}"
    );
    assert!(
        msg.contains("Example call:") && msg.contains("<pattern>"),
        "example call must exercise 'pattern': {msg}"
    );
}

/// Scenario: AstGrepRefactor empty source_file error names the tool and shows full usage
#[tokio::test]
async fn scenario_astgrep_refactor_empty_source_file_error_includes_recovery() {
    // @step Given I register the AstGrepRefactor direct tool
    let tool = AstGrepRefactorTool::new(Uuid::new_v4());
    let toolset = single_toolset(tool);
    assert!(
        toolset.contains("AstGrepRefactor"),
        "AstGrepRefactor must be registered"
    );

    // @step When I call AstGrepRefactor with source_file set to the empty string
    let msg = toolset_error(
        &toolset,
        "AstGrepRefactor",
        r#"{"pattern":"fn old($$$) { $$$ }","language":"rust","source_file":"","replacement":"//","preview":true}"#,
    )
    .await;

    // @step Then the error contains the substring 'source_file parameter is required'
    assert!(
        msg.contains("source_file parameter is required"),
        "original message must stay intact: {msg}"
    );

    // @step And the error names the tool 'AstGrepRefactor'
    assert!(
        msg.contains("Accepted parameters for AstGrepRefactor"),
        "error must name the registered tool 'AstGrepRefactor': {msg}"
    );

    // @step And the error lists accepted parameters (language, pattern, source_file required; replacement, batch, preview, transforms optional with types)
    for param in [
        "language (string, required)",
        "pattern (string, required)",
        "source_file (string, required)",
        "replacement",
        "batch",
        "preview",
        "transforms",
    ] {
        assert!(
            msg.contains(param),
            "accepted parameter entry '{param}' missing: {msg}"
        );
    }

    // @step And the error shows a complete example call
    assert!(
        msg.contains("Example call:") && msg.contains("source_file"),
        "example call must exercise 'source_file': {msg}"
    );
}

/// Scenario: DeepSearch empty query error is argument validation with full usage
#[tokio::test]
async fn scenario_deepsearch_empty_query_error_is_validation_with_recovery() {
    // @step Given I register the DeepSearch direct tool
    let tool = DeepSearchTool::new(Uuid::new_v4());
    let toolset = single_toolset(tool);
    assert!(
        toolset.contains("DeepSearch"),
        "DeepSearch must be registered"
    );

    // @step When I call DeepSearch with query set to the empty string
    let result = toolset
        .call("DeepSearch", r#"{"query":""}"#.to_string())
        .await;
    assert!(
        result.is_err(),
        "expected an argument-validation error for an empty query"
    );
    let rig_err = result.expect_err("checked above");

    // @step Then the error contains the substring 'query is required and must not be empty'
    let msg = rig_err.to_string();
    assert!(
        msg.contains("query is required and must not be empty"),
        "original message must stay intact: {msg}"
    );

    // @step And the error names the tool 'DeepSearch'
    assert!(
        msg.contains("Accepted parameters for DeepSearch"),
        "error must name the registered tool 'DeepSearch': {msg}"
    );

    // @step And the error is an argument-validation failure, not an execution failure
    // (the inner codelet ToolError must be ToolError::Validation, not Execution)
    let inner = inner_tool_error(&rig_err);
    match inner {
        ToolError::Validation { .. } => {}
        other => panic!(
            "empty-query error must be ToolError::Validation (argument validation), \
             not an execution failure; got: {other:?}"
        ),
    }

    // @step And the error lists accepted parameters (query required; scope, max_depth, max_recursion_depth optional with types and defaults)
    assert!(
        msg.contains("query (string, required)"),
        "required 'query' parameter must be listed: {msg}"
    );
    for param in ["scope", "max_depth", "max_recursion_depth"] {
        assert!(
            msg.contains(param),
            "optional parameter '{param}' must be listed: {msg}"
        );
    }

    // @step And the error shows a complete example call
    assert!(
        msg.contains("Example call:") && msg.contains("<query>"),
        "example call must exercise 'query': {msg}"
    );
}

/// Scenario: RequestUserInput empty questions error names the tool and shows full usage
#[tokio::test]
async fn scenario_request_user_input_empty_questions_error_includes_recovery() {
    // @step Given I register the RequestUserInput direct tool
    let session = Uuid::new_v4();
    let handler: HitlHandler =
        Arc::new(move |_, _| Ok(HitlResponse::Cancelled { cancelled: true }));
    set_hitl_handler(session, Some(handler));
    let tool = RequestUserInputTool::new(session);
    let toolset = single_toolset(tool);
    assert!(
        toolset.contains("request_user_input"),
        "RequestUserInputTool must be registered under its registered name"
    );

    // @step When I call RequestUserInput with an empty questions array
    let msg = toolset_error(&toolset, "request_user_input", r#"{"questions":[]}"#).await;

    // @step Then the error contains the substring 'questions array must not be empty'
    assert!(
        msg.contains("questions array must not be empty"),
        "original message must stay intact: {msg}"
    );

    // @step And the error names the tool 'RequestUserInput'
    // (the tool self-identifies with its registered rig name
    // 'request_user_input' — not a bare internal alias)
    assert!(
        msg.contains("Accepted parameters for request_user_input"),
        "error must name the tool by its registered name 'request_user_input': {msg}"
    );

    // @step And the error lists accepted parameters (questions array required, maximum 3; each question id, header, question required; options optional)
    assert!(
        msg.contains("questions (array"),
        "'questions' parameter must be listed with its array type: {msg}"
    );
    for param in ["id", "header", "question", "options"] {
        assert!(
            msg.contains(param),
            "question sub-parameter '{param}' must be listed: {msg}"
        );
    }

    // @step And the error shows a complete example call
    assert!(
        msg.contains("Example call:") && msg.contains("questions"),
        "example call must exercise 'questions': {msg}"
    );

    clear_all_hitl_handlers();
}

/// Scenario: Schedule add without cron returns a tool error with full usage
#[tokio::test]
async fn scenario_schedule_add_without_cron_returns_tool_error_with_recovery() {
    // @step Given I register the Schedule direct tool with a schema for actions add, list, pause, resume, remove
    let session = Uuid::new_v4();
    // Handler mirroring the agent-loop/napi validation order (fail-fast,
    // same messages, before any file I/O) so the tool's LLM-visible error
    // is observable in the test.
    let handler: ScheduleHandler = Arc::new(|req: ScheduleRequest| {
        if req.action == "add" {
            let has = |v: &Option<String>| v.as_ref().is_some_and(|s| !s.is_empty());
            if !has(&req.name) {
                return ScheduleResult::error("Schedule name is required");
            }
            if !has(&req.cron) {
                return ScheduleResult::error("Cron expression is required");
            }
            if !has(&req.timezone) {
                return ScheduleResult::error("Timezone is required");
            }
            if !has(&req.job_type) {
                return ScheduleResult::error("Job type is required");
            }
            match req.job_type.as_deref() {
                Some("agent") => {
                    if !has(&req.role) || !has(&req.prompt) {
                        return ScheduleResult::error("Agent jobs require role and prompt fields");
                    }
                }
                Some("shell") => {
                    if !has(&req.command) {
                        return ScheduleResult::error("Shell jobs require a command field");
                    }
                }
                Some(other) => {
                    return ScheduleResult::error(&format!(
                        "Invalid job_type: {other}. Must be 'agent' or 'shell'"
                    ));
                }
                None => unreachable!("job_type checked above"),
            }
        }
        ScheduleResult::error("handler stub: action not implemented in test")
    });
    set_schedule_handler(session, Some(handler));
    let tool = ScheduleTool::new(session);
    let toolset = single_toolset(tool);
    assert!(toolset.contains("Schedule"), "Schedule must be registered");

    // @step When I call Schedule with action add and a name but no cron expression
    let result = toolset
        .call(
            "Schedule",
            r#"{"action":"add","name":"nightly","timezone":"UTC","job_type":"shell","command":"echo hi"}"#
                .to_string(),
        )
        .await;

    // @step Then the LLM-visible error is not the raw serialized JSON blob {"success":false,"error":"..."}
    assert!(
        result.is_err(),
        "expected a tool error, got the serialized ScheduleResult JSON"
    );
    let msg = result.expect_err("checked above").to_string();
    assert!(
        !msg.starts_with('{') && !msg.contains("\"success\""),
        "the LLM-visible error must be a tool error, not serialized JSON: {msg}"
    );

    // @step And the error contains the substring 'Cron expression is required'
    assert!(
        msg.contains("Cron expression is required"),
        "original handler message must stay intact: {msg}"
    );

    // @step And the error names the tool 'Schedule'
    assert!(
        msg.contains("Accepted parameters for Schedule"),
        "error must name the registered tool 'Schedule': {msg}"
    );

    // @step And the error states that 'cron' is required for the add action
    assert!(
        msg.contains("add action") && msg.contains("cron"),
        "error must state that cron is required for the add action: {msg}"
    );

    // @step And the error lists all accepted parameters with types and conditionally-required notes (name, cron, timezone, job_type, role, prompt, command, overlap_policy with default skip)
    for param in [
        "action",
        "name",
        "cron",
        "timezone",
        "job_type",
        "role",
        "prompt",
        "command",
        "overlap_policy",
    ] {
        assert!(
            msg.contains(param),
            "parameter '{param}' must be listed in the reference: {msg}"
        );
    }
    assert!(
        msg.contains("skip"),
        "overlap_policy default 'skip' must be surfaced: {msg}"
    );

    // @step And the error shows a complete example add call
    assert!(
        msg.contains("Example call:") && msg.contains("\"action\": \"add\""),
        "example call must exercise the add action: {msg}"
    );

    clear_all_schedule_handlers();
}

/// Scenario: Done tool arg error uses the registered tool name
#[tokio::test]
async fn scenario_done_tool_arg_error_uses_registered_name() {
    // @step Given I register the Done direct tool whose registered name is 'Done'
    let tool = DoneTool::new(Uuid::new_v4());
    let toolset = single_toolset(tool);
    assert!(
        toolset.contains("Done"),
        "DoneTool must be registered under its registered name 'Done'"
    );

    // @step When I call Done with no arguments
    let result = toolset.call("Done", "{}".to_string()).await;
    assert!(
        result.is_err(),
        "expected an argument error for a missing summary"
    );
    let msg = result.expect_err("checked above").to_string();

    // @step Then the error names the tool 'Done', not a lowercase alias
    assert!(
        msg.contains("Done tool") || msg.contains("Accepted parameters for Done"),
        "error must name the registered tool 'Done': {msg}"
    );
    assert!(
        !msg.contains("Accepted parameters for done") && !msg.contains("done tool"),
        "error must not name the lowercase alias: {msg}"
    );

    // @step And the error keeps the existing missing-summary message substring intact
    assert!(
        msg.contains("missing field `summary`") || msg.contains("summary"),
        "the summary-required message substring must stay intact: {msg}"
    );

    // @step And the error lists accepted parameters (summary string required; evidence array; goal_assessment string)
    assert!(
        msg.contains("summary (string, required)"),
        "required 'summary' parameter must be listed: {msg}"
    );
    assert!(
        msg.contains("evidence (array") && msg.contains("goal_assessment (string"),
        "optional parameters must be listed with types: {msg}"
    );

    // @step And the error shows a complete example call
    assert!(
        msg.contains("Example call:") && msg.contains("summary"),
        "example call must exercise 'summary': {msg}"
    );
}

// =============================================================================
// tool_usage renderer scenarios (rule 5)
// =============================================================================

/// Scenario: oneOf with non-object variants renders each variant type
#[test]
fn scenario_oneof_non_object_variants_render_type_labels() {
    // @step Given a tool whose schema declares a required property 'session_id' as oneOf string or array of string
    let schema = json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["spawn", "get_status"]
            },
            "session_id": {
                "oneOf": [
                    { "type": "string" },
                    { "type": "array", "items": { "type": "string" } }
                ]
            }
        },
        "required": ["action"]
    });

    // @step When I call the tool with session_id missing
    // (the parameter reference is what the recovery block renders for the
    // missing-parameter error — rendering it directly is the observable
    // behavior of the scenario)
    let out = render_parameter_reference("AgentManager", &schema);

    // @step Then the parameter reference lists session_id with both allowed types (string and array of string)
    assert!(
        out.contains("session_id (string | array of string"),
        "both allowed types must be listed: {out}"
    );

    // @step And the parameter reference contains no empty object placeholder for a variant that has no properties
    assert!(
        !out.contains("* {}"),
        "empty-object placeholder must not be emitted: {out}"
    );
}

/// Scenario: anyOf properties render their alternatives
#[test]
fn scenario_anyof_properties_render_alternatives() {
    // @step Given a tool whose schema declares properties 'action' and 'transport' as anyOf unions
    // (mirrors the schemars-generated ConnectMCP schema: $ref to a oneOf enum
    // definition, plus a null alternative for transport)
    let schema = json!({
        "type": "object",
        "properties": {
            "action": {
                "default": "connect",
                "allOf": [{ "$ref": "#/definitions/McpAction" }]
            },
            "transport": {
                "anyOf": [
                    { "$ref": "#/definitions/McpTransport" },
                    { "type": "null" }
                ]
            }
        },
        "definitions": {
            "McpAction": {
                "oneOf": [
                    { "type": "string", "enum": ["connect"] },
                    { "type": "string", "enum": ["disconnect"] },
                    { "type": "string", "enum": ["list"] }
                ]
            },
            "McpTransport": {
                "oneOf": [
                    { "type": "string", "enum": ["stdio"] },
                    { "type": "string", "enum": ["http"] }
                ]
            }
        }
    });

    // @step When I call the tool with an invalid 'action' value
    // (invalid-enum failures surface the same parameter reference —
    // rendering it directly is the observable behavior of the scenario)
    let out = render_parameter_reference("ConnectMCP", &schema);

    // @step Then the parameter reference describes each alternative of 'action' (e.g. the action shape with its required type and url fields)
    assert!(
        out.contains("connect") && out.contains("disconnect") && out.contains("list"),
        "each action alternative must be described: {out}"
    );
    assert!(
        out.contains("stdio") && out.contains("http"),
        "each transport alternative must be described: {out}"
    );

    // @step And the parameter reference does not render 'action' or 'transport' as bare '(any)' with no sub-values
    assert!(
        !out.contains("(any)"),
        "bare '(any)' with no sub-values must not be emitted: {out}"
    );
}
