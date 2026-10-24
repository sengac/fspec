//! Feature: spec/features/automatic-compaction-lifecycle.feature

#[path = "support/automatic_compaction.rs"]
mod support;

#[path = "support/bug210_completion_metrics.rs"]
mod metrics;

use codelet_rpc_types::{SessionState, SessionStatus, StreamChunk};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_automatic_completion_keeps_the_unfinished_task_active() {
    // @step Given an unfinished agent task requires automatic compaction
    let mut fixture = support::Harness::new(false, false, false).await;
    // @step When the committed summary is followed by resumed inference
    fixture.send();
    let chunks = fixture.until_done().await;
    let completion = chunks
        .iter()
        .position(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
        .expect("automatic compaction completes");
    let resumed = chunks
        .iter()
        .position(|chunk| matches!(chunk, StreamChunk::Text { text, .. } if text.contains("Original task finished")))
        .expect("remaining task finishes");
    assert!(completion < resumed);
    // @step Then no idle state is announced before the remaining task finishes
    assert!(!chunks[..resumed].iter().any(|chunk| matches!(
        chunk,
        StreamChunk::SessionStateChange {
            state: SessionState::Idle
        }
    )));
    // @step And the resumed task announces its running state
    assert!(chunks[completion..resumed].iter().any(|chunk| matches!(
        chunk,
        StreamChunk::SessionStateChange {
            state: SessionState::Running
        }
    )));
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_applying_a_manual_summary_leaves_an_idle_session_idle() -> metrics::TestResult {
    // @step Given an idle session has a manually submitted valid summary
    let fixture = metrics::Fixture::new().await?;
    assert_eq!(fixture.session.get_status(), SessionStatus::Idle);
    let mut inner = fixture.session.inner.lock().await;
    let queue =
        codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler(
            fixture.session.id,
            &inner,
            std::env::temp_dir(),
            fixture.session.clone(),
            None,
        );
    codelet_tools::inject_summary::execute_inject_summary(fixture.session.id, metrics::DAG.into())?;
    // @step When that summary is durably applied
    let applied = queue.apply(&mut inner, &fixture.session);
    codelet_tools::set_inject_summary_handler(fixture.session.id, None);
    codelet_tools::set_generate_compaction_handler(fixture.session.id, None);
    assert!(applied?);
    assert!(!fixture.boundary()?.publication_pending);
    // @step Then the session remains idle without starting inference
    assert_eq!(fixture.session.get_status(), SessionStatus::Idle);
    assert!(fixture
        .server
        .received_requests()
        .await
        .ok_or("recorded inference requests")?
        .is_empty());
    assert!(!fixture.chunks().iter().any(|chunk| matches!(
        chunk,
        StreamChunk::SessionStateChange {
            state: SessionState::Running
        }
    )));
    Ok(())
}
