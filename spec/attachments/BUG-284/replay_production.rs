use codelet_providers::codex::history::{CodexCheckpoint, CodexHistory};
use serde_json::Value;
use std::error::Error;
use std::path::Path;

fn prepare(checkpoint: &Value, request: &Value) -> Result<(), Box<dyn Error>> {
    let history = CodexHistory::default();
    let saved: CodexCheckpoint = serde_json::from_value(checkpoint.clone())?;
    history.restore(Some(saved))?;
    let mut candidate = request.clone();
    history.prepare(&mut candidate)?;
    Ok(())
}

fn earlier(root: &Path, session: &str, expected_changed: usize, expected_omitted: usize) -> Result<(), Box<dyn Error>> {
    let folder = root.join(session);
    let checkpoint: Value = serde_json::from_slice(&std::fs::read(folder.join("checkpoint.json"))?)?;
    let original: Value = serde_json::from_slice(&std::fs::read(folder.join("reconstructed-input.json"))?)?;
    let baseline_error = prepare(&checkpoint, &original).err().ok_or("earlier baseline unexpectedly accepted")?.to_string();
    if !baseline_error.contains("assistant, tool result or DAG ancestry") { return Err(baseline_error.into()); }
    let saved = checkpoint["items"].as_array().ok_or("items missing")?;
    let mut corrected = original.clone();
    let input = corrected["input"].as_array_mut().ok_or("input missing")?;
    let mut changed = 0;
    for item in input {
        if item["type"] == "function_call_output" {
            if let Some(canonical) = saved.iter().find(|v| v["type"] == "function_call_output" && v["call_id"] == item["call_id"]) {
                if item["output"] != canonical["output"] {
                    changed += 1;
                    item["output"] = canonical["output"].clone();
                }
            }
        }
    }
    if changed != expected_changed { return Err("unexpected changed outcome count".into()); }
    let after_outputs = prepare(&checkpoint, &corrected);
    if after_outputs.is_ok() != (expected_omitted == 0) { return Err("unexpected outcome-only counterfactual".into()); }
    let input = corrected["input"].as_array_mut().ok_or("input missing")?;
    let before = input.len();
    input.retain(|item| item["role"] != "user" || saved.iter().any(|s| s["role"] == item["role"] && s["content"] == item["content"]));
    let omitted = before - input.len();
    if omitted != expected_omitted { return Err(format!("unexpected omitted count {omitted}").into()); }
    prepare(&checkpoint, &corrected)?;
    println!("{session}: real persisted history REJECTED; replacing {changed} altered outcomes in cloned input {}{}", if expected_omitted == 0 { "ACCEPTED" } else { "still REJECTED" }, if omitted > 0 { format!("; also omitting {omitted} unmatched user originals in clone ACCEPTED") } else { String::new() });
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new("/tmp/compaction-investigation/private");
    for session in [
        "4d8eedb6-fe1d-43cb-9aa9-70252fb8cd61",
        "7b41d57c-d7aa-47f3-b2c7-b689012703c4",
    ] {
        let folder = root.join(session);
        let checkpoint: Value = serde_json::from_slice(&std::fs::read(folder.join("checkpoint.json"))?)?;
        let original: Value = serde_json::from_slice(&std::fs::read(folder.join("reconstructed-input.json"))?)?;
        let failed = prepare(&checkpoint, &original);
        let error = failed.as_ref().err().map(ToString::to_string).unwrap_or_default();
        if !error.contains("assistant, tool result or DAG ancestry") {
            return Err(format!("unexpected baseline outcome for {session}: {error}").into());
        }
        // Counterfactual only: remove exactly the unmatched supervisor blocks
        // in a cloned local request. Saved/live histories remain untouched.
        let mut counterfactual = original.clone();
        let known = checkpoint["items"].as_array().ok_or("checkpoint items missing")?;
        let input = counterfactual["input"].as_array_mut().ok_or("input missing")?;
        let before = input.len();
        input.retain(|item| {
            let text = item.pointer("/content/0/text").and_then(Value::as_str);
            !(item["role"] == "user"
                && text.is_some_and(|t| t.starts_with("[SUPERVISOR:"))
                && !known.iter().any(|saved| saved["role"] == item["role"] && saved["content"] == item["content"]))
        });
        let removed = before - input.len();
        prepare(&checkpoint, &counterfactual)?;
        let input = counterfactual["input"].as_array_mut().ok_or("input missing")?;
        let outcome = input.iter_mut().find(|item| item["type"] == "function_call_output")
            .ok_or("missing real outcome")?;
        outcome["output"] = Value::String("diagnostic tamper of isolated copy".into());
        let tampered = prepare(&checkpoint, &counterfactual);
        if tampered.is_ok() { return Err("strict tamper rejection unexpectedly passed".into()); }
        println!("{session}: real persisted history REJECTED ({error}); cloned input omitting {removed} unmatched supervisor messages ACCEPTED; changed tool outcome REJECTED");
    }
    let earlier_root = Path::new("/tmp/compaction-investigation/earlier-private");
    earlier(earlier_root, "7c55a508-ff74-4a68-895f-c01e9bdbb03c", 233, 9)?;
    earlier(earlier_root, "c09c6649-9724-42d6-8aa7-005a885b8d27", 165, 0)?;
    Ok(())
}
