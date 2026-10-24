use codelet_rpc_types::{
    StreamChunk, TranscriptAssembler, TranscriptCursor, TranscriptError, TranscriptFragment,
    TranscriptPage,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fragment(text: &str, offset: usize, total: usize) -> TranscriptPage {
    let end = offset + text.len();
    let finished = end == total;
    TranscriptPage {
        generation: "generation".into(),
        snapshot_end: 1,
        fragment: Some(TranscriptFragment {
            sequence: 0,
            byte_offset: offset,
            total_bytes: total,
            json_fragment: text.into(),
            original: None,
        }),
        next_cursor: TranscriptCursor {
            generation: "generation".into(),
            sequence: u64::from(finished),
            byte_offset: if finished { 0 } else { end },
        },
        complete: finished,
    }
}

#[test]
fn arbitrary_event_inspection_requires_a_complete_event_boundary() -> TestResult {
    let cursor = TranscriptCursor {
        generation: "generation".into(),
        sequence: 17,
        byte_offset: 0,
    };
    let mut assembler = TranscriptAssembler::from_cursor(cursor.clone())?;
    let json = serde_json::to_string(&StreamChunk::done())?;
    let mut page = fragment(&json, 0, json.len());
    page.fragment.as_mut().ok_or("fragment")?.sequence = 17;
    page.next_cursor.sequence = 18;
    page.snapshot_end = 18;
    assert!(matches!(
        assembler.accept(&page),
        Ok(Some(StreamChunk::Done))
    ));
    let mut partial = cursor;
    partial.byte_offset = 1;
    assert!(matches!(
        TranscriptAssembler::from_cursor(partial),
        Err(TranscriptError::InvalidCursor)
    ));
    Ok(())
}

#[test]
fn original_coordinates_survive_fragmentation_and_reject_mixed_records() -> TestResult {
    let original = codelet_rpc_types::TranscriptOriginal {
        turn_index: 17,
        message_id: "persisted-record".into(),
        timestamp: "2020-01-02T03:04:05Z".into(),
        chunk_index: 4,
    };
    let json = serde_json::to_string(&StreamChunk::text("original content".into()))?;
    let mut first = fragment(&json[..1], 0, json.len());
    first.fragment.as_mut().ok_or("fragment")?.original = Some(original.clone());
    let mut last = fragment(&json[1..], 1, json.len());
    last.fragment.as_mut().ok_or("fragment")?.original = Some(original.clone());
    let mut assembler = TranscriptAssembler::default();
    assert!(assembler.accept(&first)?.is_none());
    assert!(assembler.original().is_none());
    let mut mixed = last.clone();
    mixed
        .fragment
        .as_mut()
        .ok_or("fragment")?
        .original
        .as_mut()
        .ok_or("identity")?
        .chunk_index += 1;
    assert!(matches!(
        assembler.accept(&mixed),
        Err(TranscriptError::InvalidCursor)
    ));
    assert!(assembler.accept(&last)?.is_some());
    assert_eq!(assembler.original(), Some(&original));
    Ok(())
}

#[test]
fn every_unicode_boundary_preserves_the_original_event_and_wire_shape() -> TestResult {
    let original = StreamChunk::text("完整🙂\n\t\"quoted\"\\日本語".into());
    let json = serde_json::to_string(&original)?;
    for (split, _) in json.char_indices().skip(1) {
        let mut assembler = TranscriptAssembler::default();
        let first = fragment(&json[..split], 0, json.len());
        let wire = serde_json::to_string(&first)?;
        let decoded = serde_json::from_str::<TranscriptPage>(&wire)?;
        assert_eq!(decoded, first);
        assert!(assembler.accept(&decoded)?.is_none());
        let last = fragment(&json[split..], split, json.len());
        let delivered = assembler.accept(&last)?.ok_or("complete")?;
        assert_eq!(serde_json::to_string(&delivered)?, json);
    }
    Ok(())
}

#[test]
fn retries_are_idempotent_and_invalid_pages_preserve_the_cursor() -> TestResult {
    let original = StreamChunk::text("完整🙂\n".into());
    let json = serde_json::to_string(&original)?;
    let split = json.find('完').ok_or("Unicode payload")?;
    let first = fragment(&json[..split], 0, json.len());
    let last = fragment(&json[split..], split, json.len());
    let mut assembler = TranscriptAssembler::default();
    assert!(assembler.accept(&first)?.is_none());
    assert!(assembler.accept(&first)?.is_none());
    let cursor = assembler.cursor().cloned();
    let mut invalid = last.clone();
    invalid.fragment.as_mut().ok_or("fragment")?.byte_offset += 1;
    assert!(matches!(
        assembler.accept(&invalid),
        Err(TranscriptError::InvalidCursor)
    ));
    assert_eq!(assembler.cursor(), cursor.as_ref());
    let delivered = assembler.accept(&last)?.ok_or("whole event")?;
    assert_eq!(
        serde_json::to_value(delivered)?,
        serde_json::to_value(original)?
    );
    assert!(assembler.accept(&last)?.is_none());
    assert!(assembler.accept(&first)?.is_none());
    Ok(())
}

#[test]
fn malformed_final_payload_can_retry_without_losing_prior_fragments() -> TestResult {
    let original = serde_json::to_string(&StreamChunk::done())?;
    let first = fragment(&original[..1], 0, original.len());
    let last = fragment(&original[1..], 1, original.len());
    let mut assembler = TranscriptAssembler::default();
    assembler.accept(&first)?;
    let mut invalid = last.clone();
    invalid.fragment.as_mut().ok_or("fragment")?.json_fragment = "x".repeat(original.len() - 1);
    assert!(matches!(
        assembler.accept(&invalid),
        Err(TranscriptError::InvalidPayload(_))
    ));
    assert!(matches!(
        assembler.accept(&last),
        Ok(Some(StreamChunk::Done))
    ));
    Ok(())
}

#[test]
fn missing_and_cross_generation_fragments_fail_closed() -> TestResult {
    let original = serde_json::to_string(&StreamChunk::text("original".into()))?;
    let mut assembler = TranscriptAssembler::default();
    let skipped = fragment(&original[1..], 1, original.len());
    assert!(matches!(
        assembler.accept(&skipped),
        Err(TranscriptError::InvalidCursor)
    ));
    assert!(assembler.cursor().is_none());
    let first = fragment(&original[..1], 0, original.len());
    assembler.accept(&first)?;
    let mut changed = skipped;
    changed.generation = "replacement".into();
    changed.next_cursor.generation = changed.generation.clone();
    assert!(matches!(
        assembler.accept(&changed),
        Err(TranscriptError::GenerationMismatch)
    ));
    Ok(())
}
