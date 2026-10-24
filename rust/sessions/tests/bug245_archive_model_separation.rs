//! Feature: spec/features/archival-model-replay-separation.feature

#[path = "bug245/support.rs"]
mod support;

use codelet_core::persistence::*;
use codelet_core::SessionManagerHandle;
use codelet_rpc_types::{StreamChunk, TranscriptError};
use serde_json::json;
use support::{replay::*, *};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn compacted_cold_resume_separates_original_transcript_and_physical_model_input() -> TestResult
{
    // @step Given a saved tool conversation with two committed compactions and an uncompacted suffix
    let Some(id) = cold_process_session()? else {
        let (directory, manager, id, _) = manager().await?;
        seed_compacted(&manager, &id).await?;
        drop(manager);
        return run_cold_process(
            directory.path(),
            &id,
            "compacted_cold_resume_separates_original_transcript_and_physical_model_input",
        )
        .await;
    };
    let before = serde_json::to_value(load_session(uuid::Uuid::parse_str(&id.value)?)?)?;
    // @step When a fresh process resumes the session and reads its transcript
    let manager = cold_resume(&id)?;
    let chunks = transcript(&manager, &id)?;
    let encoded = serde_json::to_string(&chunks)?;
    // @step Then every original text tool argument and tool result appears once in order without synthetic user summaries
    let mut previous = 0;
    for expected in [
        "covered original 雪",
        "original assistant",
        "full original arguments",
        "complete original tool result 雪",
        "uncompacted suffix",
    ] {
        assert_eq!(
            encoded.matches(expected).count(),
            1,
            "original missing or duplicated: {expected}: {encoded}"
        );
        let position = encoded.find(expected).ok_or("original position")?;
        assert!(position >= previous);
        previous = position;
    }
    assert!(!chunks.iter().any(|chunk| matches!(chunk, StreamChunk::UserInput { text } if text.contains("reduced committed work"))));
    manager.resume_session(&id)?;
    assert_eq!(
        conversation(&transcript(&manager, &id)?)?,
        conversation(&chunks)?
    );
    assert_eq!(
        serde_json::to_value(load_session(uuid::Uuid::parse_str(&id.value)?)?)?,
        before
    );
    // @step And the next physical provider request contains only the committed summary and suffix
    let wire = next_anthropic_wire(&manager, &id, 0).await?.to_string();
    assert!(wire.contains("reduced committed work 2"), "{wire}");
    assert!(wire.contains("uncompacted suffix"), "{wire}");
    for excluded in [
        "covered original 雪",
        "original assistant",
        "full original arguments",
        "complete original tool result 雪",
    ] {
        assert!(
            !wire.contains(excluded),
            "archive leaked into inference: {wire}"
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn fully_compacted_archives_retain_their_terminal_metadata_boundary() -> TestResult {
    // @step Given a compacted conversation without an uncompacted suffix
    let (_directory, manager, id, _) = manager().await?;
    seed_compacted(&manager, &id).await?;
    compact(&manager, &id).await?;
    manager.destroy_session(&id.value)?;
    // @step When its archive is cold restored
    manager.resume_session(&id)?;
    let chunks = transcript(&manager, &id)?;
    // @step Then one labelled compaction marker follows all covered originals without original message coordinates
    let markers = chunks.iter().enumerate().filter(|(_, chunk)| {
        matches!(chunk, StreamChunk::UserNotification { message, .. } if message.starts_with("[compaction metadata]"))
    }).map(|(index, _)| index).collect::<Vec<_>>();
    assert_eq!(markers.len(), 1);
    assert!(chunks
        .iter()
        .enumerate()
        .filter(|(_, chunk)| matches!(
            chunk,
            StreamChunk::UserInput { .. }
                | StreamChunk::Text { .. }
                | StreamChunk::Thinking { .. }
                | StreamChunk::ToolCall { .. }
                | StreamChunk::ToolResult { .. }
        ))
        .all(|(index, _)| index < markers[0]));
    let mut assembler = codelet_rpc_types::TranscriptAssembler::default();
    let session = manager.get_session(&id.value)?;
    let mut snapshot = None;
    loop {
        let page = session.get_transcript_page(assembler.cursor().cloned(), snapshot, 1024)?;
        if matches!(assembler.accept(&page)?, Some(StreamChunk::UserNotification { message, .. }) if message.starts_with("[compaction metadata]"))
        {
            assert!(assembler.original().is_none());
        }
        if page.complete {
            break;
        }
        snapshot = Some(page.snapshot_end);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn compaction_metadata_marks_the_covered_original_boundary() -> TestResult {
    // @step Given a saved tool conversation with two committed compactions and an uncompacted suffix
    let (_directory, manager, id, _) = manager().await?;
    seed_compacted(&manager, &id).await?;
    let uuid = uuid::Uuid::parse_str(&id.value)?;
    let manifest = load_session(uuid)?;
    let before = serde_json::to_value(&manifest)?;
    let state = manifest.compaction.as_ref().ok_or("missing compaction")?;
    let commit = manifest
        .compaction_commit
        .as_ref()
        .ok_or("missing commit")?;
    // @step When its archive is restored repeatedly
    manager.destroy_session(&id.value)?;
    manager.resume_session(&id)?;
    manager.resume_session(&id)?;
    let chunks = transcript(&manager, &id)?;
    let markers = chunks
        .iter()
        .enumerate()
        .filter_map(|(index, chunk)| match chunk {
            StreamChunk::UserNotification { message, .. }
                if message.starts_with("[compaction metadata]") =>
            {
                Some((index, message))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    // @step Then a single labelled compaction marker preserves the committed summary generation timestamp and covered boundary
    assert_eq!(markers.len(), 1);
    let (marker_index, marker) = markers[0];
    assert!(marker.contains(&state.summary));
    assert!(marker.contains(&state.compacted_at.to_rfc3339()));
    assert!(marker.contains(&format!("generation {}", commit.identity.generation)));
    assert!(marker.contains(&format!("originals 0..{}", state.compacted_before_index)));
    // @step And the marker follows the covered originals before the suffix without changing durable state
    let covered = chunks.iter().position(|chunk| matches!(chunk, StreamChunk::ToolResult { tool_result, .. } if tool_result.content.contains("complete original tool result"))).ok_or("missing covered result")?;
    let suffix = chunks.iter().position(|chunk| matches!(chunk, StreamChunk::UserInput { text } if text == "uncompacted suffix")).ok_or("missing suffix")?;
    assert!(covered < marker_index && marker_index < suffix);
    assert_eq!(serde_json::to_value(load_session(uuid)?)?, before);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn unavailable_original_is_a_visible_typed_transcript_failure() -> TestResult {
    // @step Given a compacted session whose covered original record is unavailable
    let (_directory, manager, id, _) = manager().await?;
    seed_compacted(&manager, &id).await?;
    let uuid = uuid::Uuid::parse_str(&id.value)?;
    let mut manifest = load_session(uuid)?;
    let original = manifest.messages[0].message_id;
    let missing = uuid::Uuid::new_v4();
    manifest.messages[0].message_id = missing;
    save_session(&manifest)?;
    manager.destroy_session(&id.value)?;
    let _ = manager.resume_session(&id);
    // @step When the archived transcript is requested
    let result = manager
        .get_session(&id.value)?
        .get_transcript_page(None, None, 1024);
    // @step Then a typed archive failure identifies the unavailable original instead of silently omitting it
    assert!(
        matches!(&result, Err(TranscriptError::JournalUnavailable(message)) if message.contains(&missing.to_string())),
        "{result:?}"
    );
    manifest = load_session(uuid)?;
    manifest.messages[0].message_id = original;
    save_session(&manifest)?;
    manager.resume_session(&id)?;
    let recovered = serde_json::to_string(&transcript(&manager, &id)?)?;
    assert_eq!(recovered.matches("covered original 雪").count(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn large_archive_hydration_does_not_fill_runtime_output_buffer() -> TestResult {
    // @step Given a saved archive with many original records and a large tool result
    let (_directory, manager, id, _) = manager().await?;
    let uuid = uuid::Uuid::parse_str(&id.value)?;
    let mut manifest = load_session(uuid)?;
    for index in 0..128 {
        append(
            &mut manifest,
            "user",
            json!(format!("original {index:04} 雪")),
        )?;
    }
    append(
        &mut manifest,
        "assistant",
        json!([{"type":"tool_use","id":"large-call","name":"read_file","input":{"path":"large.txt"}}]),
    )?;
    let large = "complete large result 雪\n".repeat(4000);
    append(
        &mut manifest,
        "user",
        json!([{"type":"tool_result","tool_use_id":"large-call","content":large}]),
    )?;
    compact(&manager, &id).await?;
    manifest = load_session(uuid)?;
    let before = serde_json::to_value(&manifest)?;
    drop(manager);
    // @step When its transcript is hydrated repeatedly through bounded cursor pages
    let manager = cold_resume(&id)?;
    let chunks = transcript(&manager, &id)?;
    assert_eq!(
        serde_json::to_value(transcript(&manager, &id)?)?,
        serde_json::to_value(&chunks)?
    );
    // @step Then all records retain original positions identities timestamps and complete payloads
    let first = manager
        .get_session(&id.value)?
        .get_transcript_page(None, None, 1024)?;
    let source = serde_json::to_value(first)?["fragment"]["original"].clone();
    let original = get_message(manifest.messages[0].message_id)?.ok_or("first original")?;
    assert_eq!(source["turn_index"], 0);
    assert_eq!(source["message_id"], original.id.to_string());
    assert_eq!(source["timestamp"], original.created_at.to_rfc3339());
    assert_eq!(source["chunk_index"], 0);
    let encoded = serde_json::to_string(&chunks)?;
    for index in 0..128 {
        assert_eq!(
            encoded.matches(&format!("original {index:04} 雪")).count(),
            1
        );
    }
    assert!(chunks.iter().any(|chunk| matches!(chunk, StreamChunk::ToolResult {tool_result, ..} if tool_result.content == large)));
    // @step And hydration retains at most one decoded original without filling the runtime output buffer or changing durable model state
    assert!(manager.get_session(&id.value)?.resident_transcript_chunks() < 10);
    assert_eq!(serde_json::to_value(load_session(uuid)?)?, before);
    Ok(())
}
