use super::fixture::{TestResult, SUFFIX};
use serde_json::Value;

pub fn assert_suffix(wire: &Value) -> TestResult {
    let input = wire["input"].as_array().ok_or("Codex input")?;
    let mut calls = Vec::new();
    let mut results = Vec::new();
    for (position, item) in input.iter().enumerate() {
        if item["call_id"] != "call_after_snapshot" {
            continue;
        }
        match item["type"].as_str() {
            Some("function_call") => {
                assert!(item["id"].as_str().is_some_and(|id| !id.is_empty()));
                assert_eq!(item["name"], "lookup");
                let arguments: Value = serde_json::from_str(
                    item["arguments"].as_str().ok_or("Codex call arguments")?,
                )?;
                assert_eq!(arguments, serde_json::json!({"path":"src/deep.rs"}));
                calls.push(position);
            }
            Some("function_call_output") => {
                assert_eq!(item["output"], SUFFIX);
                results.push(position);
            }
            _ => return Err("unexpected Codex suffix item type".into()),
        }
    }
    assert_eq!(
        calls.len(),
        1,
        "Codex duplicated or omitted the suffix call"
    );
    assert_eq!(
        results.len(),
        1,
        "Codex duplicated or omitted the suffix result"
    );
    assert!(calls[0] < results[0], "Codex reordered suffix call/result");
    Ok(())
}

/// Retain a legacy inline copy of a real, validated streaming checkpoint so the
/// acceptance gate still proves immutable old-record preservation. The shipping
/// writer now stores native state outside conversational originals.
pub fn retain_legacy_checkpoint(id: uuid::Uuid) -> TestResult {
    use codelet_core::persistence::*;
    let mut manifest = load_session(id)?;
    assert!(
        manifest.provider_state.is_some(),
        "actual streaming writer must publish native state"
    );
    let checkpoint = get_session_message_envelopes(id)?
        .into_iter()
        .map(|encoded| serde_json::from_str::<Value>(&encoded))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .rfind(|value| {
            value.get("codexContinuation").is_some()
                && (value["_providerState"].is_object() || value["_compactionSummary"] == true)
        })
        .ok_or("real persisted Codex checkpoint")?;
    let mut metadata = checkpoint
        .as_object()
        .ok_or("checkpoint object")?
        .iter()
        .filter(|(key, _)| {
            matches!(
                key.as_str(),
                "provider" | "codexContinuation" | "_codexReset"
            )
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    // Immediately after promotion the new generation is owned by the durable
    // boundary checkpoint; after a physical send its latest state is sidecar-owned.
    metadata.insert("provider".into(), serde_json::json!("codex"));
    metadata.insert(
        "message".into(),
        serde_json::json!({"role":"assistant","content":[]}),
    );
    append_message_with_metadata(&mut manifest, "assistant", "", metadata)?;
    Ok(())
}
