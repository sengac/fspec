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
