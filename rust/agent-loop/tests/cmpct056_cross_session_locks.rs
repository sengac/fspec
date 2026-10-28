//! Feature: spec/features/cross-session-compaction-ownership.feature

#[path = "cmpct056/support.rs"]
pub mod support;

use std::sync::Arc;
use std::time::Duration;

use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_tools::GenerateCompactionArgs;
use rig::tool::Tool;
use serial_test::serial;
use support::{assert_clean, assert_no_completion, snapshot, Fixture, Server};
use tokio::sync::Barrier;

const DEADLINE: Duration = Duration::from_secs(3);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn scenario_reciprocal_streaming_requests_terminate_without_acquiring_busy_targets() {
    // @step Given two streaming callers each owning their session lock
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let first_tool = fixture.tool(&fixture.caller);
    let second_tool = fixture.tool(&fixture.target);
    let before_first = snapshot(&fixture.caller).await;
    let before_second = snapshot(&fixture.target).await;
    let mut first_chunks = fixture.caller.subscribe_to_stream();
    let mut second_chunks = fixture.target.subscribe_to_stream();
    fixture.caller.set_status(SessionStatus::Running);
    fixture.target.set_status(SessionStatus::Running);
    let barrier = Arc::new(Barrier::new(2));
    let first = fixture.caller.clone();
    let second = fixture.target.clone();
    let first_barrier = barrier.clone();
    let target_id = second.id.to_string();
    let caller_id = first.id.to_string();
    // @step When a barrier releases reciprocal cross-session compaction requests
    let mut first_task = tokio::spawn(async move {
        let _owner = first.inner.lock().await;
        first_barrier.wait().await;
        let result = first_tool
            .call(GenerateCompactionArgs {
                session_id: Some(target_id),
            })
            .await;
        first_barrier.wait().await;
        result
    });
    let mut second_task = tokio::spawn(async move {
        let _owner = second.inner.lock().await;
        barrier.wait().await;
        let result = second_tool
            .call(GenerateCompactionArgs {
                session_id: Some(caller_id),
            })
            .await;
        barrier.wait().await;
        result
    });
    let results = tokio::time::timeout(DEADLINE, async {
        tokio::join!(&mut first_task, &mut second_task)
    })
    .await;
    if results.is_err() {
        first_task.abort();
        second_task.abort();
        let _ = tokio::join!(first_task, second_task);
    }
    // @step Then both requests return explicit busy failures within the harness deadline
    assert!(
        results.is_ok(),
        "reciprocal requests must not form a lock cycle"
    );
    let (first_result, second_result) = results.expect("bounded results");
    for result in [first_result, second_result] {
        assert!(result
            .expect("task completed")
            .expect_err("target busy")
            .to_string()
            .contains("busy"));
    }
    // @step And committed messages and activity remain unchanged without completion
    assert_eq!(snapshot(&fixture.caller).await, before_first);
    assert_eq!(snapshot(&fixture.target).await, before_second);
    assert_eq!(fixture.caller.get_status(), SessionStatus::Running);
    assert_eq!(fixture.target.get_status(), SessionStatus::Running);
    assert_no_completion(&mut first_chunks);
    assert_no_completion(&mut second_chunks);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn scenario_successful_cross_target_compaction_restores_idle_activity() {
    // @step Given an Idle target with committed context and a successful DAG builder
    let server = Server::new(200).await;
    let fixture = Fixture::new(&server.url).await;
    let before = snapshot(&fixture.target).await;
    let mut chunks = fixture.target.subscribe_to_stream();
    // @step When a different session compacts the target
    let task = fixture.start();
    tokio::time::timeout(DEADLINE, server.arrived.wait())
        .await
        .expect("builder request");
    server.release.wait().await;
    let result = tokio::time::timeout(DEADLINE, task)
        .await
        .expect("bounded compaction")
        .expect("task");
    // @step Then the new DAG is pinned and completion is published
    assert!(result.is_ok(), "successful builder: {result:?}");
    assert_ne!(snapshot(&fixture.target).await, before);
    let mut completed = false;
    while let Ok(chunk) = chunks.try_recv() {
        if matches!(chunk, StreamChunk::CompactionComplete { .. }) {
            completed = true;
        }
    }
    assert!(completed);
    // @step And a cold durable reload identifies the same admitted generation and covered originals
    let manifest = codelet_core::persistence::load_session(fixture.target.id)
        .expect("cold durable target manifest");
    let boundary = manifest
        .compaction_commit
        .expect("durable committed generation");
    assert!(!boundary.publication_pending);
    assert_eq!(boundary.covered_prefix.len(), 2);
    let snapshot = fixture
        .target
        .committed_context_snapshot()
        .expect("published admitted context");
    assert_eq!(snapshot.generation, boundary.identity.generation as i64);
    assert_eq!(
        u64::from(snapshot.tokens.input_tokens),
        boundary.input_tokens
    );
    // @step And the target is Idle with no compaction progress or busy flag
    assert_clean(&fixture.target);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn scenario_busy_capture_leaves_the_idle_target_unchanged() {
    // @step Given an Idle target whose session lock is owned
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let before = snapshot(&fixture.target).await;
    let mut chunks = fixture.target.subscribe_to_stream();
    let owner = fixture.target.inner.lock().await;
    let tool = fixture.tool(&fixture.caller);
    // @step When cross-target compaction requests capture before ownership is available
    let mut operation = Box::pin(tool.call(GenerateCompactionArgs {
        session_id: Some(fixture.target.id.to_string()),
    }));
    let outcome = futures::poll!(&mut operation);
    assert!(
        matches!(outcome, std::task::Poll::Ready(Err(ref error)) if error.to_string().contains("busy"))
    );
    drop(operation);
    drop(owner);
    // @step Then the committed context and Idle activity remain unchanged without completion
    assert_eq!(snapshot(&fixture.target).await, before);
    assert_no_completion(&mut chunks);
    // @step And no compaction progress or busy flag remains
    assert_clean(&fixture.target);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn scenario_cancellation_during_generation_releases_target_operation_ownership() {
    // @step Given an Idle target with a DAG builder waiting at a barrier
    let server = Server::new(200).await;
    let fixture = Fixture::new(&server.url).await;
    let before = snapshot(&fixture.target).await;
    let mut chunks = fixture.target.subscribe_to_stream();
    let task = fixture.start();
    tokio::time::timeout(DEADLINE, server.arrived.wait())
        .await
        .expect("captured target");
    // @step When cross-target compaction is cancelled after capture
    task.abort();
    let _ = task.await;
    // @step Then the committed context and Idle activity remain unchanged without completion
    assert_eq!(snapshot(&fixture.target).await, before);
    assert_no_completion(&mut chunks);
    // @step And no compaction progress or busy flag remains
    assert_clean(&fixture.target);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn scenario_busy_commit_rejects_the_candidate_without_awaiting_the_target() {
    // @step Given an Idle target captured by a DAG builder waiting at a barrier
    let server = Server::new(200).await;
    let fixture = Fixture::new(&server.url).await;
    let before = snapshot(&fixture.target).await;
    let mut chunks = fixture.target.subscribe_to_stream();
    let mut task = fixture.start();
    tokio::time::timeout(DEADLINE, server.arrived.wait())
        .await
        .expect("captured target");
    // @step When the target lock becomes busy before the builder returns a valid DAG
    let owner = fixture.target.inner.lock().await;
    server.release.wait().await;
    let result = tokio::time::timeout(DEADLINE, &mut task).await;
    if result.is_err() {
        task.abort();
        let _ = task.await;
    }
    // @step Then compaction returns an explicit busy failure within the harness deadline
    assert!(result.is_ok(), "commit must not wait for another owner");
    assert!(result
        .expect("bounded result")
        .expect("task")
        .expect_err("busy commit")
        .to_string()
        .contains("busy"));
    drop(owner);
    // @step And the committed context and Idle activity remain unchanged without completion
    assert_eq!(snapshot(&fixture.target).await, before);
    assert_clean(&fixture.target);
    assert_no_completion(&mut chunks);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn scenario_a_second_operation_cannot_take_ownership_from_an_active_builder() {
    // @step Given an Idle target captured by a DAG builder waiting at a barrier
    let server = Server::new(200).await;
    let fixture = Fixture::new(&server.url).await;
    let before = snapshot(&fixture.target).await;
    let mut chunks = fixture.target.subscribe_to_stream();
    let task = fixture.start();
    tokio::time::timeout(DEADLINE, server.arrived.wait())
        .await
        .expect("first builder captured target");
    let progress = fixture.target.get_compaction_progress();
    let tool = fixture.tool(&fixture.caller);
    // @step When a second cross-target compaction requests the same target
    let outcome = tokio::time::timeout(
        DEADLINE,
        tool.call(GenerateCompactionArgs {
            session_id: Some(fixture.target.id.to_string()),
        }),
    )
    .await;
    // @step Then the second operation returns an explicit busy failure within the harness deadline
    assert!(outcome
        .expect("second operation terminates without waiting")
        .expect_err("operation already owned")
        .to_string()
        .contains("busy"));
    // @step And the first operation retains its activity and progress until cancellation
    assert_eq!(fixture.target.get_status(), SessionStatus::Compacting);
    assert_eq!(
        fixture.target.get_compaction_progress().map(|state| (
            state.phase,
            state.current,
            state.total
        )),
        progress.map(|state| (state.phase, state.current, state.total))
    );
    task.abort();
    let _ = task.await;
    // @step And cancellation restores the unchanged Idle target without completion
    assert_eq!(snapshot(&fixture.target).await, before);
    assert_clean(&fixture.target);
    assert_no_completion(&mut chunks);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn scenario_failed_builder_cannot_publish_a_cross_target_fallback() {
    // @step Given an Idle target with committed context and a failing DAG builder
    let server = Server::new(400).await;
    let fixture = Fixture::new(&server.url).await;
    let before = snapshot(&fixture.target).await;
    let mut chunks = fixture.target.subscribe_to_stream();
    // @step When a different session compacts the target
    let task = fixture.start();
    tokio::time::timeout(DEADLINE, server.arrived.wait())
        .await
        .expect("builder request");
    server.release.wait().await;
    let result = tokio::time::timeout(DEADLINE, task)
        .await
        .expect("bounded failure")
        .expect("task");
    // @step Then compaction returns an explicit failure without a substitute DAG
    assert!(
        result.is_err(),
        "builder failure must not become fallback success"
    );
    // @step And the committed context and Idle activity remain unchanged without completion
    assert_eq!(snapshot(&fixture.target).await, before);
    assert_clean(&fixture.target);
    assert_no_completion(&mut chunks);
}
