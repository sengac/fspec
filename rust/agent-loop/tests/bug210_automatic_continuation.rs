//! Feature: spec/features/automatic-compaction-continuation.feature

#[path = "support/automatic_compaction.rs"]
mod support;

use codelet_rpc_types::StreamChunk;
use std::sync::atomic::Ordering;
use support::Harness;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_overflow_recovery_resumes_work_without_another_user_message() {
    // @step Given an agent with completed work encounters a context overflow
    let mut fixture = Harness::new(false, false, false).await;
    fixture.completed_tool.store(true, Ordering::SeqCst);
    // @step When automatic compaction commits a usable summary
    fixture.send();
    let chunks = fixture.until_done().await;
    // @step Then the agent continues the original task on the reduced history
    assert_eq!(
        fixture.infer.load(Ordering::SeqCst),
        3,
        "resume before Done without new user input"
    );
    let requests = fixture
        .server
        .received_requests()
        .await
        .expect("physical requests");
    let resumed = String::from_utf8_lossy(&requests.last().expect("continued request").body);
    assert!(
        resumed.contains("Completed evidence"),
        "continuation uses committed DAG"
    );
    assert!(
        !resumed.contains("committed response"),
        "oversized old history is not replayed"
    );
    // @step And completed work is not repeated and completion is not announced prematurely
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 1);
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("completed-command.txt"))
            .expect("real completed tool side effect"),
        "executed\n",
        "the completed command executes exactly once"
    );
    assert_eq!(
        chunks
            .iter()
            .filter(|chunk| matches!(chunk, StreamChunk::Done))
            .count(),
        1
    );
    assert!(!chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::Error { .. })));
    fixture.assert_originals_available();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_loop_detector_interruption_still_permits_its_corrective_retry() {
    // @step Given an agent is interrupted by the repetitive-output detector rather than the user
    let mut fixture = Harness::new(false, false, false).await;
    fixture.loop_abort.store(true, Ordering::SeqCst);
    fixture.send();
    // @step When the outer loop schedules its corrective retry
    fixture.until_done().await;
    let resumed_chunks = fixture.until_done().await;
    // @step Then inference resumes without another user message
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 0);
    // @step And the next request includes the corrective note and finishes the task
    let requests = fixture
        .server
        .received_requests()
        .await
        .expect("physical requests");
    let resumed = String::from_utf8_lossy(&requests.last().expect("corrective retry").body);
    assert!(resumed.contains("repetitive output"));
    assert!(resumed.contains("fresh approach"));
    assert!(resumed_chunks.iter().any(
        |chunk| matches!(chunk, StreamChunk::Text { text, .. } if text.contains("Original task finished"))
    ));
    assert!(!fixture.session.has_pending_loop_abort_note());
    assert!(!fixture.session.is_interrupted.load(Ordering::SeqCst));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_an_active_agent_resumes_after_submitting_its_own_summary() {
    // @step Given an active agent requests compaction of its own unfinished task
    let mut fixture = Harness::new(false, false, false).await;
    fixture.self_compaction.store(true, Ordering::SeqCst);
    // @step When its submitted summary is durably applied by the outer loop
    fixture.send();
    let chunks = fixture.until_done().await;
    // @step Then the agent resumes on that summary without another user message
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 2);
    assert!(chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    // @step And completion is announced only after the remaining task finishes
    assert_eq!(
        chunks
            .iter()
            .filter(|chunk| matches!(chunk, StreamChunk::Done))
            .count(),
        1
    );
    let requests = fixture
        .server
        .received_requests()
        .await
        .expect("continued requests");
    assert!(
        String::from_utf8_lossy(&requests.last().expect("continuation").body)
            .contains("Completed evidence")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cancellation_after_summary_generation_prevents_automatic_continuation() {
    // @step Given an automatic summary completes as the user cancels the session
    let mut fixture = Harness::new(true, false, false).await;
    fixture.cancel_on_summary.store(true, Ordering::SeqCst);
    // @step When the outer loop considers continuing the interrupted task
    fixture.send();
    fixture.until_done().await;
    // @step Then no continuation inference request starts
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
    assert!(fixture.session.is_interrupted.load(Ordering::SeqCst));
    fixture.assert_originals_available();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_overflow_stops_at_a_finite_recovery_budget() {
    // @step Given an agent repeatedly exceeds its context after successful compaction
    let mut fixture = Harness::new(true, false, false).await;
    // @step When the automatic recovery budget is exhausted
    fixture.send();
    let chunks = fixture.until_done().await;
    // @step Then the user receives an explicit terminal recovery error
    assert!(chunks.iter().any(|chunk| matches!(chunk, StreamChunk::Error { error, .. } if error.contains("recovery budget"))), "explicit exhausted budget: {chunks:?}");
    // @step And no further compaction or inference request starts
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 3);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 4);
    let requests = fixture
        .server
        .received_requests()
        .await
        .expect("bounded requests")
        .len();
    tokio::task::yield_now().await;
    assert_eq!(
        fixture
            .server
            .received_requests()
            .await
            .expect("requests after completion")
            .len(),
        requests
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cancelling_compaction_does_not_restart_the_agent() {
    // @step Given automatic compaction is waiting for its summary
    let mut fixture = Harness::new(false, false, true).await;
    fixture.send();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while fixture.builders.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("builder entered");
    // @step When the user cancels the session
    fixture.session.interrupt();
    let chunks = fixture.until_done().await;
    // @step Then the session stops without another inference request
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
    assert!(!chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    // @step And its previously committed conversation remains available
    fixture.assert_originals_available();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_failed_compaction_does_not_restart_the_agent() {
    // @step Given automatic compaction cannot produce a usable summary
    let mut fixture = Harness::new(false, true, false).await;
    // @step When the failed recovery finishes
    fixture.send();
    let chunks = fixture.until_done().await;
    // @step Then the user receives an actionable error without another inference request
    assert!(chunks.iter().any(
        |chunk| matches!(chunk, StreamChunk::Error { error, .. } if error.contains("retry later"))
    ));
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
    assert!(!chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    // @step And its previously committed conversation remains available
    fixture.assert_originals_available();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_new_user_task_receives_a_fresh_recovery_budget() {
    // @step Given an earlier task exhausted its automatic recovery budget
    let mut fixture = Harness::new(true, false, false).await;
    fixture.send();
    let first = fixture.until_done().await;
    assert!(first.iter().any(|chunk| matches!(chunk, StreamChunk::Error { error, .. } if error.contains("recovery budget"))));
    let builders = fixture.builders.load(Ordering::SeqCst);
    fixture.close_injections();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // @step When the user submits a new task
    fixture.send();
    let second = fixture.until_done().await;
    // @step Then the new task can recover from a context overflow
    assert_eq!(
        fixture.builders.load(Ordering::SeqCst),
        builders + 3,
        "new task receives the full recovery budget"
    );
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 8);
    assert!(second
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
}
