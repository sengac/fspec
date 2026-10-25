//! Feature: spec/features/responses-terminal-semantics.feature

#[path = "support/bug262_chat.rs"]
mod chat;
#[path = "support/bug264_responses.rs"]
mod responses;
#[path = "support/prov161_runtime.rs"]
mod runtime;

use responses::{consume, event, queued_tools, run_done, terminal, usage};
use runtime::TestResult;
use serde_json::{json, Value};
use std::error::Error;

#[tokio::test]
async fn failed_responses_retain_backend_causes_without_tools() -> TestResult {
    // @step Given a failed Responses terminal with a backend code and message and queued tools
    let body = queued_tools() + terminal("failed", Value::Null).as_str();
    // @step When its real local stream is consumed
    let observed = consume(body).await?;
    // @step Then the typed failure cause survives without successful final or executable calls
    assert_eq!(observed.errors.len(), 1, "{observed:?}");
    let error = &observed.errors[0];
    assert!(
        error.to_string().contains("rejected tool schema"),
        "{error}"
    );
    assert!(error.to_string().contains("invalid_schema"), "{error}");
    let mut source = error.source();
    let mut typed = false;
    while let Some(cause) = source {
        typed |= cause.to_string().contains("rejected tool schema");
        source = cause.source();
    }
    assert!(typed, "backend cause was flattened: {error}");
    assert!(observed.finals.is_empty());
    assert_eq!(observed.calls, 0);
    Ok(())
}

#[tokio::test]
async fn incomplete_responses_preserve_normalized_recovery_metadata() -> TestResult {
    // @step Given max output or content filtered incomplete terminals with consistent usage and queued tools
    for (reason, stop) in [
        ("max_output_tokens", "max_tokens"),
        ("content_filter", "content_filter"),
    ] {
        let terminal: Value = serde_json::from_str(
            terminal("incomplete", usage())
                .trim()
                .trim_start_matches("data: "),
        )?;
        let mut terminal = terminal;
        terminal["response"]["incomplete_details"]["reason"] = json!(reason);
        // @step When their real local streams are consumed
        let observed = consume(queued_tools() + event(terminal).as_str()).await?;
        // @step Then the final metadata preserves the incomplete reason and exact usage without executable calls
        assert!(observed.errors.is_empty(), "{observed:?}");
        assert_eq!(observed.finals, [(50, Some(stop.into()))]);
        assert_eq!(observed.calls, 0);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn invalid_terminal_evidence_cannot_be_hidden_by_native_done() -> TestResult {
    // @step Given an attributed armed Done call queued before missing or inconsistent terminal usage
    for tokens in [
        Value::Null,
        json!({"input_tokens":20,"output_tokens":30,"total_tokens":1}),
    ] {
        // @step When the response passes through the real shared stream loop
        // @step Then Done is not accepted and no trailing effect or ordinary nudge occurs
        run_done(
            queued_tools() + terminal("completed", tokens).as_str(),
            false,
        )
        .await?;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn completed_native_done_settles_once_with_paired_invocation_identities() -> TestResult {
    // @step Given a valid Responses terminal containing native Done and trailing work
    let body = queued_tools() + terminal("completed", usage()).as_str();
    // @step When the response passes through normal admission and the shared stream loop
    // @step Then physical usage is billed once before accepted Done and its call ID remains paired while trailing work is cancelled
    run_done(body, true).await
}

#[tokio::test]
async fn explicit_backend_errors_and_metadata_completions_retain_their_contracts() -> TestResult {
    // @step Given an explicit backend error or a valid metadata only completed terminal
    let body = event(
        json!({"type":"error","error":{"message":"invalid schema","code":"invalid_function_parameters"}}),
    );
    // @step When the real local Responses request is consumed
    let failed = consume(body).await?;
    let completed = consume(terminal("completed", usage())).await?;
    // @step Then the error is actionable or the completion contains exact usage without inventing tools
    assert_eq!(failed.errors.len(), 1);
    assert!(failed.errors[0]
        .to_string()
        .contains("invalid_function_parameters"));
    assert!(failed.finals.is_empty());
    assert!(completed.errors.is_empty(), "{completed:?}");
    assert_eq!(completed.finals.len(), 1);
    assert_eq!(completed.finals[0].0, 50);
    assert_eq!(failed.calls + completed.calls, 0);
    Ok(())
}
