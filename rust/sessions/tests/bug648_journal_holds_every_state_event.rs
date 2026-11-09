//! BUG-648: a session's transcript journal is its event log, so a client
//! that attaches later replays every state event it missed.
//!
//! Feature: spec/features/attaching-to-a-running-session-rebuilds-its-live-state.feature

#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_rpc_types::{
    NotificationSeverity, RequestHolder, RequestKind, RequestRoute, RequestStatus, SessionState,
    SessionStatus, StreamChunk, TranscriptAssembler,
};
use support::TestResult;

/// Every chunk of the session's transcript, read page by page from the start.
fn journal(
    session: &codelet_sessions::background_session::BackgroundSession,
) -> Result<Vec<StreamChunk>, Box<dyn std::error::Error>> {
    let mut assembler = TranscriptAssembler::default();
    let mut snapshot = None;
    let mut chunks = Vec::new();
    loop {
        let page = session.get_transcript_page(assembler.cursor().cloned(), snapshot, 4096)?;
        if let Some(chunk) = assembler.accept(&page)? {
            chunks.push(chunk);
        }
        if page.complete {
            return Ok(chunks);
        }
        snapshot = Some(page.snapshot_end);
    }
}

fn encoded(chunks: &[StreamChunk]) -> Result<Vec<String>, serde_json::Error> {
    chunks.iter().map(serde_json::to_string).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn request_and_provider_waiting_events_are_journaled_where_they_were_broadcast() -> TestResult
{
    // @step Given a session that broadcasts a text chunk, an InteractionRequestChanged chunk, a ProviderWaiting chunk, a status change and another text chunk
    let (_directory, manager, id, _) = support::manager().await?;
    let session = manager.get_session(&id.value)?;
    let mut broadcast = manager.chunks_tx().subscribe();
    session.handle_output(StreamChunk::text("before".into()));
    session.handle_output(StreamChunk::InteractionRequestChanged {
        route: RequestRoute {
            request_id: "request-1".into(),
            origin: id.clone(),
            kind: RequestKind::Question,
            chain: vec![id.clone()],
            holder: RequestHolder::User,
        },
        status: RequestStatus::Pending,
    });
    session.handle_output(StreamChunk::ProviderWaiting {
        message: Some("Waiting for the provider".into()),
    });
    session.set_status(SessionStatus::Running);
    session.handle_output(StreamChunk::user_notification(
        "after".into(),
        NotificationSeverity::Info,
    ));
    let mut sent = Vec::new();
    while let Ok((from, chunk)) = broadcast.try_recv() {
        if from == id {
            sent.push(chunk);
        }
    }
    assert!(sent
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::InteractionRequestChanged { .. })));
    assert!(sent
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::ProviderWaiting { .. })));

    // @step When its transcript is read page by page
    let pages = journal(&session)?;

    // @step Then the pages hold every chunk it broadcast, in the order it broadcast them
    let tail = pages
        .len()
        .checked_sub(sent.len())
        .ok_or("the transcript is shorter than the broadcast")?;
    assert_eq!(encoded(&pages[tail..])?, encoded(&sent)?);

    // @step And the last status change in the pages is the session's status
    let last = pages
        .iter()
        .rev()
        .find_map(|chunk| match chunk {
            StreamChunk::SessionStateChange { state } => Some(*state),
            _ => None,
        })
        .ok_or("no status change in the transcript")?;
    assert_eq!(last, SessionState::Running);
    assert_eq!(session.get_status(), SessionStatus::Running);
    Ok(())
}
