//! Session Scanning Pipeline
//!
//! Reads sessions from the persistence layer, extracts structural entities
//! from tool call metadata in messages, loads into nanograph, updates watermarks.
//!
//! Extracted from indexing.rs to keep files under 300 lines.

use crate::graph;
use crate::graph::extractors::GraphEntity;
use crate::graph::llm_extraction::ConversationTurn;
use crate::graph::watermark::{read_index_state, update_session_watermark, write_index_state};
use crate::persistence;
use crate::session_search_handler::resolve_message_content;
use tracing::{info, warn};

use super::indexing::unindexed_turn_range;

/// Result of scanning and indexing all sessions.
#[derive(Debug)]
pub struct ScanResult {
    /// Number of sessions scanned.
    pub sessions_scanned: u32,
    /// Number of sessions skipped (fully indexed).
    pub sessions_skipped: u32,
    /// Total entities loaded into the graph.
    pub entities_loaded: u32,
}

/// Scan all sessions from the persistence layer and extract structural entities
/// into the graph database.
///
/// For each session:
/// 1. Read the watermark from index-state.json
/// 2. Skip if fully indexed
/// 3. Load messages via `get_session_messages_full()`
/// 4. For each unindexed message, look for tool call patterns in metadata
/// 5. Run structural extractors to produce graph entities
/// 6. If extraction_mode is "hybrid" or "llm_only", batch user/assistant turns
///    and call the LLM extraction pipeline for Concept/Decision/Relation entities
/// 7. Load entities into graph and update watermark
pub async fn scan_and_index_sessions(
    provider_name: Option<&str>,
    model_id: Option<&str>,
    extraction_mode: Option<&str>,
) -> Result<ScanResult, String> {
    let project_path = persistence::get_data_dir()?;
    let graph_dir = project_path.join("graph/agent-memory.nano");

    let sessions = persistence::list_all_sessions()?;
    let mut index_state = read_index_state(&graph_dir);

    let mut result = ScanResult {
        sessions_scanned: 0,
        sessions_skipped: 0,
        entities_loaded: 0,
    };

    for session in &sessions {
        let session_id = session.id.to_string();
        let total_turns = session.messages.len() as u32;

        // Check watermark
        let watermark = index_state
            .sessions
            .get(&session_id)
            .map(|w| w.last_indexed_turn)
            .unwrap_or(0);

        let range = match unindexed_turn_range(total_turns, watermark) {
            Some(r) => r,
            None => {
                result.sessions_skipped += 1;
                continue;
            }
        };

        // Load all messages for this session
        let messages = match persistence::get_session_messages_full(session) {
            Ok(msgs) => msgs,
            Err(e) => {
                warn!("Failed to load messages for session {session_id}: {e}");
                continue;
            }
        };

        let mut session_entities: Vec<GraphEntity> = Vec::new();
        let mode = extraction_mode.unwrap_or("hybrid");
        let run_structural = mode == "hybrid" || mode == "structural";
        let run_llm = (mode == "hybrid" || mode == "llm_only") && provider_name.is_some();

        // --- Phase 1: Structural extraction (zero-cost, pattern matching) ---
        let start_idx = range.0.saturating_sub(1) as usize; // 1-based to 0-based
        if run_structural {
            for (idx, msg) in messages.iter().enumerate().skip(start_idx) {
                let turn_index = idx as u32;
                let content = resolve_message_content(msg);

                // Extract entities from assistant messages that contain tool call patterns
                if msg.role == "assistant" {
                    // Check metadata for structural annotations
                    extract_from_annotations(msg, &session_id, turn_index, &mut session_entities);

                    // Fallback: scan message content for tool call patterns
                    extract_entities_from_content(
                        &content,
                        &session_id,
                        turn_index,
                        &mut session_entities,
                    );
                }
            }
        }

        // --- Phase 2: LLM extraction (Concepts, Decisions, Relations) ---
        let mut llm_failed = false;
        if run_llm {
            // Build ConversationTurn structs from ALL unindexed messages
            let conversation_turns: Vec<ConversationTurn> = messages
                .iter()
                .enumerate()
                .skip(start_idx)
                .map(|(idx, msg)| {
                    let content = resolve_message_content(msg);
                    ConversationTurn {
                        role: msg.role.clone(),
                        content,
                        turn_index: idx as u32,
                    }
                })
                .collect();

            if !conversation_turns.is_empty() {
                let batch_size = 10_u32; // Default from IndexingConfig
                let llm_result = graph::llm_caller::extract_from_session_turns(
                    &conversation_turns,
                    &session_id,
                    batch_size,
                    provider_name.unwrap_or("claude"),
                    model_id,
                ).await;

                if llm_result.failed_batches > 0 {
                    llm_failed = true;
                    warn!(
                        session = %session_id,
                        failed = llm_result.failed_batches,
                        total = llm_result.batch_count,
                        "LLM extraction had failures — watermark will not be updated"
                    );
                }

                if !llm_result.entities.is_empty() {
                    info!(
                        session = %session_id,
                        concepts = llm_result.entities.iter()
                            .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Concept"))
                            .count(),
                        decisions = llm_result.entities.iter()
                            .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Decision"))
                            .count(),
                        relations = llm_result.entities.iter()
                            .filter(|e| matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "RelatesTo"))
                            .count(),
                        "LLM extraction produced entities"
                    );
                    session_entities.extend(llm_result.entities);
                }
            }
        }

        // Load entities into graph — deduplicate by slug to avoid @key violations
        if !session_entities.is_empty() {
            // Deduplicate: keep the last entity for each (type, slug) pair
            let mut seen = std::collections::HashSet::new();
            let mut deduped = Vec::new();
            for entity in session_entities.into_iter().rev() {
                let key = match &entity {
                    GraphEntity::Node { node_type, slug, .. } => {
                        format!("node:{}:{}", node_type, slug)
                    }
                    GraphEntity::Edge { edge_type, from_slug, to_slug, .. } => {
                        format!("edge:{}:{}:{}", edge_type, from_slug, to_slug)
                    }
                };
                if seen.insert(key) {
                    deduped.push(entity);
                }
            }
            deduped.reverse();

            let count = deduped.len();
            let jsonl = graph::merge::entities_to_jsonl(&deduped);
            // Load per line — individual entity failures shouldn't block the session
            for line in jsonl.lines() {
                if !line.trim().is_empty() {
                    if let Err(e) = graph::graph_db_load_jsonl(line).await {
                        warn!("Failed to load entity for session {session_id}: {e}");
                    }
                }
            }
            result.entities_loaded += count as u32;
            info!(
                session = %session_id,
                entities = count,
                "indexed session entities into graph"
            );
        }

        // Update watermark — only if ALL extraction completed successfully
        // If LLM extraction had failures, don't update so the session is retried
        if !llm_failed {
            let now = chrono::Utc::now().to_rfc3339();
            update_session_watermark(&mut index_state, &session_id, total_turns, &now);
        }
        result.sessions_scanned += 1;
    }

    // Persist updated watermarks
    if result.sessions_scanned > 0 {
        write_index_state(&graph_dir, &index_state)?;
    }

    Ok(result)
}

/// Extract entities from structural annotations in message metadata.
fn extract_from_annotations(
    msg: &persistence::StoredMessage,
    session_id: &str,
    turn_index: u32,
    entities: &mut Vec<GraphEntity>,
) {
    let annotations = match msg.metadata.get("annotations") {
        Some(a) => a,
        None => return,
    };
    let arr = match annotations.as_array() {
        Some(a) => a,
        None => return,
    };

    for annotation in arr {
        // FileModification annotations
        if let Some(fm) = annotation.get("FileModification") {
            if let (Some(path), Some(op)) = (
                fm.get("path").and_then(|v| v.as_str()),
                fm.get("operation").and_then(|v| v.as_str()),
            ) {
                let tool_name = match op {
                    "Created" => "Write",
                    "Modified" => "Edit",
                    _ => continue,
                };
                entities.extend(graph::extractors::extract_from_file_operation(
                    tool_name, path, session_id, turn_index,
                ));
            }
        }
        // FspecMilestone annotations
        if let Some(fm) = annotation.get("FspecMilestone") {
            if let Some(command) = fm.get("command").and_then(|v| v.as_str()) {
                let args = fm
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let work_unit_id = args.first().copied().unwrap_or("");
                let title = args.get(1).copied().unwrap_or("");
                entities.extend(graph::extractors::extract_from_fspec_command(
                    command,
                    work_unit_id,
                    title,
                    session_id,
                ));
            }
        }
    }
}

/// Extract entities from message content by scanning for tool call patterns.
///
/// Fallback for sessions without structural annotations. Looks for
/// "Successfully wrote to" / "Successfully edited" patterns in tool results.
fn extract_entities_from_content(
    content: &str,
    session_id: &str,
    turn_index: u32,
    entities: &mut Vec<GraphEntity>,
) {
    for line in content.lines() {
        if let Some(path) = line.strip_prefix("Successfully wrote to ") {
            let path = path.trim();
            if !path.is_empty() {
                entities.extend(graph::extractors::extract_from_file_operation(
                    "Write", path, session_id, turn_index,
                ));
            }
        } else if let Some(path) = line.strip_prefix("Successfully edited ") {
            let path = path.trim();
            if !path.is_empty() {
                entities.extend(graph::extractors::extract_from_file_operation(
                    "Edit", path, session_id, turn_index,
                ));
            }
        }
    }
}
