//! COV-057: every typed content block (text, tool use, tool result with its
//! raw output metadata, thinking) and multi-part structure survives a real
//! save and cold restore of the session's message store.

use super::persisted::{
    assistant, assistant_of, fresh, reply, restore, save, text, tool_result, user, user_of, Outcome,
};
use super::*;
use codelet_core::persistence::{ToolUseResultMetadata, UserContent};
use serde_json::json;

fn tool_use(id: &str, name: &str, input: serde_json::Value) -> AssistantContent {
    AssistantContent::ToolUse {
        id: id.into(),
        call_id: None,
        name: name.into(),
        input,
    }
}

/// The wire `type` tag of every content part, in order.
fn types<T: serde::Serialize>(parts: &[T]) -> Outcome<Vec<String>> {
    parts
        .iter()
        .map(|part| {
            let value = serde_json::to_value(part)?;
            Ok(value["type"].as_str().ok_or("untyped part")?.to_owned())
        })
        .collect()
}

/// Save one envelope and return the single restored one.
fn round_trip(session: uuid::Uuid, envelope: MessageEnvelope) -> Outcome<MessageEnvelope> {
    save(session, &[envelope])?;
    let mut restored = restore(session)?;
    assert_eq!(restored.len(), 1, "restored {restored:?}");
    Ok(restored.remove(0))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_text_content_persisted_and_restored() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message containing text "Hello, world"
    let envelope = reply(assistant(vec![text("Hello, world")]));
    // @step When the session is saved and restored
    let restored = round_trip(session, envelope)?;
    let message = assistant_of(&restored)?;
    // @step Then the restored message has content type "text"
    assert_eq!(types(&message.content)?, ["text"]);
    // @step And the restored message text equals "Hello, world"
    assert_eq!(message.content, vec![text("Hello, world")]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_tool_use_content_persisted_and_restored() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message containing a tool use:
    let input = json!({"path": "/etc/config.json"});
    let call = tool_use("toolu_01AyTnm7YLfybhnhEhwwZvAY", "read_file", input.clone());
    // @step When the session is saved and restored
    let restored = round_trip(session, reply(assistant(vec![call])))?;
    let message = assistant_of(&restored)?;
    // @step Then the restored message has content type "tool_use"
    assert_eq!(types(&message.content)?, ["tool_use"]);
    let AssistantContent::ToolUse {
        id,
        name,
        input: restored_input,
        ..
    } = &message.content[0]
    else {
        return Err("tool use lost".into());
    };
    // @step And the restored tool use id equals "toolu_01AyTnm7YLfybhnhEhwwZvAY"
    assert_eq!(id, "toolu_01AyTnm7YLfybhnhEhwwZvAY");
    // @step And the restored tool use name equals "read_file"
    assert_eq!(name, "read_file");
    // @step And the restored tool use input equals {"path": "/etc/config.json"}
    assert_eq!(restored_input, &input);
    Ok(())
}

/// Save one tool result (optionally with raw metadata) and restore it.
async fn tool_result_round_trip(
    part: UserContent,
) -> Outcome<(String, String, bool, Option<ToolUseResultMetadata>)> {
    let (_directory, _manager, _, session) = fresh().await?;
    let restored = round_trip(session, user(vec![part]))?;
    let parts = user_of(&restored)?;
    assert_eq!(types(parts)?, ["tool_result"]);
    match &parts[0] {
        UserContent::ToolResult {
            tool_use_id,
            content,
            is_error,
            tool_use_result,
            ..
        } => Ok((
            tool_use_id.clone(),
            content.clone(),
            *is_error,
            tool_use_result.clone(),
        )),
        other => Err(format!("expected tool result: {other:?}").into()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_tool_result_content_persisted_and_restored() -> TestResult {
    // @step Given a session with a user message containing a tool result:
    let part = tool_result("toolu_01AyTnm7YLfybhnhEhwwZvAY", r#"{"port": 8080}"#, false);
    // @step When the session is saved and restored
    // @step Then the restored message has content type "tool_result"
    let (id, content, is_error, _) = tool_result_round_trip(part).await?;
    // @step And the restored tool result tool_use_id equals "toolu_01AyTnm7YLfybhnhEhwwZvAY"
    assert_eq!(id, "toolu_01AyTnm7YLfybhnhEhwwZvAY");
    // @step And the restored tool result content equals {"port": 8080}
    assert_eq!(content, r#"{"port": 8080}"#);
    // @step And the restored tool result is_error equals false
    assert!(!is_error);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_tool_error_result_persisted_with_error_flag() -> TestResult {
    // @step Given a session with a user message containing a tool result:
    let part = tool_result("toolu_xyz789", "Permission denied", true);
    // @step When the session is saved and restored
    // @step Then the restored message has content type "tool_result"
    let (id, content, is_error, _) = tool_result_round_trip(part).await?;
    assert_eq!(id, "toolu_xyz789");
    // @step And the restored tool result is_error equals true
    assert!(is_error);
    // @step And the restored tool result content equals "Permission denied"
    assert_eq!(content, "Permission denied");
    Ok(())
}

fn with_raw(part: UserContent, raw: ToolUseResultMetadata) -> UserContent {
    match part {
        UserContent::ToolResult {
            tool_use_id,
            call_id,
            content,
            is_error,
            ..
        } => UserContent::ToolResult {
            tool_use_id,
            call_id,
            content,
            is_error,
            tool_use_result: Some(raw),
        },
        other => other,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_tool_result_raw_output_metadata_preserved() -> TestResult {
    // @step Given a session with a user message containing a tool result:
    let part = tool_result("toolu_abc123", "formatted output", false);
    // @step And the tool result has raw metadata:
    let part = with_raw(
        part,
        ToolUseResultMetadata::with_output("raw stdout content", "warning: deprecated"),
    );
    // @step When the session is saved and restored
    let (_, content, _, raw) = tool_result_round_trip(part).await?;
    assert_eq!(content, "formatted output");
    // @step Then the restored tool result has raw metadata
    let raw = raw.ok_or("raw metadata lost")?;
    // @step And the raw metadata stdout equals "raw stdout content"
    assert_eq!(raw.stdout.as_deref(), Some("raw stdout content"));
    // @step And the raw metadata stderr equals "warning: deprecated"
    assert_eq!(raw.stderr.as_deref(), Some("warning: deprecated"));
    // @step And the raw metadata interrupted equals false
    assert!(!raw.interrupted && !raw.is_image);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_interrupted_tool_execution_preserved() -> TestResult {
    // @step Given a session with a user message containing a tool result:
    let part = tool_result("toolu_abc123", "partial output", true);
    // @step And the tool result has raw metadata:
    let raw = ToolUseResultMetadata {
        interrupted: true,
        ..ToolUseResultMetadata::default()
    };
    // @step When the session is saved and restored
    let (_, content, is_error, raw) = tool_result_round_trip(with_raw(part, raw)).await?;
    assert_eq!(content, "partial output");
    // @step Then the raw metadata interrupted equals true
    assert!(raw.ok_or("raw metadata lost")?.interrupted);
    // @step And the tool result is_error equals true
    assert!(is_error);
    Ok(())
}

/// Save one thinking block and return its restored text and signature.
async fn thinking_round_trip(
    thinking: &str,
    signature: Option<&str>,
) -> Outcome<(String, Option<String>)> {
    let (_directory, _manager, _, session) = fresh().await?;
    let part = AssistantContent::Thinking {
        thinking: thinking.into(),
        signature: signature.map(Into::into),
    };
    let restored = round_trip(session, reply(assistant(vec![part])))?;
    let message = assistant_of(&restored)?;
    assert_eq!(types(&message.content)?, ["thinking"]);
    match &message.content[0] {
        AssistantContent::Thinking {
            thinking,
            signature,
        } => Ok((thinking.clone(), signature.clone())),
        other => Err(format!("expected thinking: {other:?}").into()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_thinking_content_with_signature_persisted_and_restored() -> TestResult {
    // @step Given a session with an assistant message containing thinking:
    let signature = "EqsDCkYIChgCKkDLMHNM46KZ1nevdcXu9...";
    // @step When the session is saved and restored
    // @step Then the restored message has content type "thinking"
    let restored =
        thinking_round_trip("Let me analyze this step by step...", Some(signature)).await?;
    // @step And the restored thinking text equals "Let me analyze this step by step..."
    assert_eq!(restored.0, "Let me analyze this step by step...");
    // @step And the restored thinking signature equals "EqsDCkYIChgCKkDLMHNM46KZ1nevdcXu9..."
    assert_eq!(restored.1.as_deref(), Some(signature));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_thinking_content_without_signature_persisted() -> TestResult {
    // @step Given a session with an assistant message containing thinking:
    // @step When the session is saved and restored
    // @step Then the restored message has content type "thinking"
    let restored = thinking_round_trip("Let me think about this problem...", None).await?;
    // @step And the restored thinking text equals "Let me think about this problem..."
    assert_eq!(restored.0, "Let me think about this problem...");
    // @step And the restored thinking signature is null
    assert_eq!(restored.1, None);
    Ok(())
}
