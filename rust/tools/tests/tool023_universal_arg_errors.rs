#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/tool-call-arg-errors-include-recovery-guidance-all-tools.feature
//
// TOOL-023 (universal scope): every tool's argument error must be a
// self-contained recovery surface — it names the tool, states which field
// was missing or wrong, lists the tool's accepted parameters (full usage),
// and shows at least one complete canonical example call.
//
// Scenarios below map 1:1 to the universal-scope Gherkin scenarios in the
// feature file; every Gherkin step is mirrored by an @step comment.

use codelet_tools::facade::{ClaudeWebSearchFacade, FacadeToolWrapper};
use codelet_tools::{ReadTool, WebSearchTool};
use rig::tool::ToolSet;
use uuid::Uuid;

/// Scenario: Calling a direct tool with no arguments names the tool and lists accepted parameters
#[tokio::test]
async fn scenario_direct_tool_no_args_names_tool_and_lists_params() {
    // @step Given I have a direct rig tool registered (e.g. WebSearchTool) whose schema declares a required 'action' parameter plus optional parameters
    let tool = WebSearchTool::new(Uuid::new_v4());
    let toolset = ToolSet::from_tools(vec![tool]);
    assert!(
        toolset.contains("WebSearch"),
        "WebSearchTool must be registered in the ToolSet"
    );

    // @step When I call the tool with an empty argument object (missing required field)
    let result = toolset.call("WebSearch", "{}".to_string()).await;
    assert!(
        result.is_err(),
        "expected an argument error for an empty argument object"
    );
    let msg = result.expect_err("checked above").to_string();

    // @step Then the error returned to the LLM names the tool (e.g. 'WebSearch')
    assert!(
        msg.contains("WebSearch"),
        "error must name the tool 'WebSearch': {msg}"
    );

    // @step And the error states which required parameter is missing ('action')
    assert!(
        msg.contains("missing field `action`"),
        "error must state the missing required field 'action': {msg}"
    );

    // @step And the error lists the tool's accepted parameters from its schema (required and optional, with types and valid values where declared)
    assert!(
        msg.contains("Accepted parameters for WebSearch"),
        "error must list the accepted parameters: {msg}"
    );
    assert!(
        msg.contains("action (object, required)"),
        "required 'action' parameter with its type must be listed: {msg}"
    );
    assert!(
        msg.contains("capture_screenshot"),
        "oneOf variant values must be listed: {msg}"
    );

    // @step And the error shows at least one complete canonical example call that would succeed
    assert!(
        msg.contains("Example call:"),
        "error must show a canonical example call: {msg}"
    );
    assert!(
        msg.contains("\"action\""),
        "example call must exercise the required 'action' parameter: {msg}"
    );
}

/// Scenario: Facade tool missing a required parameter names the tool and shows full usage
#[tokio::test]
async fn scenario_facade_tool_missing_required_param_shows_full_usage() {
    // @step Given I call a provider-facade tool (e.g. the Claude WebSearch facade) with arguments missing a required field
    let session_id = Uuid::new_v4();
    let wrapper = FacadeToolWrapper::new(std::sync::Arc::new(ClaudeWebSearchFacade), session_id);
    let toolset = ToolSet::from_tools(vec![wrapper]);

    // @step When the facade maps the provider parameters and finds the required field missing
    let result = toolset.call("WebSearch", "{}".to_string()).await;
    assert!(
        result.is_err(),
        "expected a validation error for the missing 'action_type' field"
    );
    let msg = result.expect_err("checked above").to_string();

    // @step Then the validation error names the tool and the missing parameter
    assert!(
        msg.contains("WebSearch"),
        "error must name the tool 'WebSearch': {msg}"
    );
    assert!(
        msg.contains("action_type"),
        "error must name the missing parameter 'action_type': {msg}"
    );

    // @step And the error includes the tool's accepted parameters (required and optional with types, defaults, and enums) rendered from its definition
    assert!(
        msg.contains("Accepted parameters for WebSearch"),
        "error must include the accepted-parameter reference: {msg}"
    );
    assert!(
        msg.contains("open_page") && msg.contains("find_in_page"),
        "enum values for 'action_type' must be listed: {msg}"
    );
    assert!(
        msg.contains("query"),
        "optional 'query' parameter must be listed: {msg}"
    );

    // @step And the error shows at least one complete canonical example call that would succeed
    assert!(
        msg.contains("Example call:"),
        "error must show a canonical example call: {msg}"
    );
}

/// Scenario: Argument error for any tool (not just Fspec) names the tool, the missing field, and the full usage
#[tokio::test]
async fn scenario_any_tool_arg_error_includes_full_usage() {
    // @step Given I have any agent-exposed tool registered (e.g. WebSearch, Read, Bash, or a facade tool)
    let tool = ReadTool::new(Uuid::new_v4());
    let toolset = ToolSet::from_tools(vec![tool]);
    assert!(
        toolset.contains("Read"),
        "ReadTool must be registered in the ToolSet"
    );

    // @step When I call the tool with arguments missing a required field (e.g. WebSearch called with no parameters at all)
    let result = toolset.call("Read", "{}".to_string()).await;
    assert!(
        result.is_err(),
        "expected an argument error for an empty argument object"
    );
    let msg = result.expect_err("checked above").to_string();

    // @step Then the LLM-visible error does NOT stop at the raw serde passthrough (e.g. it is not just 'JSON error: missing field `action` at line 1 column 2')
    assert!(
        msg.contains("Accepted parameters for Read"),
        "error must go beyond the raw serde passthrough: {msg}"
    );

    // @step And the error names the tool (e.g. 'WebSearch')
    assert!(
        msg.contains("Read tool"),
        "error must name the tool 'Read': {msg}"
    );

    // @step And the error states which field(s) are required and were missing or invalid (e.g. 'action')
    assert!(
        msg.contains("missing field `file_path`"),
        "error must state the missing required field 'file_path': {msg}"
    );

    // @step And the error lists the tool's full accepted parameters with types, valid values (enums/oneOf), required-vs-optional, and defaults
    assert!(
        msg.contains("file_path (string, required)"),
        "required 'file_path' parameter with its type must be listed: {msg}"
    );
    assert!(
        msg.contains("offset") && msg.contains("limit") && msg.contains("pdf_mode"),
        "optional parameters must be listed: {msg}"
    );

    // @step And the error includes at least one complete canonical example call showing how to invoke the tool correctly
    assert!(
        msg.contains("Example call:"),
        "error must show a canonical example call: {msg}"
    );
    assert!(
        msg.contains("file_path"),
        "example call must exercise the required 'file_path' parameter: {msg}"
    );
}
