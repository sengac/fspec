//! Feature: spec/features/lossless-transcript-delivery.feature

#[path = "bug244/support.rs"]
pub mod support;

#[path = "bug244/history_fixture.rs"]
mod history_fixture;

#[path = "bug244/archival_prefix.rs"]
mod archival_prefix;

#[path = "bug244/live_resume.rs"]
mod live_resume;

use codelet_core::SessionManagerHandle;
use codelet_rpc_types::{StreamChunk, TranscriptAssembler, TranscriptError};
use support::{Fixture, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn long_transcript_resumes_exactly_once_across_both_transports() -> TestResult {
    // @step Given a real session containing more than 2479 ordered chunks including tool calls and results
    let fixture = Fixture::new().await?;
    fixture.seed()?;
    fixture.cold_restore()?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    assert!(background.get_buffered_output(usize::MAX).len() > 2479);
    // @step When its transcript is hydrated through embedded and WebSocket transports in bounded pages
    let (embedded, websocket, server) = fixture.backends().await?;
    for backend in [embedded, websocket] {
        // @step And more output arrives between pages and hydration is repeated
        let mut initial_snapshot = true;
        let delivered = hydrate_at_boundary(backend.as_ref(), &fixture.id, |complete| {
            if initial_snapshot {
                background.handle_output(StreamChunk::text("live-tail".into()));
                background.handle_output(StreamChunk::done());
                initial_snapshot = !complete;
            }
        })
        .await?;
        let expected = background.get_buffered_output(usize::MAX);
        let repeated = hydrate(backend.as_ref(), &fixture.id).await?;
        // @step Then every original and new event is available once in source order with complete tool content
        assert_eq!(content(&delivered)?, content(&expected)?);
        assert_eq!(
            content(&repeated)?,
            content(&background.get_buffered_output(usize::MAX))?
        );
    }
    server.abort();
    Ok(())
}

async fn hydrate_at_boundary(
    backend: &dyn codelet_fspec_tui::FspecBackend,
    id: &codelet_rpc_types::SessionId,
    boundary: impl FnMut(bool),
) -> Result<Vec<StreamChunk>, Box<dyn std::error::Error>> {
    let mut assembler = TranscriptAssembler::default();
    hydrate_from(backend, id, &mut assembler, boundary).await
}

async fn hydrate_from(
    backend: &dyn codelet_fspec_tui::FspecBackend,
    id: &codelet_rpc_types::SessionId,
    assembler: &mut TranscriptAssembler,
    mut boundary: impl FnMut(bool),
) -> Result<Vec<StreamChunk>, Box<dyn std::error::Error>> {
    let mut chunks = Vec::new();
    for _snapshot in 0..2 {
        let mut snapshot_end = None;
        loop {
            let page = backend
                .get_transcript_page(
                    id.clone(),
                    assembler.cursor().cloned(),
                    snapshot_end,
                    65_536,
                )
                .await?;
            assert!(serde_json::to_vec(&page)?.len() <= 65_536);
            if let Some(snapshot_end) = snapshot_end {
                assert_eq!(page.snapshot_end, snapshot_end);
            }
            if let Some(chunk) = assembler.accept(&page)? {
                chunks.push(chunk);
            }
            assert!(assembler.accept(&page)?.is_none());
            boundary(page.complete);
            if page.complete {
                break;
            }
            snapshot_end = Some(page.snapshot_end);
        }
    }
    Ok(chunks)
}

async fn hydrate(
    backend: &dyn codelet_fspec_tui::FspecBackend,
    id: &codelet_rpc_types::SessionId,
) -> Result<Vec<StreamChunk>, Box<dyn std::error::Error>> {
    hydrate_at_boundary(backend, id, |_| {}).await
}

fn content(chunks: &[StreamChunk]) -> Result<Vec<serde_json::Value>, serde_json::Error> {
    chunks.iter().map(serde_json::to_value).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn large_unicode_tool_results_cross_bounded_pages_without_truncation() -> TestResult {
    // @step Given a real session with a Unicode tool result larger than 256 KiB
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    let original = "完整🙂\n".repeat(32_768);
    background.handle_output(StreamChunk::tool_result(
        codelet_rpc_types::ToolResultInfo {
            tool_call_id: "large-original".into(),
            content: original.clone(),
            is_error: false,
        },
    ));
    // @step When the transcript crosses bounded pages through both transports
    let (embedded, websocket, server) = fixture.backends().await?;
    for backend in [embedded, websocket] {
        let delivered = hydrate(backend.as_ref(), &fixture.id).await?;
        // @step Then no page exceeds 64 KiB and the complete original tool result is available once
        assert_eq!(
            content(&delivered)?,
            content(&background.get_buffered_output(usize::MAX))?
        );
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn invalid_cursors_fail_visibly_without_corrupting_an_existing_transcript() -> TestResult {
    // @step Given an existing transcript and its generation cursor
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("preserved transcript".into()));
    let before = content(&background.get_buffered_output(usize::MAX))?;
    let (embedded, websocket, server) = fixture.backends().await?;
    for backend in [embedded, websocket] {
        // @step When a page uses an invalid generation or sequence
        let initial = backend
            .get_transcript_page(fixture.id.clone(), None, None, 1024)
            .await?;
        let mut wrong_generation = initial.next_cursor.clone();
        wrong_generation.generation = "missing-generation".into();
        let result = backend
            .get_transcript_page(fixture.id.clone(), Some(wrong_generation), None, 1024)
            .await;
        // @step Then a typed recoverable failure preserves the already displayed content
        assert!(matches!(
            result,
            Err(codelet_fspec_tui::transport::BackendError::Transcript(
                TranscriptError::GenerationMismatch
            ))
        ));
        let mut wrong_sequence = initial.next_cursor.clone();
        wrong_sequence.sequence = u64::MAX;
        assert!(matches!(
            backend
                .get_transcript_page(fixture.id.clone(), Some(wrong_sequence), None, 1024)
                .await,
            Err(codelet_fspec_tui::transport::BackendError::Transcript(
                TranscriptError::InvalidCursor
            ))
        ));
        assert!(matches!(
            backend
                .get_transcript_page(fixture.id.clone(), None, None, 65_537)
                .await,
            Err(codelet_fspec_tui::transport::BackendError::Transcript(
                TranscriptError::InvalidByteLimit
            ))
        ));
        assert!(matches!(
            backend
                .get_transcript_page(
                    codelet_rpc_types::SessionId::new("missing-session"),
                    None,
                    None,
                    1024
                )
                .await,
            Err(codelet_fspec_tui::transport::BackendError::Transcript(
                TranscriptError::SessionUnavailable(_)
            ))
        ));
        assert_eq!(
            content(&background.get_buffered_output(usize::MAX))?,
            before
        );
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn broadcast_lag_recovers_from_the_authoritative_cursor() -> TestResult {
    // @step Given a real session with a subscriber that falls more than 256 events behind
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("before lag".into()));
    background.handle_output(StreamChunk::done());
    let prefix_len = background.get_buffered_output(usize::MAX).len();
    let (embedded, websocket, server) = fixture.backends().await?;
    let mut subscribers = Vec::new();
    for backend in [embedded, websocket] {
        let mut assembler = TranscriptAssembler::default();
        hydrate_from(backend.as_ref(), &fixture.id, &mut assembler, |_| {}).await?;
        subscribers.push((backend, assembler));
    }
    let mut receiver = background.subscribe_to_stream();
    for index in 0..2700 {
        background.handle_output(StreamChunk::text(format!("lagged-{index:04}\n")));
    }
    background.handle_output(StreamChunk::done());
    assert!(
        matches!(receiver.recv().await, Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) if skipped > 256)
    );
    // @step When the subscriber recovers using its transcript cursor
    for (backend, mut assembler) in subscribers {
        let delivered = hydrate_from(backend.as_ref(), &fixture.id, &mut assembler, |_| {}).await?;
        // @step Then the complete missed content and final tail are delivered exactly once
        assert_eq!(
            content(&delivered)?,
            content(&background.get_buffered_output(usize::MAX)[prefix_len..])?
        );
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn repeated_restoration_does_not_rebroadcast_history() -> TestResult {
    // @step Given a saved real session and a live output subscriber
    let fixture = Fixture::new().await?;
    fixture.seed()?;
    fixture.manager.destroy_session(&fixture.id.value)?;
    let manifest =
        codelet_core::persistence::load_session(uuid::Uuid::parse_str(&fixture.id.value)?)?;
    fixture
        .manager
        .create_session_from_manifest(&manifest, &manifest.provider)
        .await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    let mut receiver = background.subscribe_to_stream();
    let envelopes = codelet_core::persistence::get_session_message_envelopes(background.id)?;
    // @step When the same saved history is restored twice
    fixture
        .manager
        .restore_session_messages(&fixture.id, envelopes.clone())?;
    let once = content(&background.get_buffered_output(usize::MAX))?;
    assert!(once.len() > 2479);
    fixture
        .manager
        .restore_session_messages(&fixture.id, envelopes)?;
    // @step Then the transcript contains one copy and no historical content is broadcast as new activity
    assert_eq!(content(&background.get_buffered_output(usize::MAX))?, once);
    assert!(matches!(
        receiver.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    Ok(())
}
