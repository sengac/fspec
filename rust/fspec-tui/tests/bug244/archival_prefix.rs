use crate::support::{Fixture, TestResult};
use codelet_rpc_types::{
    StreamChunk, ToolResultInfo, TranscriptAssembler, TranscriptError, TranscriptOriginal,
};
use std::sync::atomic::{AtomicBool, Ordering};

fn original(index: usize) -> TranscriptOriginal {
    TranscriptOriginal {
        turn_index: index,
        message_id: format!("persisted-{index}"),
        timestamp: "2020-01-02T03:04:05Z".into(),
        chunk_index: 0,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn clearing_during_archive_preparation_rejects_the_old_generation() -> TestResult {
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    let before = background.get_transcript_page(None, None, 1024)?;
    let events = std::iter::once_with(|| {
        tokio::task::block_in_place(|| background.clear_history())
            .map_err(|error| TranscriptError::JournalUnavailable(error.to_string()))?;
        Ok((
            StreamChunk::text("obsolete archive".into()),
            Some(original(0)),
        ))
    });
    assert!(matches!(
        background.seed_archival_transcript(events),
        Err(TranscriptError::GenerationMismatch)
    ));
    let after = background.get_transcript_page(None, None, 1024)?;
    assert_ne!(before.generation, after.generation);
    assert!(
        !serde_json::to_string(&background.get_buffered_output(usize::MAX))?
            .contains("obsolete archive")
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn disk_prefix_preserves_original_coordinates_and_a_concurrent_runtime_suffix() -> TestResult
{
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    let large = "完整🙂\n".repeat(32_768);
    let events = (0..10).map(|index| {
        if index == 3 {
            background.handle_output(StreamChunk::text("concurrent runtime tail".into()));
        }
        let chunk = if index == 5 {
            StreamChunk::tool_result(ToolResultInfo {
                tool_call_id: "large-original".into(),
                content: large.clone(),
                is_error: false,
            })
        } else {
            StreamChunk::text(format!("original-{index}"))
        };
        Ok((chunk, Some(original(index))))
    });
    assert!(background.seed_archival_transcript(events)?);
    assert_eq!(background.resident_transcript_chunks(), 1);
    let consumed = AtomicBool::new(false);
    assert!(
        !background.seed_archival_transcript(std::iter::once_with(|| {
            consumed.store(true, Ordering::SeqCst);
            Ok((StreamChunk::done(), None))
        }))?
    );
    assert!(!consumed.load(Ordering::SeqCst));
    let expected = background.get_buffered_output(usize::MAX);
    assert_eq!(expected.len(), 11);
    assert_eq!(background.resident_transcript_chunks(), 1);
    let (embedded, websocket, server) = fixture.backends().await?;
    for backend in [embedded, websocket] {
        let mut assembler = TranscriptAssembler::default();
        let mut snapshot_end = None;
        let mut delivered = Vec::new();
        let mut coordinates = Vec::new();
        loop {
            let page = backend
                .get_transcript_page(
                    fixture.id.clone(),
                    assembler.cursor().cloned(),
                    snapshot_end,
                    1024,
                )
                .await?;
            assert!(serde_json::to_vec(&page)?.len() <= 1024);
            if let Some(chunk) = assembler.accept(&page)? {
                delivered.push(chunk);
                coordinates.push(assembler.original().cloned());
            }
            if page.complete {
                break;
            }
            snapshot_end = Some(page.snapshot_end);
        }
        assert_eq!(
            serde_json::to_value(&delivered)?,
            serde_json::to_value(&expected)?
        );
        assert_eq!(
            coordinates[..10],
            (0..10)
                .map(|index| Some(original(index)))
                .collect::<Vec<_>>()
        );
        assert!(coordinates[10].is_none());
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn failed_disk_prefix_is_not_installed_and_can_retry_without_losing_live_output() -> TestResult
{
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("preserved live output".into()));
    let before = background.get_transcript_page(None, None, 1024)?;
    let live = serde_json::to_value(background.get_buffered_output(usize::MAX))?;
    let result = background.seed_archival_transcript([
        Ok((
            StreamChunk::text("partial archive".into()),
            Some(original(0)),
        )),
        Err(TranscriptError::JournalUnavailable(
            "missing original".into(),
        )),
    ]);
    assert!(matches!(
        result,
        Err(TranscriptError::JournalUnavailable(_))
    ));
    assert!(
        matches!(background.get_transcript_page(None, None, 1024), Err(TranscriptError::JournalUnavailable(message)) if message.contains("missing original"))
    );
    assert_eq!(
        serde_json::to_value(background.get_buffered_output(usize::MAX))?,
        live
    );
    assert!(background.seed_archival_transcript([Ok((StreamChunk::done(), Some(original(0))))])?);
    assert_eq!(background.completed_turn_count(), 1);
    assert_ne!(
        background.get_transcript_page(None, None, 1024)?.generation,
        before.generation
    );
    assert_eq!(background.get_info().message_count, 1);
    assert_eq!(background.get_buffered_output(usize::MAX).len(), 2);
    Ok(())
}
