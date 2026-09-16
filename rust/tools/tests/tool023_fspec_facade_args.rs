#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/tool-call-arg-errors-include-recovery-guidance.feature
//
// This test file validates the Fspec-provider-facade half of TOOL-023: the
// Claude/OpenAI/Gemini/ZAI Fspec facades must detect the string-vs-object
// `args` gotcha and the non-string `project_root` mistake BEFORE serde
// deserialization, and emit dedicated, tool-named validation errors with a
// corrected example. Scenarios map directly to the Gherkin scenarios in the
// feature file above; every Gherkin step is mirrored by an @step comment.

use codelet_tools::facade::{
    ClaudeFspecFacade, FspecToolFacade, GeminiFspecFacade, InternalFspecParams, ZAIFspecFacade,
};
use serde_json::json;

fn validation_message(e: codelet_tools::ToolError) -> String {
    match e {
        codelet_tools::ToolError::Validation { message, .. } => message,
        other => panic!("expected ToolError::Validation, got: {other:?}"),
    }
}

/// Scenario: Claude facade rejects args sent as a JSON object with a dedicated explanation
#[test]
fn scenario_claude_facade_rejects_args_sent_as_json_object() {
    // @step Given I call the Claude Fspec facade with {"command":"board","args":{"status":"backlog"}}
    let input = json!({"command": "board", "args": {"status": "backlog"}});

    // @step When the facade maps the provider parameters
    let result = ClaudeFspecFacade.map_params(input);
    assert!(
        result.is_err(),
        "expected a validation error for object-shaped args"
    );
    let msg = validation_message(result.expect_err("checked above"));

    // @step Then the facade returns a validation error
    // (asserted by validation_message's panic-on-non-Validation)

    // @step And the error message names the tool "Fspec"
    assert!(
        msg.contains("Fspec"),
        "error must name the tool 'Fspec': {msg}"
    );

    // @step And the error message says the "args" parameter must be a string containing JSON, not a JSON object
    assert!(
        msg.contains("must be a string containing JSON, not a JSON object"),
        "missing string-vs-object explanation: {msg}"
    );

    // @step And the error message shows a corrected example using args as a JSON string
    assert!(
        msg.contains("\"args\": \"{\\\"status\\\": \\\"backlog\\\"}\""),
        "missing corrected example: {msg}"
    );

    // @step And the error message does NOT contain the substring "Invalid arguments"
    assert!(
        !msg.contains("Invalid arguments"),
        "dedicated error must not use the old double-wrapped prefix: {msg}"
    );
}

/// Scenario: Claude facade rejects non-string project_root with a type explanation
#[test]
fn scenario_claude_facade_rejects_non_string_project_root() {
    // @step Given I call the Claude Fspec facade with {"command":"board","args":"{}","project_root":42}
    let input = json!({"command": "board", "args": "{}", "project_root": 42});

    // @step When the facade maps the provider parameters
    let result = ClaudeFspecFacade.map_params(input);
    assert!(
        result.is_err(),
        "expected a validation error for numeric project_root"
    );
    let msg = validation_message(result.expect_err("checked above"));

    // @step Then the facade returns a validation error
    // (asserted by validation_message's panic-on-non-Validation)

    // @step And the error message names the tool "Fspec"
    assert!(
        msg.contains("Fspec"),
        "error must name the tool 'Fspec': {msg}"
    );

    // @step And the error message says the "project_root" parameter must be a string
    assert!(
        msg.contains("project_root") && msg.contains("must be a string"),
        "missing project_root type explanation: {msg}"
    );
}

/// Scenario: Gemini facade args sent as an object gets the same dedicated explanation
#[test]
fn scenario_gemini_facade_args_object_gets_dedicated_explanation() {
    // @step Given I call the Gemini Fspec facade with {"command":"board","args":{"status":"backlog"}}
    let input = json!({"command": "board", "args": {"status": "backlog"}});

    // @step When the facade maps the provider parameters
    let result = GeminiFspecFacade.map_params(input);
    assert!(
        result.is_err(),
        "expected a validation error for object-shaped args"
    );
    let msg = validation_message(result.expect_err("checked above"));

    // @step Then the facade returns a validation error
    // (asserted by validation_message's panic-on-non-Validation)

    // @step And the error message names the tool "fspec_command"
    assert!(
        msg.contains("fspec_command"),
        "error must name the tool 'fspec_command': {msg}"
    );

    // @step And the error message says the "args" parameter must be a string containing JSON, not a JSON object
    assert!(
        msg.contains("must be a string containing JSON, not a JSON object"),
        "missing string-vs-object explanation: {msg}"
    );
}

/// Scenario: ZAI facade arguments sent as an object gets the same dedicated explanation
#[test]
fn scenario_zai_facade_arguments_object_gets_dedicated_explanation() {
    // @step Given I call the ZAI Fspec facade with {"command":"board","arguments":{"status":"backlog"}}
    let input = json!({"command": "board", "arguments": {"status": "backlog"}});

    // @step When the facade maps the provider parameters
    let result = ZAIFspecFacade.map_params(input);
    assert!(
        result.is_err(),
        "expected a validation error for object-shaped arguments"
    );
    let msg = validation_message(result.expect_err("checked above"));

    // @step Then the facade returns a validation error
    // (asserted by validation_message's panic-on-non-Validation)

    // @step And the error message names the tool "run_fspec"
    assert!(
        msg.contains("run_fspec"),
        "error must name the tool 'run_fspec': {msg}"
    );

    // @step And the error message says the "arguments" parameter must be a string containing JSON, not a JSON object
    assert!(
        msg.contains("arguments")
            && msg.contains("must be a string containing JSON, not a JSON object"),
        "missing string-vs-object explanation: {msg}"
    );
}

/// Scenario: Missing command field error includes a usage example
#[test]
fn scenario_missing_command_field_error_includes_usage_example() {
    // @step Given I call the Gemini Fspec facade with {"args":"{}"}
    let input = json!({"args": "{}"});

    // @step When the facade maps the provider parameters and finds command missing
    let result = GeminiFspecFacade.map_params(input);
    assert!(
        result.is_err(),
        "expected a validation error for missing command"
    );
    let msg = validation_message(result.expect_err("checked above"));

    // @step Then the facade returns a validation error
    // (asserted by validation_message's panic-on-non-Validation)

    // @step And the error message names the tool "fspec_command"
    assert!(
        msg.contains("fspec_command"),
        "error must name the tool 'fspec_command': {msg}"
    );

    // @step And the error message includes an example invocation with command and args
    assert!(
        msg.contains("\"command\"") && msg.contains("\"args\""),
        "missing example invocation with command and args: {msg}"
    );

    // @step And the error message mentions the "help" command for available commands
    assert!(
        msg.contains("help"),
        "missing mention of the help command: {msg}"
    );
}

/// Scenario: Facades pass unknown command names through unchanged
#[test]
fn scenario_facades_pass_unknown_command_names_through_unchanged() {
    // @step Given I call the Claude Fspec facade with {"command":"list-workunit","args":"{}"}
    let input = json!({"command": "list-workunit", "args": "{}"});

    // @step When the facade maps the provider parameters
    let params: InternalFspecParams = ClaudeFspecFacade
        .map_params(input)
        .expect("valid string args must map successfully");

    // @step Then the facade returns success with command "list-workunit"
    assert_eq!(params.command, "list-workunit");
    assert_eq!(params.args, "{}");

    // @step And the facade does NOT add or strip any hint text itself
    // (proxy assertion: the did-you-mean line only appears in the dispatcher
    // error; the mapped params carry no suggestion text)
    assert!(
        !params.command.contains("Did you mean") && !params.args.contains("Did you mean"),
        "facade must not inject suggestion text: {params:?}"
    );
}
