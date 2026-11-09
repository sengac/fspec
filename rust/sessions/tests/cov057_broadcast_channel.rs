//! Feature: spec/features/broadcast-channel-for-parent-stream-observation.feature
//!
//! COV-057: drives the real `BackgroundSession::handle_output` and
//! `subscribe_to_stream` (the supervisor broadcast channel) on a stub-provider
//! session, asserting both the per-session output buffer and what each
//! broadcast receiver observes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "cov057_support/session.rs"]
pub mod support;

use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::{BackgroundSession, SUPERVISOR_BROADCAST_CAPACITY};
use support::{data_directory, stub_session, TestResult};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};

fn text_of(chunk: &StreamChunk) -> Option<&str> {
    match chunk {
        StreamChunk::Text { text, .. } => Some(text),
        _ => None,
    }
}

fn buffered_texts(session: &BackgroundSession) -> Vec<String> {
    session
        .get_buffered_output(10_000)
        .iter()
        .filter_map(text_of)
        .map(str::to_owned)
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_broadcast_with_no_subscribers_still_buffers_normally() -> TestResult {
    // @step Given a BackgroundSession with broadcast channel initialized
    let directory = data_directory()?;
    let (session, _input) = stub_session(directory.path(), "broadcast")?;
    // @step And no watchers have subscribed to the stream
    // (no subscribe_to_stream call: the broadcast has zero receivers)
    // @step When handle_output is called with a TextDelta chunk
    session.handle_output(StreamChunk::text("unobserved delta".into()));
    // @step Then the chunk should be added to the output buffer
    assert_eq!(buffered_texts(&session), vec!["unobserved delta"]);
    // @step And no error should occur from the broadcast
    session.handle_output(StreamChunk::text("second delta".into()));
    assert_eq!(
        buffered_texts(&session),
        vec!["unobserved delta", "second delta"]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_single_watcher_receives_chunks_via_broadcast() -> TestResult {
    // @step Given a BackgroundSession with broadcast channel initialized
    let directory = data_directory()?;
    let (session, _input) = stub_session(directory.path(), "broadcast")?;
    // @step And a watcher has called subscribe_to_stream to get a receiver
    let mut watcher = session.subscribe_to_stream();
    // @step When handle_output is called with a TextDelta chunk
    session.handle_output(StreamChunk::text("observed delta".into()));
    // @step Then the watcher should receive the same chunk via its receiver
    let received = watcher.try_recv()?;
    assert_eq!(text_of(&received), Some("observed delta"));
    // @step And the chunk should also be buffered normally
    let buffered = session.get_buffered_output(10_000);
    let last = buffered.last().ok_or("output buffer is empty")?;
    assert_eq!(serde_json::to_value(last)?, serde_json::to_value(received)?);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_multiple_watchers_receive_chunks_independently() -> TestResult {
    // @step Given a BackgroundSession with broadcast channel initialized
    let directory = data_directory()?;
    let (session, _input) = stub_session(directory.path(), "broadcast")?;
    // @step And watcher A has subscribed to the stream
    let mut watcher_a = session.subscribe_to_stream();
    // @step And watcher B has subscribed to the stream
    let mut watcher_b = session.subscribe_to_stream();
    // @step When handle_output is called with a TextDelta chunk
    session.handle_output(StreamChunk::text("shared delta".into()));
    // @step Then watcher A should receive the chunk via its receiver
    let chunk_a = watcher_a.try_recv()?;
    assert_eq!(text_of(&chunk_a), Some("shared delta"));
    // @step And watcher B should receive the chunk via its receiver
    let chunk_b = watcher_b.try_recv()?;
    assert_eq!(text_of(&chunk_b), Some("shared delta"));
    // @step And both received chunks should be identical
    assert_eq!(
        serde_json::to_value(chunk_a)?,
        serde_json::to_value(chunk_b)?
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_slow_watcher_receives_lagged_error_when_falling_behind() -> TestResult {
    // @step Given a BackgroundSession with broadcast channel capacity of 256
    assert_eq!(SUPERVISOR_BROADCAST_CAPACITY, 256);
    let directory = data_directory()?;
    let (session, _input) = stub_session(directory.path(), "broadcast")?;
    // @step And a watcher has subscribed to the stream
    let mut watcher = session.subscribe_to_stream();
    // @step And the watcher has not consumed any chunks
    // @step When handle_output is called 300 times with chunks
    for index in 0..300 {
        session.handle_output(StreamChunk::text(format!("delta {index}")));
    }
    // @step Then the watcher should receive RecvError::Lagged when trying to receive
    match watcher.recv().await {
        Err(RecvError::Lagged(skipped)) => assert_eq!(skipped, 300 - 256),
        other => panic!("expected RecvError::Lagged, got {other:?}"),
    }
    assert_eq!(buffered_texts(&session).len(), 300);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_dropped_receiver_does_not_affect_other_watchers() -> TestResult {
    // @step Given a BackgroundSession with broadcast channel initialized
    let directory = data_directory()?;
    let (session, _input) = stub_session(directory.path(), "broadcast")?;
    // @step And watcher A has subscribed to the stream
    let watcher_a = session.subscribe_to_stream();
    // @step And watcher B has subscribed to the stream
    let mut watcher_b = session.subscribe_to_stream();
    // @step When watcher A drops its receiver
    drop(watcher_a);
    // @step And handle_output is called with a TextDelta chunk
    session.handle_output(StreamChunk::text("after drop".into()));
    // @step Then watcher B should still receive the chunk normally
    assert_eq!(text_of(&watcher_b.try_recv()?), Some("after drop"));
    // @step And the parent session should continue operating normally
    session.handle_output(StreamChunk::text("still running".into()));
    assert_eq!(text_of(&watcher_b.try_recv()?), Some("still running"));
    assert_eq!(
        buffered_texts(&session),
        vec!["after drop", "still running"]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_late_subscriber_starts_receiving_from_current_position() -> TestResult {
    // @step Given a BackgroundSession with broadcast channel initialized
    let directory = data_directory()?;
    let (session, _input) = stub_session(directory.path(), "broadcast")?;
    // @step And handle_output has been called 10 times with chunks
    for index in 0..10 {
        session.handle_output(StreamChunk::text(format!("early {index}")));
    }
    // @step When a new watcher subscribes to the stream
    let mut watcher = session.subscribe_to_stream();
    // @step And handle_output is called with a new chunk
    session.handle_output(StreamChunk::text("fresh".into()));
    // @step Then the new watcher should receive only the new chunk
    assert_eq!(text_of(&watcher.try_recv()?), Some("fresh"));
    // @step And the new watcher should not receive the previous 10 chunks
    assert!(matches!(watcher.try_recv(), Err(TryRecvError::Empty)));
    assert_eq!(buffered_texts(&session).len(), 11);
    Ok(())
}
