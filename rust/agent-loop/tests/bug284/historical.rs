use super::fixture::{Harness, TestResult};
use codelet_providers::codex::history::{CodexCheckpoint, CodexHistory};
use serde_json::{json, Value};

const HISTORIES: [&str; 4] = [
    include_str!("../../../../spec/attachments/BUG-284/4d8eedb6-projected-fixture.json"),
    include_str!("../../../../spec/attachments/BUG-284/7b41d57c-projected-fixture.json"),
    include_str!("../../../../spec/attachments/BUG-284/7c55a508-projected-fixture.json"),
    include_str!("../../../../spec/attachments/BUG-284/c09c6649-projected-fixture.json"),
];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_untouched_accepted_history_remains_eligible_for_selected_compaction() -> TestResult
{
    // @step Given a genuine Codex conversation with a completed tool effect
    let mut harness = Harness::new()?;
    harness.seed().await?;
    // @step When no canceled submission has been added
    super::retention::equal_replay(&harness).await?;
    // @step Then selected compaction publishes a valid generation without repeating the effect
    harness.compact().await?;
    let manifest = codelet_core::persistence::load_session(harness.session.id)?;
    let boundary = manifest
        .compaction_commit
        .ok_or("missing committed generation")?;
    assert_eq!(boundary.identity.generation, 1);
    assert!(!boundary.publication_pending);
    assert_eq!(
        std::fs::read_to_string(harness.directory.path().join("effect"))?,
        "executed once"
    );
    Ok(())
}

#[test]
fn scenario_legacy_divergent_histories_and_altered_ancestry_remain_fail_closed() -> TestResult {
    // @step Given four sanitized derivatives of preserved failed Codex histories
    for encoded in HISTORIES {
        let evidence: Value = serde_json::from_str(encoded)?;
        let preserved = serde_json::to_vec(&evidence)?;
        let checkpoint: CodexCheckpoint = serde_json::from_value(evidence["checkpoint"].clone())?;
        let history = CodexHistory::default();
        history.restore(Some(checkpoint.clone()))?;
        let mut divergent = evidence["request"].clone();
        // @step When their divergent histories or altered assistant tool or DAG evidence are checked
        let failure = history
            .prepare(&mut divergent)
            .err()
            .ok_or("legacy divergence accepted")?;
        assert!(
            failure
                .to_string()
                .contains("assistant, tool result or DAG ancestry"),
            "{failure}"
        );
        let mut control = json!({"model":checkpoint.model,"input":checkpoint.items});
        let items = control["input"].as_array_mut().ok_or("control input")?;
        let pending: Vec<_> = items
            .iter()
            .filter(|item| item["type"] == "function_call")
            .filter(|call| {
                !items.iter().any(|item| {
                    item["type"] == "function_call_output" && item["call_id"] == call["call_id"]
                })
            })
            .map(|call| call["call_id"].clone())
            .collect();
        assert_eq!(pending.len(), 1);
        for call_id in pending {
            let genuine = evidence["request"]["input"]
                .as_array()
                .ok_or("evidence input")?
                .iter()
                .find(|item| item["type"] == "function_call_output" && item["call_id"] == call_id)
                .ok_or("missing actual pending outcome in evidence")?;
            items.push(genuine.clone());
        }
        history.prepare(&mut control.clone())?;
        for change in ["assistant", "invocation", "outcome", "missing", "ancestor"] {
            let mut altered = control.clone();
            let items = altered["input"].as_array_mut().ok_or("input")?;
            let index = items
                .iter()
                .position(|item| match change {
                    "assistant" => item["role"] == "assistant",
                    "invocation" => item["type"] == "function_call",
                    "outcome" | "missing" => item["type"] == "function_call_output",
                    _ => item["role"] == "user",
                })
                .ok_or("missing negative-control witness")?;
            match change {
                "assistant" | "ancestor" => {
                    items[index]["content"][0]["text"] = json!("tampered ancestry")
                }
                "invocation" => items[index]["arguments"] = json!("{\"tampered\":true}"),
                "outcome" => items[index]["output"] = json!("tampered outcome"),
                _ => {
                    items.remove(index);
                }
            }
            // @step Then strict provider lineage refuses the incompatible continuation
            assert!(
                history.prepare(&mut altered).is_err(),
                "{change} was accepted"
            );
        }
        // @step And no provider request runs and all historical evidence bytes remain unchanged
        assert_eq!(history.snapshot()?, Some(checkpoint));
        assert_eq!(serde_json::to_vec(&evidence)?, preserved);
        assert_eq!(serde_json::from_str::<Value>(encoded)?, evidence);
    }
    Ok(())
}
