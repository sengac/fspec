#![doc = "Feature: spec/features/outer-error-terminal-journaling.feature"]

#[path = "support/automatic_compaction.rs"]
pub mod automatic;
#[path = "support/bug258.rs"]
mod support;

use automatic::Harness;
use codelet_core::persistence::transcript::{JournalEvent, JournalStore};
use codelet_rpc_types::{SessionStatus, StreamChunk};
use std::sync::atomic::Ordering;
use support::{cold, failures, prepare, Environment, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn invalid_request_preparation_retains_one_cold_terminal_failure() -> TestResult {
    // @step Given a real background turn with invalid selected request limits
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, false, false).await;
    prepare(fixture.directory.path(), fixture.session.id)?;
    fixture
        .session
        .inner
        .lock()
        .await
        .provider_manager_mut()
        .set_model_direct("openai", "o3", Some(4096), Some(4096), None)?;
    // @step When request preparation fails before streaming begins
    fixture.send();
    let live = fixture.until_done().await;
    assert!(live.iter().any(|chunk| matches!(chunk, StreamChunk::Error { error } if error.contains("output reservation"))));
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.session.get_status(), SessionStatus::Idle);
    // @step Then one safe terminal failure survives repeated cold replay without model pollution
    cold(fixture.directory.path(), fixture.session.id, 1)?;
    assert_eq!(failures(fixture.session.id)?.len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn failed_outer_overflow_recovery_retains_the_original_request_outcome() -> TestResult {
    // @step Given a real background request whose overflow recovery builder fails
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, true, false).await;
    prepare(fixture.directory.path(), fixture.session.id)?;
    // @step When the outer owner abandons recovery and publishes completion
    fixture.send();
    let live = fixture.until_done().await;
    assert!(live
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::Error { .. })));
    assert!(fixture.builders.load(Ordering::SeqCst) > 0);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
    // @step Then one safe failure is durable on the original request and cold replay retains it once
    cold(fixture.directory.path(), fixture.session.id, 1)?;
    let records = JournalStore::configured(fixture.session.id)?.read()?;
    let original = records
        .first()
        .ok_or("missing original request")?
        .identity
        .request_id;
    let failures = failures(fixture.session.id)?;
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].identity.request_id, original);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn exhausted_outer_recovery_retains_the_final_abandoned_outcome() -> TestResult {
    // @step Given a real background request repeatedly overflows after successful compaction
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(true, false, false).await;
    prepare(fixture.directory.path(), fixture.session.id)?;
    // @step When its outer recovery budget is exhausted
    fixture.send();
    let live = fixture.until_done().await;
    assert!(live.iter().any(
        |chunk| matches!(chunk, StreamChunk::Error { error } if error.contains("recovery budget"))
    ));
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 3);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 4);
    // @step Then the final abandoned request has one cold terminal failure and no further inference starts
    cold(fixture.directory.path(), fixture.session.id, 1)?;
    let records = JournalStore::configured(fixture.session.id)?.read()?;
    let request = records
        .iter()
        .rev()
        .find(|record| matches!(record.event, JournalEvent::AttemptStarted))
        .ok_or("missing final request")?
        .identity
        .request_id;
    let failures = failures(fixture.session.id)?;
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].identity.request_id, request);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 4);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn inner_journaled_errors_are_not_duplicated_by_the_fallback() -> TestResult {
    // @step Given a real background stream that journals its own provider error
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, false, false).await;
    prepare(fixture.directory.path(), fixture.session.id)?;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({"error":{"code":"bad_request","message":"deterministic invalid provider request"}})))
        .with_priority(1).mount(&fixture.server).await;
    // @step When the outer error fallback also terminates that request
    fixture.send();
    let live = fixture.until_done().await;
    assert!(live
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::Error { .. })));
    // @step Then durable and cold replay retain exactly one terminal failure
    cold(fixture.directory.path(), fixture.session.id, 1)?;
    assert_eq!(failures(fixture.session.id)?.len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn successful_automatic_recovery_adds_no_spurious_failure() -> TestResult {
    // @step Given a real background request with recoverable overflow
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, false, false).await;
    prepare(fixture.directory.path(), fixture.session.id)?;
    // @step When automatic recovery succeeds and the original task finishes
    fixture.send();
    let live = fixture.until_done().await;
    assert!(live.iter().any(|chunk| matches!(chunk, StreamChunk::Text { text, .. } if text.contains("Original task finished"))));
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 2);
    // @step Then durable and cold replay contain no provider failure and completion remains truthful
    cold(fixture.directory.path(), fixture.session.id, 0)?;
    assert!(failures(fixture.session.id)?.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn applied_compaction_budget_exhaustion_retains_failure_instead_of_premature_completion(
) -> TestResult {
    // @step Given a real background task that repeatedly succeeds while applying compaction
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, false, false).await;
    prepare(fixture.directory.path(), fixture.session.id)?;
    support::repeat_applied_compaction(&fixture).await;
    // @step When the automatic continuation budget is exhausted after applied compaction
    fixture.send();
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut exhausted = false;
        loop {
            let chunk = fixture.chunks.recv().await?;
            exhausted |=
                matches!(&chunk, StreamChunk::Error { error } if error.contains("recovery budget"));
            if exhausted && matches!(chunk, StreamChunk::Done) {
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
        }
    })
    .await??;
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 4);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 4);
    assert_eq!(fixture.session.get_status(), SessionStatus::Idle);
    // @step Then the final request retains one cold terminal failure instead of success and no further inference starts
    let records = JournalStore::configured(fixture.session.id)?.read()?;
    let request = records
        .last()
        .ok_or("missing final request")?
        .identity
        .request_id;
    let terminals: Vec<_> = records
        .iter()
        .filter(|record| {
            record.identity.request_id == request
                && matches!(record.event, JournalEvent::Terminal { .. })
        })
        .collect();
    assert_eq!(terminals.len(), 1);
    assert!(
        matches!(
            terminals[0].event,
            JournalEvent::Terminal {
                outcome: codelet_core::persistence::transcript::TerminalOutcome::ProviderError
            }
        ),
        "final request was prematurely finalized: {:?}",
        terminals[0].event
    );
    cold(fixture.directory.path(), fixture.session.id, 1)?;
    assert_eq!(
        failures(fixture.session.id)?[0].identity.request_id,
        request
    );
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 4);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn cold_outer_failure_child() -> TestResult {
    support::cold_child().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn failed_explicit_builder_retry_preserves_actionable_failure_without_another_terminal(
) -> TestResult {
    // @step Given a real background request whose automatic compaction builder failed
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, true, false).await;
    prepare(fixture.directory.path(), fixture.session.id)?;
    fixture.send();
    fixture.until_done().await;
    // @step When fresh user input retries the still failing builder
    fixture.send();
    let live = fixture.until_done().await;
    // @step Then its actionable live error and one cold failure per request survive without another terminal or inference
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.session.get_status(), SessionStatus::Idle);
    assert_eq!(
        live.iter()
            .filter(|chunk| matches!(chunk,
                StreamChunk::Error { error } if error.contains("retry later")
            ))
            .count(),
        1,
        "missing or duplicate actionable outer error: {live:?}"
    );
    let retained = failures(fixture.session.id)?;
    assert_eq!(retained.len(), 2);
    assert_ne!(
        retained[0].identity.request_id,
        retained[1].identity.request_id
    );
    assert!(matches!(
        retained[1].event,
        JournalEvent::Terminal {
            outcome: codelet_core::persistence::transcript::TerminalOutcome::CompactionFailed
        }
    ));
    cold(fixture.directory.path(), fixture.session.id, 2)?;
    Ok(())
}
