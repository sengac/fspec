//! Feature: spec/features/codex-terminal-error-history-preservation.feature

#[path = "fixtures/terminal_error_history.rs"]
mod fixture;

use fixture::{call, completed, history, rejection, transport, Harness, Output, TestResult};
use rig::message::{AssistantContent, Message, UserContent};
use serde_json::json;

/// @step Given a Codex tool has completed a real filesystem change
/// @step When the following inference receives a terminal provider rejection
/// @step Then the original tool call and genuine outcome remain unchanged
/// @step And the provider error is surfaced without claiming context was removed
/// @step And a later prompt replays the outcome exactly once without repeating the tool
#[tokio::test]
#[serial_test::serial]
async fn a_later_prompt_preserves_completed_work_after_a_provider_rejection() -> TestResult {
    for code in ["server_is_overloaded", "invalid_api_key", "bad_request"] {
        let mut harness = Harness::new(vec![
            completed(vec![call(1, false)]),
            rejection(code),
            transport::terminal("completed", None, None),
        ])
        .await?;
        let output = Output::default();
        let result = harness.run("write once", &output).await;
        let expected = output.results();
        assert_eq!(expected.len(), 1);
        harness.assert_retained(&expected)?;
        assert!(
            result.is_err(),
            "terminal failure must propagate explicitly"
        );
        output.assert_failure(code);
        harness.assert_continuation(&expected).await?;
        assert_eq!(
            harness
                .server
                .requests
                .lock()
                .map_err(|_| "requests poisoned")?
                .len(),
            3
        );
    }
    Ok(())
}

/// @step Given a Codex tool has completed a real filesystem change
/// @step When the following inference receives a terminal provider rejection
/// @step And the conversation and checkpoint are saved and cold reloaded
/// @step Then the original tool call and genuine outcome remain unchanged
/// @step And a later prompt replays the outcome exactly once without repeating the tool
#[tokio::test]
#[serial_test::serial]
async fn cold_resume_preserves_completed_work_after_overload() -> TestResult {
    let mut harness = Harness::new(vec![
        completed(vec![call(1, false)]),
        rejection("server_is_overloaded"),
        transport::terminal("completed", None, None),
    ])
    .await?;
    let output = Output::default();
    let _result = harness.run("write once", &output).await;
    let expected = output.results();
    assert_eq!(expected.len(), 1);
    harness.cold_reload()?;
    harness.assert_retained(&expected)?;
    harness.assert_continuation(&expected).await?;
    Ok(())
}

/// @step Given a Codex batch contains successful work and a genuine tool error
/// @step When the following inference receives a terminal provider rejection
/// @step Then all invocation identities and genuine outcomes remain unchanged
/// @step And a later prompt replays each outcome exactly once without repeating the tools
#[tokio::test]
#[serial_test::serial]
async fn a_rejected_continuation_preserves_every_completed_batch_outcome() -> TestResult {
    let mut harness = Harness::new(vec![
        completed(vec![call(1, false), call(2, true)]),
        rejection("server_is_overloaded"),
        transport::terminal("completed", None, None),
    ])
    .await?;
    let output = Output::default();
    let result = harness.run("write a batch", &output).await;
    let expected = output.results();
    assert_eq!(expected.len(), 2);
    assert!(expected[1].2.contains("genuine tool failure"), "{expected:?}");
    harness.assert_retained(&expected)?;
    assert!(result.is_err());
    harness.assert_continuation(&expected).await?;
    Ok(())
}

/// @step Given a new Codex conversation without tool calls
/// @step When inference receives a terminal provider rejection
/// @step Then the original user message remains in conversation history
/// @step And the provider error is surfaced without claiming context was removed
/// @step And a later prompt reaches the provider with both user messages
#[tokio::test]
#[serial_test::serial]
async fn a_rejected_first_request_preserves_the_user_message() -> TestResult {
    let mut harness = Harness::new(vec![
        rejection("server_is_overloaded"),
        transport::terminal("completed", None, None),
    ])
    .await?;
    let output = Output::default();
    assert!(harness.run("original user input", &output).await.is_err());
    assert!(
        matches!(harness.session.messages.first(), Some(Message::User { content })
        if content.iter().any(|item| matches!(item, UserContent::Text(text) if text.text == "original user input")))
    );
    output.assert_failure("server_is_overloaded");
    harness.run("next user input", &Output::default()).await?;
    let requests = harness
        .server
        .requests
        .lock()
        .map_err(|_| "requests poisoned")?;
    assert_eq!(requests.len(), 2);
    let input = requests[1]["input"].to_string();
    assert!(input.contains("original user input"));
    assert!(input.contains("next user input"));
    Ok(())
}

/// @step Given a Codex checkpoint contains a call without a genuine outcome
/// @step When I try to continue through the streaming consumer
/// @step Then the original call and checkpoint remain unchanged
/// @step And replay fails explicitly before sending a request
/// @step And no successful completion or invented tool outcome is published
#[tokio::test]
#[serial_test::serial]
async fn an_unresolved_committed_call_still_fails_replay_safely() -> TestResult {
    let mut harness = Harness::new(Vec::new()).await?;
    let history = history(&harness.session)?;
    history.commit(
        &json!({"model":"shared-model","input":[]}),
        &[call(1, false)],
    )?;
    harness.session.messages.push(Message::Assistant {
        id: None,
        content: rig::OneOrMany::one(AssistantContent::ToolCall(
            history
                .tool_call(&serde_json::from_value(call(1, false))?)?
                .ok_or("pending call missing")?,
        )),
    });
    let before_messages = harness.session.messages.clone();
    let before_checkpoint = history.snapshot()?;
    let output = Output::default();
    let error = harness
        .run("continue", &output)
        .await
        .err()
        .ok_or("unresolved call accepted")?;
    assert!(format!("{error:#}").contains("no recorded outcome"));
    assert_eq!(
        &harness.session.messages[..before_messages.len()],
        before_messages.as_slice()
    );
    assert_eq!(history.snapshot()?, before_checkpoint);
    assert!(harness
        .server
        .requests
        .lock()
        .map_err(|_| "requests poisoned")?
        .is_empty());
    assert!(output.results().is_empty());
    output.assert_failure("no recorded outcome");
    Ok(())
}
