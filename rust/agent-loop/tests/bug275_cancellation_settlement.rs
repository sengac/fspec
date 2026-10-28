//! Feature: spec/features/cancellation-settlement.feature
#[path = "support/settlement_loop.rs"]
pub mod actual;
#[path = "support/cancellation_settlement.rs"]
pub mod support;
use codelet_core::persistence::messages::{durability, MessageStore};
use support::{chunks, index_fixture, read, Fixture, Gate, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn acknowledgement_precedes_durable_cancellation_settlement() -> TestResult {
    // @step Given a running tool whose cancellation persistence is paused
    let fixture = Fixture::new()?;
    let gate = Gate::new();
    let mut events = fixture.session.subscribe_to_stream();
    let observed_gate = gate.clone();
    let observer = async {
        observed_gate.held.notified().await;
        let mut acknowledged = false;
        while let Ok(event) = events.recv().await {
            let event = codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value(&event);
            if event
                .to_string()
                .contains("Interruption requested; settling")
            {
                acknowledged = true;
                break;
            }
            if event["type"] == "done" || event["type"] == "interrupted" {
                break;
            }
        }
        // @step Then visible acknowledgement arrives before persistence is released
        // @step And unrelated async work remains responsive
        tokio::task::yield_now().await;
        let before_release = chunks(&fixture.session);
        // @step And neither idle successful completion nor another inference turn overtakes settlement
        let still_owned = fixture.session.inner.try_lock().is_err();
        let running = fixture.session.get_status() == codelet_rpc_types::SessionStatus::Running;
        let no_terminal = !before_release
            .iter()
            .any(|event| event["type"] == "done" || event["type"] == "interrupted");
        let acknowledged_before_release = observed_gate.finish()?;
        Ok::<_, Box<dyn std::error::Error>>((
            acknowledged,
            still_owned,
            running,
            no_terminal,
            acknowledged_before_release,
        ))
    };
    // @step When the user interrupts the turn
    let (result, observed) = tokio::join!(support::run(&fixture, Some(gate), false), observer);
    result?;
    let (acknowledged, owned, running, no_terminal, released_after_ack) = observed?;
    assert!(
        acknowledged && released_after_ack,
        "visible acknowledgement waited for storage release"
    );
    assert!(
        owned && running && no_terminal,
        "settlement ownership or truthful state was lost"
    );
    assert_eq!(
        chunks(&fixture.session)
            .iter()
            .filter(|event| event["type"] == "done")
            .count(),
        1
    );
    actual::actual_loop(false, false).await?;
    actual::actual_loop(false, true).await
}

#[tokio::test]
#[serial_test::serial]
async fn settlement_retains_truthful_tool_outcomes_exactly_once() -> TestResult {
    // @step Given one completed tool and one unresolved running tool
    let fixture = Fixture::new()?;
    fixture.result("success", false);
    fixture.result("error", true);
    {
        let mut inner = fixture.session.inner.lock().await;
        inner
            .messages
            .push(rig::message::Message::tool_result_with_call_id(
                "success",
                Some("call-success".into()),
                "authentic-success",
            ));
        inner
            .messages
            .push(rig::message::Message::tool_result_with_call_id(
                "error",
                Some("call-error".into()),
                "authentic-error",
            ));
    }
    // @step When the turn is interrupted and safely settled
    support::run(&fixture, None, false).await?;
    let originals = read(fixture.session.id)?;
    // @step Then the completed outcome including genuine errors is preserved
    for (id, is_error) in [("success", false), ("error", true)] {
        let results: Vec<_> = originals
            .iter()
            .filter(|record| record["message"]["content"][0]["tool_use_id"] == id)
            .collect();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0]["message"]["content"][0]["content"],
            format!("authentic-{id}")
        );
        assert_eq!(results[0]["message"]["content"][0]["is_error"], is_error);
    }
    // @step And the unresolved tool has one truthful cancelled or unknown outcome
    let pending: Vec<_> = originals
        .iter()
        .filter(|record| record["message"]["content"][0]["tool_use_id"] == "running")
        .collect();
    assert_eq!(pending.len(), 1);
    assert!(pending[0]["message"]["content"][0]["content"]
        .as_str()
        .ok_or("missing outcome")?
        .contains("Side effects may have occurred"));
    let projection =
        codelet_core::persistence::transcript::JournalStore::configured(fixture.session.id)?
            .project(&Default::default())?;
    assert_eq!(
        projection
            .tools
            .iter()
            .find(|tool| tool.id == "running")
            .ok_or("running tool")?
            .state,
        codelet_core::persistence::transcript::ToolState::OutcomeUnknown
    );
    // @step And cold reopen preserves original identities and ordering without duplicates
    codelet_agent_loop::persist::retry_pending_persistence(&fixture.session.id)?;
    codelet_agent_loop::persist::retry_pending_persistence(&fixture.session.id)?;
    assert_eq!(read(fixture.session.id)?, originals);
    let invocation = originals
        .iter()
        .position(|record| record["message"]["content"][0]["id"] == "running")
        .ok_or("invocation")?;
    let outcome = originals
        .iter()
        .position(|record| record["message"]["content"][0]["tool_use_id"] == "running")
        .ok_or("outcome")?;
    assert!(invocation < outcome);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn failed_settlement_remains_recoverable() -> TestResult {
    // @step Given a cancellation whose required persistence fails
    let fixture = Fixture::new()?;
    let _result = support::run(&fixture, None, true).await;
    let failed = chunks(&fixture.session);
    // @step When the session is reopened and persistence is retried
    std::fs::remove_dir(fixture.directory.path().join("messages/messages.jsonl"))?;
    std::fs::rename(
        fixture.directory.path().join("messages/saved.jsonl"),
        fixture.directory.path().join("messages/messages.jsonl"),
    )?;
    codelet_agent_loop::persist::retry_pending_persistence(&fixture.session.id)?;
    let originals = read(fixture.session.id)?;
    codelet_agent_loop::persist::retry_pending_persistence(&fixture.session.id)?;
    // @step Then original input calls and outcomes remain recoverable exactly once
    assert_eq!(read(fixture.session.id)?, originals);
    assert_eq!(
        originals
            .iter()
            .filter(|record| record["message"]["content"][0]["text"] == "original input")
            .count(),
        1
    );
    assert_eq!(
        originals
            .iter()
            .filter(|record| record["message"]["content"][0]["id"] == "running")
            .count(),
        1
    );
    assert_eq!(
        originals
            .iter()
            .filter(|record| record["message"]["content"][0]["tool_use_id"] == "running")
            .count(),
        1
    );
    // @step And no false success or new inference bypasses the unsettled records
    assert!(failed.iter().any(|event| event["type"] == "error"));
    assert!(
        !failed.iter().any(|event| event["type"] == "done"),
        "failed settlement emitted successful completion"
    );
    actual::actual_loop(true, false).await
}

#[test]
#[serial_test::serial]
fn index_publication_uses_bounded_buffered_writes() -> TestResult {
    // @step Given a large synthetic message index
    let (directory, ids) = index_fixture(4096)?;
    // @step When a cancellation outcome publishes that index
    let (store, boundaries) = durability::observe(None, MessageStore::new);
    let mut store = store?;
    let (result, publications) = durability::observe(None, || {
        store.store(
            "user",
            "Tool execution cancelled by user interruption. Side effects may have occurred.",
        )
    });
    result?;
    // @step Then filesystem writes are buffered in bounded batches rather than per entry
    let writes = publications
        .iter()
        .filter(|boundary| **boundary == "index.batch")
        .count();
    assert!(
        (2..=4).contains(&writes),
        "4097 entries required {writes} filesystem writes"
    );
    // @step And compatible index bytes are flushed before synchronization and atomic promotion
    for observed in [&boundaries, &publications] {
        let position = |boundary| {
            observed
                .iter()
                .position(|value| *value == boundary)
                .expect("durability boundary missing")
        };
        assert!(position("index.flush") < position("index.sync"));
        assert!(position("index.sync") < position("index.rename"));
        assert!(position("index.rename") < position("index.directory"));
    }
    let bytes = std::fs::read(directory.path().join("messages/messages.idx"))?;
    assert_eq!(&bytes[..4], b"MIDX");
    assert_eq!(bytes.len(), 20 + (ids.len() + 1) * 28);
    let cold = MessageStore::new()?;
    for id in ids {
        assert_eq!(cold.get(id).ok_or("missing original")?.id, id);
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn publication_failures_retain_crash_safe_history() -> TestResult {
    // @step Given an index publication with injected write flush sync rename or directory failure
    for boundary in [
        "index.write",
        "index.batch",
        "index.flush",
        "index.sync",
        "index.rename",
        "index.directory",
    ] {
        let (directory, ids) = index_fixture(64)?;
        let original = std::fs::read(directory.path().join("messages/messages.jsonl"))?;
        // @step When publication fails and storage is cold reopened
        let (result, _) = durability::observe(Some(boundary), MessageStore::new);
        assert!(result.is_err(), "failure boundary {boundary} was bypassed");
        let cold = MessageStore::new()?;
        // @step Then committed originals remain readable with intact identities and hashes
        for id in ids {
            assert_eq!(cold.get(id).ok_or("missing committed original")?.id, id);
        }
        assert_eq!(
            std::fs::read(directory.path().join("messages/messages.jsonl"))?,
            original
        );
        // @step And incomplete publication never masquerades as durable successful settlement
        assert_eq!(
            std::fs::read_dir(directory.path().join("messages"))?
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
                .count(),
            0
        );
    }
    support::cancelled_promotion()
}
