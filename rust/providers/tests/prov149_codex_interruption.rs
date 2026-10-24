//! Feature: spec/features/codex-interrupted-continuation.feature

use codelet_providers::codex::history::{CheckpointError, CodexHistory};
use rig::completion::CompletionError;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn interrupted_tool_calls_receive_cancellation_outputs_before_new_input() -> TestResult {
    for saved in [true, false] {
        // @step Given a saved Codex response with parallel tool calls
        let history = CodexHistory::default();
        let initial = json!({"model":"test","input":[{"role":"user","content":"inspect"}]});
        let calls = [
            json!({"type":"function_call","id":"fc_1","call_id":"call_1","name":"inspect","arguments":"{}"}),
            json!({"type":"custom_tool_call","id":"fc_2","call_id":"call_2","name":"patch","input":"patch"}),
        ];
        if saved {
            history.commit(&initial, &calls)?;
        }
        // @step And only one tool result survived an interruption
        let result =
            json!({"type":"function_call_output","call_id":"call_1","output":"real result"});
        let new_user = json!({"type":"message","role":"user","content":"stop and review"});
        let mut next =
            json!({"model":"test","input":[initial["input"][0],calls[0],calls[1],result,new_user]});
        // @step When a new user message continues the session
        let previous = history.snapshot()?;
        let unchanged = next.clone();
        let error = history
            .prepare(&mut next)
            .err()
            .ok_or("missing tool outcome was unexpectedly accepted")?;
        // @step Then the missing outcome is rejected without changing checkpoint or input
        assert!(matches!(error, CompletionError::RequestError(ref source)
            if matches!(source.downcast_ref::<CheckpointError>(), Some(CheckpointError::MissingOutcome(call)) if call == "call_2")));
        assert_eq!(history.snapshot()?, previous);
        assert_eq!(next, unchanged);
        // @step When the caller supplies the genuine cancellation outcome and retries
        next["input"].as_array_mut().ok_or("input missing")?.insert(
            4,
            json!({"type":"custom_tool_call_output","call_id":"call_2","output":"cancelled by user"}),
        );
        history.prepare(&mut next)?;
        let input = next["input"].as_array().ok_or("missing input")?;
        // @step Then the completed tool result is preserved
        assert_eq!(input[3], result);
        // @step And each unfinished call receives a cancellation output before the new message
        assert_eq!(input[4]["type"], "custom_tool_call_output");
        assert_eq!(input[4]["call_id"], "call_2");
        assert!(input[4]["output"]
            .as_str()
            .ok_or("missing cancellation")?
            .contains("cancel"));
        assert_eq!(input[5], new_user);
        assert_eq!(input.len(), 6);
        assert_eq!(history.snapshot()?.is_some(), saved);
    }
    Ok(())
}

#[test]
fn reminder_bearing_user_messages_still_invalidate_replaced_conversations() -> TestResult {
    // @step Given a saved Codex conversation whose user message includes a system reminder
    let history = CodexHistory::default();
    let initial = json!({"model":"test","input":[{"role":"user","content":[
        {"type":"input_text","text":"<system-reminder>workflow</system-reminder>\ninspect old code"}
    ]}]});
    history.commit(
        &initial,
        &[json!({"type":"reasoning","summary":[],"encrypted_content":"old opaque state"})],
    )?;
    let mut replacement = json!({"model":"test","input":[{"type":"message","role":"user","content":[
        {"type":"input_text","text":"<system-reminder>compacted summary</system-reminder>"}
    ]}]});
    let original: Value = replacement.clone();
    let previous = history.snapshot()?.ok_or("saved checkpoint missing")?;
    // @step When local compaction commits an advancing boundary before replacing the conversation
    history.transition_generation(previous.generation + 1)?;
    history.prepare(&mut replacement)?;
    // @step Then the old typed continuation is discarded
    let current = history.snapshot()?.ok_or("committed boundary missing")?;
    assert_eq!(current.generation, previous.generation + 1);
    assert!(current.items.is_empty());
    // @step And the replacement message is sent without stale reasoning or tool calls
    assert_eq!(replacement, original);
    Ok(())
}
