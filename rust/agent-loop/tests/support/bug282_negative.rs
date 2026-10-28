use crate::fixture::{history, Fixture, TestResult, MODEL};
use codelet_core::persistence::compaction_boundary::CompactionSource;
use codelet_core::persistence::compaction_commit::CompactionCommitError;
use codelet_core::persistence::*;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    Outcome,
    CallArguments,
    CallName,
    OutputIdentity,
    Assistant,
    MissingOutcome,
    EstablishedDag,
}

impl Change {
    pub const ALL: [Self; 7] = [
        Self::Outcome,
        Self::CallArguments,
        Self::CallName,
        Self::OutputIdentity,
        Self::Assistant,
        Self::MissingOutcome,
        Self::EstablishedDag,
    ];
}

pub async fn setup(
    change: Change,
) -> Result<(Fixture, CompactionSource), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let mut source = fixture
        .background
        .inner
        .lock()
        .await
        .capture_durable_compaction(fixture.background.id)?;
    fixture.run(1, true).await?;
    if change == Change::EstablishedDag {
        let committed = fixture.background.inner.lock().await.commit_selected_compaction(
            source, uuid::Uuid::new_v4(),
            "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">original task; authentic work retained</dag-node>",
            |boundary| {
                assert_eq!(load_session(boundary.identity.session_id)?.compaction_commit.as_ref(), Some(boundary));
                Ok(())
            },
        )?;
        assert_eq!(
            committed
                .compaction_commit
                .as_ref()
                .ok_or("boundary")?
                .identity
                .generation,
            1
        );
        fixture.cold().await?;
        source = fixture
            .background
            .inner
            .lock()
            .await
            .capture_durable_compaction(fixture.background.id)?;
        fixture.run(2, true).await?;
    }
    let control = fixture.run(10, false).await?;
    assert_eq!(
        control
            .requests
            .lock()
            .map_err(|_| "control requests")?
            .len(),
        1
    );
    fixture
        .background
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()?;
    Ok((fixture, source))
}

fn remove_outcome(fixture: &Fixture) -> TestResult {
    let manifest = load_session(fixture.background.id)?;
    let mut outcomes = Vec::new();
    for reference in &manifest.messages {
        let original = get_message(reference.message_id)?.ok_or("original")?;
        if original.metadata.get("message").is_some_and(|message| {
            message["content"]
                .as_array()
                .is_some_and(|content| content.iter().any(|part| part["type"] == "tool_result"))
        }) {
            outcomes.push(original);
        }
    }
    assert_eq!(outcomes.len(), 1);
    let removed = &outcomes[0];
    let session = fixture.background.inner.try_lock()?;
    let native = history(&session)?;
    let saved_checkpoint = native.snapshot()?;
    update_session_manifest(fixture.background.id, |current| {
        current
            .messages
            .retain(|reference| reference.message_id != removed.id);
        Ok(())
    })?;
    let changed = load_session(fixture.background.id)?;
    assert_eq!(changed.messages.len() + 1, manifest.messages.len());
    assert!(!changed
        .messages
        .iter()
        .any(|reference| reference.message_id == removed.id));
    assert_eq!(
        serde_json::to_value(get_message(removed.id)?.ok_or("removed record")?)?,
        serde_json::to_value(removed)?
    );
    assert_eq!(native.snapshot()?, saved_checkpoint);
    Ok(())
}

pub async fn mutate(fixture: &Fixture, change: Change) -> TestResult {
    if change == Change::MissingOutcome {
        return remove_outcome(fixture);
    }
    let session = fixture.background.inner.lock().await;
    let native = history(&session)?;
    let checkpoint = native.snapshot()?.ok_or("checkpoint")?;
    let mut request = json!({"model":MODEL,"input":serde_json::to_value(&checkpoint.items)?});
    let before = request["input"].as_array().ok_or("input")?.clone();
    let input = request["input"].as_array_mut().ok_or("input")?;
    let index = match change {
        Change::CallArguments | Change::CallName => {
            let index = input
                .iter()
                .position(|item| item["type"] == "function_call")
                .ok_or("actual invocation")?;
            let call = &mut input[index];
            assert_eq!(call["name"], "write_effect");
            let arguments: Value =
                serde_json::from_str(call["arguments"].as_str().ok_or("arguments")?)?;
            assert_eq!(arguments, json!({"work_unit_id":"NONE"}));
            if change == Change::CallArguments {
                call["arguments"] =
                    json!(json!({"work_unit_id":"NONE","changed":true}).to_string());
            } else {
                call["name"] = json!("different_effect");
            }
            assert_eq!(call["call_id"], before[index]["call_id"]);
            assert_eq!(call["id"], before[index]["id"]);
            index
        }
        Change::Outcome | Change::OutputIdentity => {
            let index = input
                .iter()
                .position(|item| item["type"] == "function_call_output")
                .ok_or("outcome")?;
            if change == Change::Outcome {
                input[index]["output"] = json!("changed authentic outcome");
            } else {
                input[index]["call_id"] = json!("changed-output-call");
            }
            index
        }
        Change::Assistant => {
            let index = input
                .iter()
                .position(|item| item["role"] == "assistant")
                .ok_or("assistant")?;
            input[index]["content"][0]["text"] = json!("changed assistant");
            index
        }
        Change::EstablishedDag => {
            let manifest = load_session(fixture.background.id)?;
            assert_eq!(
                manifest
                    .compaction_commit
                    .as_ref()
                    .ok_or("established boundary")?
                    .identity
                    .generation,
                1
            );
            assert_eq!(checkpoint.generation, 1);
            let dag = &manifest
                .compaction
                .as_ref()
                .ok_or("established DAG")?
                .summary;
            let (index, part) = input
                .iter()
                .enumerate()
                .find_map(|(index, item)| {
                    if item["role"] != "user" {
                        return None;
                    }
                    item["content"]
                        .as_array()?
                        .iter()
                        .enumerate()
                        .find(|(_, part)| {
                            part["text"].as_str().is_some_and(|text| text.contains(dag))
                        })
                        .map(|(part, _)| (index, part))
                })
                .ok_or("existing checkpoint DAG ancestor")?;
            let prior = input[index]["content"][part]["text"]
                .as_str()
                .ok_or("DAG text")?;
            let changed_dag = dag.replace("authentic work retained", "tampered DAG ancestry");
            assert_ne!(&changed_dag, dag);
            input[index]["content"][part]["text"] = json!(prior.replacen(dag, &changed_dag, 1));
            assert_eq!(
                load_session(fixture.background.id)?
                    .compaction
                    .as_ref()
                    .ok_or("DAG")?
                    .summary,
                *dag
            );
            index
        }
        Change::MissingOutcome => return Err("missing outcome must use original removal".into()),
    };
    assert_eq!(input.len(), before.len());
    assert_ne!(input[index], before[index]);
    for (position, item) in input.iter().enumerate() {
        if position != index {
            assert_eq!(item, &before[position]);
        }
    }
    native.commit(&request, &[])?;
    Ok(())
}

pub fn assert_records(originals: &Value) -> TestResult {
    for original in originals.as_array().ok_or("originals")? {
        let id = serde_json::from_value(original["id"].clone())?;
        assert_eq!(
            &serde_json::to_value(get_message(id)?.ok_or("original record lost")?)?,
            original
        );
    }
    Ok(())
}

pub fn assert_rejection(failure: &CompactionCommitError, change: Change) {
    let expected = match change {
        Change::MissingOutcome => "incompatible provider state: checkpoint identity or source anchor",
        Change::CallArguments | Change::CallName => "selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: tool call ancestry",
        _ => "selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: assistant, tool result or DAG ancestry",
    };
    assert!(
        matches!(failure, CompactionCommitError::Rejected(reason) if reason == expected),
        "{change:?}: {failure}"
    );
    eprintln!("verified {change:?}: {failure}");
}
