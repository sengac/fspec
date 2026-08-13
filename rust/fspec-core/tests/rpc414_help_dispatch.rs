#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/fspec-tool-help-dispatch.feature
//!
//! RPC-414 — RED phase. These integration tests drive the real public
//! `codelet_fspec_core::dispatch_command` API to prove the help-routing
//! defect: today the native Rust dispatcher does an exact-match canonical
//! lookup on the whole command string, so every help shape falls through to
//! `UnknownCommand`. Each Gherkin scenario maps 1:1 to a `#[test]` below and
//! every Gherkin step carries a byte-for-byte `@step` comment.

use codelet_fspec_core::{dispatch_command, DispatchRequest};

/// Build a DispatchRequest for `command` with empty args. Help routing does
/// not touch the filesystem, so a throwaway (existing) temp path is fine.
fn help_req(command: &str) -> DispatchRequest {
    DispatchRequest {
        command: command.to_string(),
        args_json: "{}".to_string(),
        project_root: std::env::temp_dir(),
    }
}

/// Build a DispatchRequest with an explicit `args_json` payload.
fn req_with_args(command: &str, args_json: &str) -> DispatchRequest {
    DispatchRequest {
        command: command.to_string(),
        args_json: args_json.to_string(),
        project_root: std::env::temp_dir(),
    }
}

#[test]
fn embedded_help_flag_renders_per_command_usage_doc() {
    // Scenario: Embedded --help flag renders the per-command usage doc

    // @step Given the native Rust fspec dispatcher with no JS chunk callback registered
    // (the native dispatch path — `dispatch_command` — is the one under test; the
    //  standalone/TUI host reaches it exactly when no JS chunk callback is registered.)

    // @step When I dispatch a command "create-prefix --help"
    let result = dispatch_command(help_req("create-prefix --help"));

    // @step Then the dispatch result is successful
    assert!(
        result.success,
        "expected success=true for 'create-prefix --help', got {result:?}"
    );

    // @step And the output contains the create-prefix usage header
    // (format_command_help uppercases the command name as the header line.)
    assert!(
        result.data.contains("CREATE-PREFIX"),
        "output missing create-prefix usage header (CREATE-PREFIX): {}",
        result.data
    );

    // @step And the output lists the positional arguments prefix and description
    assert!(
        result.data.contains("prefix"),
        "output missing positional argument 'prefix': {}",
        result.data
    );
    assert!(
        result.data.contains("description"),
        "output missing positional argument 'description': {}",
        result.data
    );
}

#[test]
fn embedded_short_help_flag_renders_same_usage_doc_as_long_flag() {
    // Scenario: Embedded -h flag renders the same usage doc as --help

    // @step Given the native Rust fspec dispatcher with no JS chunk callback registered

    // @step When I dispatch a command "create-prefix -h"
    let short = dispatch_command(help_req("create-prefix -h"));

    // @step Then the dispatch result is successful
    assert!(
        short.success,
        "expected success=true for 'create-prefix -h', got {short:?}"
    );

    // @step And the output is identical to the output of dispatching "create-prefix --help"
    let long = dispatch_command(help_req("create-prefix --help"));
    assert_eq!(
        short.data, long.data,
        "'create-prefix -h' output must equal 'create-prefix --help' output"
    );
}

#[test]
fn help_command_with_args_command_renders_per_command_usage_doc() {
    // Scenario: help command with an args command field renders the per-command usage doc

    // @step Given the native Rust fspec dispatcher with no JS chunk callback registered

    // @step When I dispatch a command "help" with args command "create-prefix"
    let result = dispatch_command(req_with_args("help", "{\"command\":\"create-prefix\"}"));

    // @step Then the dispatch result is successful
    assert!(
        result.success,
        "expected success=true for help with args.command=create-prefix, got {result:?}"
    );

    // @step And the output is identical to the output of dispatching "create-prefix --help"
    let direct = dispatch_command(help_req("create-prefix --help"));
    assert_eq!(
        result.data, direct.data,
        "help {{command:create-prefix}} output must equal 'create-prefix --help' output"
    );
}

#[test]
fn help_command_with_no_args_renders_general_fspec_tool_help() {
    // Scenario: help command with no args renders general Fspec tool help

    // @step Given the native Rust fspec dispatcher with no JS chunk callback registered

    // @step When I dispatch a command "help" with no args
    let result = dispatch_command(req_with_args("help", "{}"));

    // @step Then the dispatch result is successful
    assert!(
        result.success,
        "expected success=true for bare 'help', got {result:?}"
    );

    // @step And the output explains how to get per-command help
    // (Robust, non-brittle assertion: general help must reference the "--help"
    //  per-command discovery mechanism it is instructing the agent to use.)
    assert!(
        result.data.contains("--help"),
        "general help must explain how to get per-command help (mention '--help'): {}",
        result.data
    );
}

#[test]
fn help_request_for_unknown_command_fails_naming_stripped_command() {
    // Scenario: Help request for an unknown command fails naming the stripped command

    // @step Given the native Rust fspec dispatcher with no JS chunk callback registered

    // @step When I dispatch a command "nonexistent-xyz --help"
    let result = dispatch_command(help_req("nonexistent-xyz --help"));

    // @step Then the dispatch result is a failure
    assert!(
        !result.success,
        "expected success=false for 'nonexistent-xyz --help', got {result:?}"
    );
    let msg = result
        .error
        .as_ref()
        .expect("expected an error message for unknown help target");

    // @step And the error message contains "Unknown fspec command"
    assert!(
        msg.contains("Unknown fspec command"),
        "missing 'Unknown fspec command' in error message: {msg}"
    );

    // @step And the error message names "nonexistent-xyz"
    // The stripped name (not the raw "nonexistent-xyz --help") must be named.
    assert!(
        msg.contains("nonexistent-xyz"),
        "error must name the stripped command 'nonexistent-xyz': {msg}"
    );
    assert!(
        !msg.contains("nonexistent-xyz --help"),
        "error must name the STRIPPED command, not the raw '<name> --help' string: {msg}"
    );
}

#[test]
fn help_request_for_real_command_without_config_degrades_gracefully() {
    // Scenario: Help request for a real command without a CONFIG degrades gracefully

    // @step Given the native Rust fspec dispatcher with no JS chunk callback registered

    // @step When I dispatch a command "board --help"
    // (board is a real canonical command that intentionally ships no help CONFIG.)
    let result = dispatch_command(help_req("board --help"));

    // @step Then the dispatch result is successful
    assert!(
        result.success,
        "expected success=true (graceful) for 'board --help', got {result:?}"
    );

    // @step And the output states no detailed help is available for board
    assert!(
        result.data.contains("board"),
        "graceful message must name the command 'board': {}",
        result.data
    );
    assert!(
        result.data.to_lowercase().contains("no detailed help"),
        "graceful message must state no detailed help is available: {}",
        result.data
    );
    // Must be a graceful success, NOT an UnknownCommand error.
    assert!(
        !result.data.contains("Unknown fspec command"),
        "'board --help' must degrade gracefully, not report Unknown fspec command: {}",
        result.data
    );
}

#[test]
fn normal_command_dispatch_is_not_intercepted_by_help_routing() {
    // Scenario: Normal command dispatch is not intercepted by help routing

    // @step Given the native Rust fspec dispatcher with no JS chunk callback registered

    // @step When I dispatch a command "create-prefix" with valid create-prefix args
    // create-prefix requires a prefix + description; supply them positionally.
    let result = dispatch_command(req_with_args(
        "create-prefix",
        "{\"_\":[\"TEST\",\"desc\"]}",
    ));

    // @step Then help routing does not intercept the command
    // RED-phase reasoning: help routing does not yet exist, so this already
    // holds today; post-GREEN it must STILL hold because help routing must
    // never intercept a bare command with no help flag. The observable proof
    // that the help path was NOT taken is that the returned data is not the
    // rendered create-prefix usage doc (whose header is "CREATE-PREFIX").
    assert!(
        !result.data.contains("CREATE-PREFIX"),
        "bare 'create-prefix' must NOT be intercepted by help routing (no usage doc): {}",
        result.data
    );

    // @step And the command is dispatched through the normal ported or stub path
    // create-prefix is a ported command: it runs its real dispatch path (which
    // either succeeds or returns a normal command error) rather than a help doc.
    // Either way the result is a genuine dispatch outcome, never the help usage
    // header asserted against above.
    let is_normal_dispatch_outcome = result.success || result.error.is_some();
    assert!(
        is_normal_dispatch_outcome,
        "bare 'create-prefix' must yield a normal dispatch outcome, got {result:?}"
    );
}
