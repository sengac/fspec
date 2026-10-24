//! Feature: spec/features/compaction-operation-cancellation.feature

#[path = "bug214/support.rs"]
mod support;

use support::{Fixture, Mode, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn scenario_cancellation_stops_a_pending_pre_prompt_compaction() -> TestResult {
    // @step Given an unfinished task triggers pre-prompt compaction with a pending builder
    let mut fixture = Fixture::new(Mode::PrePrompt).await?;
    // @step When the user interrupts the session before the builder responds
    let result = fixture.run(true).await?;
    // @step Then compaction stops promptly with an explicit cancelled outcome
    // @step And the durable conversation and checkpoint remain unchanged
    fixture.assert_cancelled(result)?;
    // @step And no compaction success or continuation request is emitted
    fixture.assert_no_success_or_continuation().await
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_cancellation_stops_a_pending_hook_recovery_compaction() -> TestResult {
    // @step Given an unfinished task triggers hook recovery with a pending builder
    let mut fixture = Fixture::new(Mode::Hook).await?;
    // @step When the user interrupts the session before the builder responds
    let result = fixture.run(true).await?;
    // @step Then compaction stops promptly with an explicit cancelled outcome
    // @step And the durable conversation and checkpoint remain unchanged
    fixture.assert_cancelled(result)?;
    // @step And no compaction success or continuation request is emitted
    fixture.assert_no_success_or_continuation().await
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_a_late_candidate_cannot_override_cancellation() -> TestResult {
    // @step Given an automatic compaction builder is about to return a valid candidate
    let mut fixture = Fixture::new(Mode::Late).await?;
    // @step When the user interrupts before the candidate is admitted
    let result = fixture.run(false).await?;
    // @step Then the candidate is rejected as cancelled without durable promotion
    fixture.assert_cancelled(result)?;
    // @step And no compaction success or continuation request is emitted
    fixture.assert_no_success_or_continuation().await
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_cancellation_during_admission_vetoes_promotion() -> TestResult {
    // @step Given a valid compaction candidate reaches selected request admission
    let mut fixture = Fixture::new(Mode::Admission).await?;
    let original_tokens = fixture.session.token_tracker.input_tokens;
    // @step When the user interrupts while its replacement request is serialized
    let result = fixture.run(false).await?;
    // @step Then the candidate is rejected as cancelled without durable promotion
    fixture.assert_cancelled(result)?;
    // @step And the previous context tokens and provider checkpoint remain unchanged
    assert_eq!(fixture.session.token_tracker.input_tokens, original_tokens);
    fixture.assert_no_success_or_continuation().await
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_cancellation_after_promotion_preserves_the_authoritative_generation() -> TestResult
{
    // @step Given automatic compaction has durably promoted a valid generation
    let mut fixture = Fixture::new(Mode::Published).await?;
    // @step When the user interrupts during committed context publication
    let _result = fixture.run(false).await?;
    // @step Then the promoted generation remains authoritative and reconciled
    let manifest = codelet_core::persistence::load_session(fixture.manifest.id)?;
    let boundary = manifest.compaction_commit.ok_or("promoted generation")?;
    assert_eq!(boundary.identity.generation, 1);
    assert!(!boundary.publication_pending);
    fixture.session.ensure_compaction_inference_ready()?;
    // @step And the original task does not issue a continuation request
    assert!(fixture
        .server
        .received_requests()
        .await
        .ok_or("requests")?
        .is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_an_explicit_builder_retry_vetoes_cancellation_during_admission() -> TestResult {
    // @step Given fresh user input restarts a previously failed compaction builder
    let mut fixture = Fixture::new(Mode::Admission).await?;
    fixture.fail_builder().await?;
    // @step When the user interrupts during replacement request admission
    let result = fixture.retry().await?;
    // @step Then the retry returns cancellation without promoting a generation
    fixture.assert_cancelled(result.map(|_| ()).map_err(Into::into))?;
    fixture.assert_no_success_or_continuation().await?;
    // @step And another fresh input cannot retry the cancelled operation
    fixture
        .interrupted
        .store(false, std::sync::atomic::Ordering::Release);
    assert!(!fixture.retry().await??);
    Ok(())
}
