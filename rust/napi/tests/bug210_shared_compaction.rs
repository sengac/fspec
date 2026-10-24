//! Feature: spec/features/napi-compaction-continuation.feature

#![cfg(not(feature = "noop"))]

#[path = "bug210_shared/support.rs"]
mod support;

use codelet_rpc_types::{SessionId, StreamChunk};
use std::sync::atomic::Ordering;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_napi_sessions_use_the_shared_automatic_recovery_lifecycle() -> support::TestResult
{
    // @step Given an unfinished NAPI session encounters a context overflow
    let mut fixture = support::Fixture::new().await?;
    // @step When its session hook runs the agent loop
    fixture.send()?;
    let events = fixture.until_done().await?;
    for (_, chunk) in &events {
        let json = codelet_napi::stream_chunk_to_json_value(chunk);
        match chunk {
            StreamChunk::ContextSnapshot { snapshot } => {
                assert_eq!(json["type"], "contextSnapshot");
                assert_eq!(json["snapshot"], serde_json::to_value(snapshot)?);
            }
            StreamChunk::CommittedCompaction { snapshot } => {
                assert_eq!(json["type"], "committedCompaction");
                assert_eq!(json["snapshot"], serde_json::to_value(snapshot)?);
            }
            StreamChunk::CompactionProgressUpdate { progress } => {
                assert_eq!(json["type"], "compactionProgressUpdate");
                assert_eq!(json["progress"], serde_json::to_value(progress)?);
            }
            _ => {}
        }
    }
    // @step Then a committed summary is followed by inference without new user input
    for (index, session) in fixture.sessions.iter().enumerate() {
        assert!(fixture.inferences[index].load(Ordering::SeqCst) >= 1,
            "session {index} must reach the redirected provider before recovery assertions: {events:?}");
        assert_eq!(
            fixture.builders[index].load(Ordering::SeqCst),
            1,
            "session {index} must recover its provider overflow: {events:?}"
        );
        assert_eq!(fixture.inferences[index].load(Ordering::SeqCst), 2);
        let id = SessionId::from(session.id.to_string());
        let chunks: Vec<_> = events
            .iter()
            .filter(|(origin, _)| origin == &id)
            .map(|(_, chunk)| chunk)
            .collect();
        let complete = chunks
            .iter()
            .position(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }))
            .ok_or("originating session must receive completion")?;
        let continued = chunks
            .iter()
            .position(|chunk| matches!(chunk, StreamChunk::Text { text, .. } if text.contains(&format!("finished-session-{index}"))))
            .ok_or("originating task must continue")?;
        let done = chunks
            .iter()
            .position(|chunk| matches!(chunk, StreamChunk::Done))
            .ok_or("originating task must finish")?;
        assert!(complete < continued && continued < done);
        // @step And its events retain the originating session identity
        assert!(!chunks.iter().any(|chunk| matches!(chunk, StreamChunk::Text { text, .. } if text.contains(&format!("finished-session-{}", 1 - index)))));
        assert!(chunks
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::CommittedCompaction { .. })));
        assert!(chunks
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::ContextSnapshot { .. })));
        assert!(chunks.iter().any(|chunk| matches!(
            chunk,
            StreamChunk::CompactionProgressUpdate { progress: Some(_) }
        )));
        assert!(chunks.iter().any(|chunk| matches!(
            chunk,
            StreamChunk::CompactionProgressUpdate { progress: None }
        )));
        let manifest = codelet_core::persistence::load_session(session.id)?;
        let commit = manifest.compaction_commit.ok_or("durable summary commit")?;
        assert_eq!(commit.identity.session_id, session.id);
        assert_eq!(commit.source.session_id, session.id);
        assert!(session.get_compaction_progress().is_none());
    }
    assert!(events.iter().all(|(origin, _)| fixture
        .sessions
        .iter()
        .any(|session| origin.value == session.id.to_string())));
    Ok(())
}
