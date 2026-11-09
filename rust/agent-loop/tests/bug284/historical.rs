use super::compaction;
use super::fixture::{Harness, TestResult};
use codelet_core::persistence::input_receipts::{self, ExecutionOutcome};
use codelet_providers::codex::history::{CodexCheckpoint, CodexHistory};
use codelet_sessions::background_session::IncomingMessage;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cancellation_and_cold_continuation_survive_successive_automatic_compactions(
) -> TestResult {
    // @step Given a real queued conversation has completed parallel tool effects once
    let mut harness = Harness::new()?;
    let session = harness.session.id;
    codelet_agent_loop::persist::persist_user_message(&session, "parallel task")?;
    let bodies = vec![
        compaction::parallel_calls(),
        compaction::answer("seed", "done"),
    ];
    let seeded = compaction::turn(&harness, "parallel task", bodies, false).await?;
    assert_eq!(seeded.requests.lock().map_err(|_| "requests")?.len(), 2);
    let root = harness.directory.path().to_path_buf();
    let effects = || -> Result<Vec<String>, std::io::Error> {
        ["a", "b"]
            .iter()
            .map(|name| std::fs::read_to_string(root.join(format!("parallel-{name}"))))
            .collect()
    };
    assert_eq!(effects()?.len(), 2);
    harness.start()?;
    // @step When canceled inputs and fresh admitted inputs span two automatic compactions and cold continuations
    for generation in 1..=2u64 {
        let mut events = harness.session.subscribe_to_stream();
        harness.session.interrupt();
        for index in 0..2 {
            let canceled = format!("canceled-g{generation}-{index}");
            let input = IncomingMessage::new("controller".into(), "operator".into(), canceled)?;
            harness.session.receive_incoming_message(input)?;
        }
        super::retention::done(&mut events, 2).await?;
        let mut events = harness.session.subscribe_to_stream();
        if generation == 1 {
            harness
                .session
                .send_input("admitted then failed".into(), None)?;
        } else {
            let held = super::retention::hold_admission(&harness, "admitted then interrupted");
            let (_, release) = held.await?;
            harness.session.interrupt();
            release.send(())?;
        }
        super::retention::done(&mut events, 1).await?;
        compaction::register_builder(session).await?;
        let fresh = format!("fresh admitted g{generation}");
        let id = compaction::admit(&harness, &fresh).await?;
        let reply = vec![compaction::answer(
            &format!("continued-{generation}"),
            "continued",
        )];
        let server = compaction::turn(&harness, &fresh, reply, true).await?;
        input_receipts::record_execution(session, id, ExecutionOutcome::Completed)?;
        // @step Then each committed generation retains exact original outcomes and valid DAG ancestry
        let manifest = codelet_core::persistence::load_session(session)?;
        let boundary = manifest.compaction_commit.as_ref().ok_or("generation")?;
        assert_eq!(boundary.identity.generation, generation);
        assert!(!boundary.publication_pending);
        let dag = &manifest.compaction.as_ref().ok_or("DAG")?.summary;
        assert!(dag.starts_with("<dag-node depth=\"D1\" turns=\"0-"));
        let full = codelet_core::persistence::get_session_messages_full(&manifest)?;
        let originals = serde_json::to_string(&full)?;
        for name in ["a", "b"] {
            let outcome = format!("parallel {name} executed once");
            let exact = full.iter().filter(|m| m.content.contains(&outcome));
            assert_eq!(exact.count(), 1);
        }
        // @step And no canceled instruction or completed tool effect executes again
        let wire = serde_json::to_string(&*server.requests.lock().map_err(|_| "requests")?)?;
        assert_eq!(server.requests.lock().map_err(|_| "requests")?.len(), 1);
        assert!(
            !wire.contains("canceled-g"),
            "canceled input reached the provider"
        );
        assert!(
            !originals.contains("canceled-g"),
            "canceled input joined the conversation"
        );
        assert_eq!(effects()?.len(), 2);
        compaction::cold(&harness).await?;
        super::retention::equal_replay(&harness).await?;
    }
    harness.stop().await;
    // @step And admitted cancellation and failure dispositions remain model visible after both compactions
    let full = codelet_core::persistence::get_session_messages_full(
        &codelet_core::persistence::load_session(session)?,
    )?;
    let receipts = input_receipts::list(session)?;
    for (text, failed) in [
        ("admitted then failed", true),
        ("admitted then interrupted", false),
    ] {
        let receipt = receipts
            .iter()
            .find(|r| r.text() == text)
            .ok_or("receipt")?;
        let outcome = receipt.execution.clone().ok_or("outcome")?;
        assert_eq!(
            matches!(outcome, ExecutionOutcome::FailedBeforeDispatch { .. }),
            failed
        );
        let disposition =
            codelet_agent_loop::input_admission::disposition_text(receipt.id, &outcome)?;
        assert_eq!(full.iter().filter(|m| m.content == disposition).count(), 1);
    }
    let canceled = receipts.iter().filter(|r| r.text().contains("canceled-g"));
    assert!(canceled.clone().count() == 4 && canceled.clone().all(|r| r.execution.is_none()));
    Ok(())
}
