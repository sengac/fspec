//! Feature: spec/features/automatic-compaction-completion-metrics.feature

#[path = "support/bug210_completion_metrics.rs"]
mod support;

#[path = "support/completion_observer.rs"]
mod completion_observer;

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_core::persistence::compaction_boundary::CompactionIdentity;
use codelet_rpc_types::{CommittedContextSnapshot, RequestContextSnapshot, StreamChunk};
use support::{Fixture, TestResult, COMPACTED, ORIGINAL};

#[tokio::test]
#[serial_test::serial]
async fn scenario_completion_reports_the_percentage_removed_rather_than_retained() -> TestResult {
    // @step Given automatic compaction begins with 166750 input tokens
    let fixture = Fixture::new().await?;
    assert_eq!(
        fixture
            .session
            .inner
            .lock()
            .await
            .token_tracker
            .input_tokens,
        ORIGINAL
    );
    // @step And its durable committed context contains 90336 input tokens
    let output = BackgroundOutput::with_provider(fixture.session.clone(), "openai".into(), None);
    // @step When the automatic builder publishes completion through the background output adapter
    fixture.run(&output).await?;
    assert_eq!(fixture.boundary()?.input_tokens, COMPACTED);
    let requests = fixture
        .server
        .received_requests()
        .await
        .ok_or("builder request")?;
    assert_eq!(requests.len(), 1);
    assert!(String::from_utf8_lossy(&requests[0].body)
        .contains(&format!("Target session: {}", fixture.session.id)));
    // @step Then the TUI-compatible completion reports 166750 original tokens and 90336 compacted tokens
    let result = fixture.completion()?;
    assert_eq!(
        (result.original_tokens, result.compacted_tokens),
        (166_750, 90_336)
    );
    // @step And the reduction rounds to 45.8 percent rather than 54.2 percent
    assert_eq!(format!("{:.1}", result.compression_ratio), "45.8");
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_self_compaction_completion_waits_for_durable_reconciliation() -> TestResult {
    // @step Given an active session submits a valid summary of its own persisted history
    let fixture = Fixture::new().await?;
    let mut inner = fixture.session.inner.lock().await;
    let queue =
        codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler(
            fixture.session.id,
            &inner,
            std::env::temp_dir(),
            fixture.session.clone(),
            None,
        );
    codelet_tools::inject_summary::execute_inject_summary(fixture.session.id, support::DAG.into())?;
    let observer = completion_observer::CompletionObserver::new(&fixture.session);
    // @step When its submitted summary is applied and published
    let result = queue.apply(&mut inner, &fixture.session);
    observer.stop();
    codelet_tools::set_inject_summary_handler(fixture.session.id, None);
    codelet_tools::set_generate_compaction_handler(fixture.session.id, None);
    assert!(result?);
    // @step Then every completion observer sees a durably reconciled generation
    assert_eq!(
        *observer
            .pending_at_completion
            .lock()
            .expect("completion observations"),
        vec![false]
    );
    // @step And exactly one successful completion describes that generation
    assert_eq!(
        fixture
            .chunks()
            .iter()
            .filter(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
            .count(),
        1
    );
    assert_eq!(fixture.completion()?.turns_summarized, 196);
    assert!(!fixture.boundary()?.publication_pending);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_completion_coverage_matches_the_durable_generation() -> TestResult {
    // @step Given an automatic builder commits a DAG covering 196 persisted messages
    let fixture = Fixture::new().await?;
    let output = BackgroundOutput::with_provider(fixture.session.clone(), "openai".into(), None);
    // @step And the committed generation retains no verified suffix messages
    // @step When completion reaches the originating session through the background output adapter
    fixture.run(&output).await?;
    let boundary = fixture.boundary()?;
    assert_eq!(boundary.covered_prefix.len(), 196);
    assert!(boundary.verified_suffix.is_empty());
    // @step Then completion reports 196 summarized messages and zero retained messages
    let result = fixture.completion()?;
    assert_eq!((result.turns_summarized, result.turns_kept), (196, 0));
    // @step And the coverage counts equal the committed covered prefix and verified suffix counts
    assert_eq!(
        result.turns_summarized as usize,
        boundary.covered_prefix.len()
    );
    assert_eq!(result.turns_kept as usize, boundary.verified_suffix.len());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_successful_publication_produces_only_one_completion_notice() -> TestResult {
    // @step Given an automatic builder has a valid DAG candidate for a durable session
    let fixture = Fixture::new().await?;
    let output = BackgroundOutput::with_provider(fixture.session.clone(), "openai".into(), None);
    // @step When the candidate commits and its context is published successfully
    fixture.run(&output).await?;
    let chunks = fixture.chunks();
    // @step Then the background adapter publishes the committed context before completion
    let published = chunks
        .iter()
        .position(|chunk| {
            matches!(chunk,
                StreamChunk::CommittedCompaction { snapshot } if snapshot.reconciled
            )
        })
        .ok_or("background output must publish the committed generation")?;
    let completed = chunks
        .iter()
        .position(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
        .ok_or("completion")?;
    assert!(published < completed);
    // @step And exactly one user-visible completion is published for that generation
    let notice_count = |chunks: &[StreamChunk]| {
        chunks
            .iter()
            .filter(|chunk| {
                matches!(
                    chunk,
                    StreamChunk::CompactionComplete { .. }
                        | StreamChunk::CommittedCompaction {
                            snapshot: CommittedContextSnapshot {
                                reconciled: false,
                                ..
                            }
                        }
                )
            })
            .count()
    };
    assert_eq!(notice_count(&chunks), 1);
    // @step And reconciling the same generation does not publish another completion notice
    let identity = fixture.boundary()?.identity;
    fixture
        .session
        .reconcile_durable_compaction(&identity)
        .await?;
    fixture
        .session
        .reconcile_durable_compaction(&identity)
        .await?;
    assert_eq!(notice_count(&fixture.chunks()), 1);
    Ok(())
}

struct RejectPublication(BackgroundOutput);

impl StreamOutput for RejectPublication {
    fn emit(&self, event: StreamEvent) {
        self.0.emit(event);
    }

    fn publish_committed_context(
        &self,
        _identity: &CompactionIdentity,
        _snapshot: CommittedContextSnapshot,
        _context: RequestContextSnapshot,
    ) -> Result<(), String> {
        Err("injected publication boundary failure".into())
    }
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_failed_publication_produces_no_successful_completion() -> TestResult {
    // @step Given an automatic builder has committed a valid DAG candidate
    let fixture = Fixture::new().await?;
    let output = RejectPublication(BackgroundOutput::with_provider(
        fixture.session.clone(),
        "openai".into(),
        None,
    ));
    // @step When publication of its committed context fails
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("publication must fail")?;
    assert!(error
        .to_string()
        .contains("injected publication boundary failure"));
    // @step Then no successful completion is published
    assert!(!fixture
        .chunks()
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    // @step And the committed generation remains pending reconciliation
    let boundary = fixture.boundary()?;
    assert!(boundary.publication_pending);
    assert!(fixture
        .session
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    // @step And retrying the blocked operation does not fabricate a successful completion
    assert!(fixture.run(&output).await.is_err());
    assert_eq!(fixture.boundary()?.identity, boundary.identity);
    assert!(!fixture
        .chunks()
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    Ok(())
}
