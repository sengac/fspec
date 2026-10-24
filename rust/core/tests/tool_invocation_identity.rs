#![doc = "Feature: spec/features/codex-tool-invocation-identity-preservation.feature"]

use codelet_core::persistence::{AssistantContent, UserContent};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// @step Given assistant tool calls and tool outcomes have distinct item and invocation identities
/// @step When their persisted content blocks are deserialized and serialized
/// @step Then both blocks retain their original item and invocation identities
#[test]
fn serialized_tool_blocks_retain_explicit_invocation_identities() -> TestResult {
    let call = json!({
        "type":"tool_use", "id":"fc_original", "call_id":"call_original",
        "name":"inspect", "input":{"path":"original"}
    });
    let result = json!({
        "type":"tool_result", "tool_use_id":"fc_original", "call_id":"call_original",
        "content":"Tool execution cancelled", "is_error":true
    });
    let saved_call: AssistantContent = serde_json::from_value(call.clone())?;
    let saved_result: UserContent = serde_json::from_value(result.clone())?;
    assert_eq!(serde_json::to_value(saved_call)?, call);
    assert_eq!(serde_json::to_value(saved_result)?, result);
    Ok(())
}

/// @step Given legacy tool calls and tool outcomes have only item identities
/// @step When their persisted content blocks are deserialized and serialized
/// @step Then no invocation identity field is added
#[test]
fn legacy_tool_blocks_remain_compatible_without_invocation_identities() -> TestResult {
    let call = json!({"type":"tool_use", "id":"legacy", "name":"inspect", "input":{}});
    let result = json!({
        "type":"tool_result", "tool_use_id":"legacy", "content":"done", "is_error":false
    });
    let saved_call: AssistantContent = serde_json::from_value(call.clone())?;
    let saved_result: UserContent = serde_json::from_value(result.clone())?;
    let encoded_call = serde_json::to_value(saved_call)?;
    let encoded_result = serde_json::to_value(saved_result)?;
    assert_eq!(encoded_call, call);
    assert_eq!(encoded_result, result);
    for block in [encoded_call, encoded_result] {
        assert_eq!(block.get("call_id"), None::<&Value>);
    }
    Ok(())
}
