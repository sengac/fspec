use super::{
    fixture::{self, Fixture, TestResult},
    runtime,
};
use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use codelet_core::persistence::*;
use codelet_rpc_types::StreamChunk;
use serde_json::json;

pub async fn cycle(fixture: &Fixture, row: &str, cycle: usize, suffix: bool) -> TestResult {
    let mut inner = fixture.background.inner.lock().await;
    if row == "codex" {
        super::codex::retain_legacy_checkpoint(fixture.background.id)?;
    }
    let queue = register_generate_compaction_handler(
        fixture.background.id,
        &inner,
        fixture.directory.path().into(),
        fixture.background.clone(),
        Some(fixture.manager.clone()),
    );
    let manifest = load_session(fixture.background.id)?;
    let candidate = fixture::dag(manifest.messages.len(), cycle)?;
    if suffix {
        fixture::append_suffix(manifest.id)?;
        inner.messages.extend([
            rig::message::Message::Assistant {
                id: None,
                content: rig::OneOrMany::one(
                    rig::message::AssistantContent::tool_call_with_call_id(
                        "call_after_snapshot",
                        "call_after_snapshot".into(),
                        "lookup",
                        json!({"path":"src/deep.rs"}),
                    ),
                ),
            },
            rig::message::Message::tool_result_with_call_id(
                "call_after_snapshot",
                Some("call_after_snapshot".into()),
                fixture::SUFFIX,
            ),
        ]);
    }
    let server = super::transport::submission(row, &candidate).await;
    let output = runtime::Output::production(
        fixture.background.clone(),
        inner.current_provider_name().to_owned(),
    );
    if row == "stub" {
        runtime::stream_parameters(
            codelet_providers::stub_model::StubModel::new(),
            &mut inner,
            manifest.id,
            &output,
            Some(json!({"stub_tool_call":{"id":format!("summary-call-{cycle}"),"name":"inject_summary","arguments":{"content":candidate,"work_unit_id":"NONE"}}})),
        ).await?;
    } else {
        runtime::send_inner(
            row,
            fixture.directory.path(),
            &server,
            &mut inner,
            manifest.id,
            &output,
        )
        .await?;
        assert_eq!(
            server
                .received_requests()
                .await
                .ok_or("submission capture")?
                .len(),
            1,
            "submission must yield before another provider request"
        );
    }
    codelet_cli::interactive_helpers::validate_no_orphan_tool_calls(&inner.messages)
        .map_err(|errors| errors.join("; "))?;
    let checkpoints = if row == "codex" {
        super::codex::retain_legacy_checkpoint(fixture.background.id)?;
        let streamed = load_session(manifest.id)?;
        let checkpoints = get_session_messages_full(&streamed)?
            .into_iter()
            .filter(|message| message.metadata.contains_key("codexContinuation"))
            .collect::<Vec<_>>();
        assert!(
            checkpoints.iter().any(|checkpoint| manifest
                .messages
                .iter()
                .any(|reference| reference.message_id == checkpoint.id)),
            "real streaming must persist a prefix checkpoint"
        );
        assert!(
            checkpoints.iter().any(|checkpoint| !manifest
                .messages
                .iter()
                .any(|reference| reference.message_id == checkpoint.id)),
            "submission must persist a post-capture suffix checkpoint"
        );
        checkpoints
    } else {
        Vec::new()
    };
    assert!(
        inner.ensure_compaction_inference_ready().is_err(),
        "queued candidate must block inference until application"
    );
    output.assert_no_completion();
    assert!(queue.apply(&mut inner, &fixture.background)?);
    assert!(
        !queue.apply(&mut inner, &fixture.background)?,
        "duplicate application"
    );
    inner.ensure_compaction_inference_ready()?;
    let committed = load_session(manifest.id)?;
    let boundary = committed
        .compaction_commit
        .as_ref()
        .ok_or("missing commit")?;
    assert_eq!(boundary.identity.generation, cycle as u64);
    assert_eq!(
        committed.token_usage.current_context_tokens,
        boundary.input_tokens
    );
    for checkpoint in checkpoints {
        assert!(
            committed
                .messages
                .iter()
                .any(|reference| reference.message_id == checkpoint.id),
            "checkpoint original reference must remain in the manifest"
        );
        assert_eq!(
            serde_json::to_value(get_message(checkpoint.id)?.ok_or("lost checkpoint original")?)?,
            serde_json::to_value(checkpoint)?,
            "checkpoint originals must survive byte-equivalent without conversational conversion"
        );
    }
    assert!(!boundary.publication_pending);
    assert_eq!(inner.token_tracker.input_tokens, boundary.input_tokens);
    let snapshot = fixture
        .background
        .committed_context_snapshot()
        .ok_or("missing committed UI snapshot")?;
    assert_eq!(snapshot.generation, cycle as i64);
    assert_eq!(
        u64::from(snapshot.tokens.input_tokens),
        boundary.input_tokens
    );
    assert_eq!(
        snapshot.context_fill.effective_tokens,
        boundary.input_tokens as f64
    );
    assert_eq!(
        snapshot.covered_turns,
        Some(boundary.covered_prefix.len() as u32)
    );
    assert_eq!(
        snapshot.retained_turns,
        Some(boundary.verified_suffix.len() as u32)
    );
    assert!(snapshot.reconciled);
    let events = fixture.background.get_buffered_output(100);
    let context = events
        .iter()
        .rposition(|event| {
            matches!(event,
                StreamChunk::CommittedCompaction { snapshot } if snapshot.generation == cycle as i64
            )
        })
        .ok_or("missing committed context event")?;
    let complete = events
        .iter()
        .rposition(|event| matches!(event, StreamChunk::CompactionComplete { .. }))
        .ok_or("missing runtime completion event")?;
    assert!(context < complete, "completion preceded committed context");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, StreamChunk::CompactionComplete { .. }))
            .count(),
        cycle,
        "duplicate or missing completion"
    );
    Ok(())
}
