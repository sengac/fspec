#![allow(dead_code)]

#[path = "../cmpct060/support.rs"]
pub mod replay;

use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};
use codelet_core::persistence::compaction_boundary::*;
use codelet_core::persistence::compaction_commit::*;
use codelet_core::persistence::*;
use codelet_rpc_types::{SessionId, StreamChunk, TranscriptAssembler};
use codelet_sessions::SessionManager;
pub use replay::TestResult;
use serde_json::{json, Value};

pub fn append(manifest: &mut SessionManifest, role: &str, content: Value) -> TestResult {
    append_message_with_metadata(
        manifest,
        role,
        "",
        serde_json::from_value(json!({"message":{"role":role,"content":content}}))?,
    )?;
    Ok(())
}

pub fn conversation(chunks: &[StreamChunk]) -> Result<Value, serde_json::Error> {
    serde_json::to_value(
        chunks
            .iter()
            .filter(|chunk| {
                matches!(
                    chunk,
                    StreamChunk::UserInput { .. }
                        | StreamChunk::IncomingMessage { .. }
                        | StreamChunk::Text { .. }
                        | StreamChunk::Thinking { .. }
                        | StreamChunk::ToolCall { .. }
                        | StreamChunk::ToolResult { .. }
                )
            })
            .collect::<Vec<_>>(),
    )
}

pub async fn compact(manager: &SessionManager, id: &SessionId) -> TestResult {
    let session = manager.get_session(&id.value)?;
    let manifest = load_session(session.id)?;
    let inner = session.inner.lock().await;
    let source = inner.capture_durable_compaction(session.id)?;
    let model = inner.current_model_id().ok_or("model")?;
    let generation = source.generation + 1;
    let payload = serde_json::to_value(
        inner
            .provider_session
            .prepare_generation(generation, &model)?,
    )?;
    let count = manifest.messages.len();
    let dag = validate_dag_candidate(
        &format!("<dag-node depth=\"D1\" turns=\"0-{}\" label=\"Archive\">reduced committed work {generation}</dag-node>\n<dag-files>\n</dag-files>", count - 1),
        &DagValidationContext {
            persisted_turn_count: count,
            required_coverage_end: count.checked_sub(1),
            ..Default::default()
        },
    )?;
    let candidate = CompactionCandidate {
        operation_id: uuid::Uuid::new_v4(),
        checkpoint: CompactionCheckpoint {
            provider: manifest.provider.clone(),
            model: model.clone(),
            generation,
            payload,
        },
        source,
        dag,
        input_tokens: 23,
        output_reservation: 8,
        token_provenance: CompactionTokenProvenance::Estimated,
    };
    drop(inner);
    let promoted = commit_compaction(candidate, &model, |_, _| Ok(()))?;
    publish_compaction(&promoted, |_| Ok(()))?;
    Ok(())
}

pub async fn seed_compacted(manager: &SessionManager, id: &SessionId) -> TestResult {
    let mut manifest = load_session(uuid::Uuid::parse_str(&id.value)?)?;
    append(&mut manifest, "user", json!("covered original 雪"))?;
    compact(manager, id).await?;
    manifest = load_session(manifest.id)?;
    append(
        &mut manifest,
        "assistant",
        json!([
            {"type":"text","text":"original assistant"},
            {"type":"tool_use","id":"archive-call","name":"read_file","input":{"path":"original.txt","detail":"full original arguments"}}
        ]),
    )?;
    append(
        &mut manifest,
        "user",
        json!([
            {"type":"tool_result","tool_use_id":"archive-call","content":"complete original tool result 雪"}
        ]),
    )?;
    compact(manager, id).await?;
    manifest = load_session(manifest.id)?;
    append(&mut manifest, "user", json!("uncompacted suffix"))?;
    Ok(())
}

pub fn transcript(
    manager: &SessionManager,
    id: &SessionId,
) -> Result<Vec<StreamChunk>, Box<dyn std::error::Error>> {
    let session = manager.get_session(&id.value)?;
    let mut assembler = TranscriptAssembler::default();
    let mut cursor = None;
    let mut snapshot = None;
    let mut chunks = Vec::new();
    loop {
        let page = session.get_transcript_page(cursor, snapshot, 1024)?;
        assert!(serde_json::to_vec(&page)?.len() <= 1024);
        if let Some(chunk) = assembler.accept(&page)? {
            chunks.push(chunk);
        }
        cursor = Some(page.next_cursor.clone());
        snapshot = Some(page.snapshot_end);
        if page.complete {
            return Ok(chunks);
        }
    }
}
