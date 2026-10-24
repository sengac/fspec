//! inject_summary handler (RPC-072 lift from
//! `rust/napi/src/inject_summary_handler.rs`).
//!
//! Bridges codelet-tools InjectSummaryTool to the session manipulation
//! logic that previously lived napi-side. Now lifted into the
//! NAPI-free agent-loop crate so the canonical `agent_loop` body can
//! call it directly.
//!
//! Feature: spec/features/inject-summary-handler.feature
//!
//! Uncaptured legacy submissions fail closed. The production registration in
//! `generate_compaction_handler` queues candidates through `TurnCompaction`,
//! whose application commits and publishes the captured durable generation.
//! Legacy pending-DAG application cannot replace context or announce completion.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use codelet_core::compaction::{FileOp, StructuralAnnotation};
use codelet_rpc_types::{CompactionResult, SessionState, StreamChunk};
use codelet_tools::inject_summary::InjectSummaryHandler;
use uuid::Uuid;

/// Callback invoked after inject_summary stores the DAG and clears the
/// compaction flag. Used by the session manager / agent_loop for
/// non-emitting side effects (e.g. clearing the compaction progress
/// spinner via `set_compaction_progress(None)`) as soon as the agent
/// calls inject_summary.
///
/// CMPCT-038: this callback MUST NOT emit `CompactionComplete` — at this
/// point only the DAG summary size is known, not the real post-injection
/// context size. The honest chunk is emitted later by
/// [`apply_pending_dag_and_emit`] once the token tracker has been
/// recalculated from the rebuilt message list (reminders + summary).
///
/// Arguments: (injected_tokens: u32) — the token count of the wrapped
/// DAG summary alone, useful for logging only.
pub type OnInjectedCallback = Arc<dyn Fn(u32) + Send + Sync>;

/// Create a legacy handler that rejects submissions without durable capture.
///
/// Production callers register an operation-owned `TurnCompaction` instead.
pub fn create_handler(
    _pending_dag: Arc<std::sync::Mutex<Option<String>>>,
    _context_window: u64,
    _compaction_in_progress: Arc<AtomicBool>,
    _on_injected: Option<OnInjectedCallback>,
) -> InjectSummaryHandler {
    Arc::new(move |_session_id: Uuid, _content: String| {
        Err("durable operation capture required; raw pending DAG submission is unavailable".into())
    })
}

/// Emit the post-injection events in the correct order:
/// 1. `SessionStateChange(Running)` — so JS sees `isLoading=true`
/// 2. `CompactionComplete` — so JS clears the compaction indicator
///
/// The ordering is critical: JS must pick up `isLoading=true` BEFORE
/// `isCompacting=false` to avoid flickering to the idle input area.
///
/// CMPCT-038: `compacted_tokens` is the session's recalculated
/// post-injection total (reminders + summary — see
/// [`apply_pending_dag_and_emit`]), NOT the DAG summary size alone.
/// The ratio is percent-of-tokens-removed (RPC-420 contract) clamped
/// to `>= 0.0` — tiny sessions where surviving reminders exceed the
/// original count must never ship a negative ratio.
pub fn emit_post_injection_events(
    emit: &dyn Fn(StreamChunk),
    original_tokens: u32,
    compacted_tokens: u32,
) {
    use codelet_cli::interactive_helpers::compression_ratio;

    // CMPCT-039: compression_ratio() itself now clamps to [0.0, 1.0], so the
    // .max(0.0) here is redundant — kept deliberately as belt-and-braces to
    // pin the CMPCT-038 wire contract at this emit site regardless of any
    // future helper changes.
    let ratio =
        (compression_ratio(original_tokens as u64, compacted_tokens as u64) * 100.0).max(0.0);
    // Step 1: Emit Running BEFORE CompactionComplete
    emit(StreamChunk::session_state_change(SessionState::Running));
    // Step 2: Emit CompactionComplete
    emit(StreamChunk::compaction_complete(CompactionResult {
        original_tokens,
        compacted_tokens,
        compression_ratio: ratio,
        turns_summarized: 0,
        turns_kept: 0,
    }));
}

/// Determine if the Done handler should set session status to Idle.
///
/// The Done handler must NOT set Idle when either:
/// - `compaction_in_progress` is still true (agent is building DAG)
/// - `pending_dag_content` has content (inject_summary stored DAG but apply_pending_dag hasn't run)
pub fn should_idle_on_done(
    compaction_in_progress: &AtomicBool,
    pending_dag_content: &std::sync::Mutex<Option<String>>,
) -> bool {
    let has_pending_dag = pending_dag_content
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false);
    let compaction_active = compaction_in_progress.load(Ordering::Acquire);
    !compaction_active && !has_pending_dag
}

/// Parse a `<dag-files>` block from DAG content, extracting (path, operation) entries.
///
/// Returns `Some(BTreeMap)` if a valid `<dag-files>` block is found, `None` otherwise.
pub fn parse_dag_files_block(dag_content: &str) -> Option<BTreeMap<String, FileOp>> {
    let start_tag = "<dag-files>";
    let end_tag = "</dag-files>";

    let start = dag_content.find(start_tag)?;
    let end = dag_content.find(end_tag)?;

    if end <= start {
        return None;
    }

    let inner = &dag_content[start + start_tag.len()..end];
    let mut entries = BTreeMap::new();

    for line in inner.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("- ") {
            if let Some((path, op)) = parse_dag_file_entry(rest) {
                entries.insert(path, op);
            }
        }
    }

    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

/// Parse a single dag-file entry like `path/to/file (Created)` into (path, FileOp).
fn parse_dag_file_entry(entry: &str) -> Option<(String, FileOp)> {
    let entry = entry.trim();
    if let Some(paren_start) = entry.rfind('(') {
        if let Some(paren_end) = entry.rfind(')') {
            if paren_end > paren_start {
                let path = entry[..paren_start].trim().to_string();
                let op_str = entry[paren_start + 1..paren_end].trim();
                let op = match op_str {
                    "Created" => FileOp::Created,
                    "Modified" => FileOp::Modified,
                    "Deleted" => FileOp::Deleted,
                    _ => return None,
                };
                if !path.is_empty() {
                    return Some((path, op));
                }
            }
        }
    }
    None
}

/// Build a `<dag-files>` block from FileModification annotations and existing dag-files.
pub fn build_dag_files_block(
    annotations: &[StructuralAnnotation],
    existing_dag_files: Option<&str>,
) -> Option<String> {
    let mut all_files: BTreeMap<String, FileOp> = BTreeMap::new();

    if let Some(existing) = existing_dag_files {
        if let Some(parsed) = parse_dag_files_block(existing) {
            all_files = parsed;
        }
    }

    for annotation in annotations {
        if let StructuralAnnotation::FileModification { path, operation } = annotation {
            all_files.insert(path.clone(), operation.clone());
        }
    }

    if all_files.is_empty() {
        return None;
    }

    let mut block = String::from("<dag-files>\n");
    for (path, op) in &all_files {
        let op_str = match op {
            FileOp::Created => "Created",
            FileOp::Modified => "Modified",
            FileOp::Deleted => "Deleted",
        };
        block.push_str(&format!("- {} ({})\n", path, op_str));
    }
    block.push_str("</dag-files>");

    Some(block)
}

/// Reject and consume uncaptured legacy pending DAG content.
///
/// A pending candidate or poisoned lock blocks inference without replacing
/// messages or counters. This compatibility entrypoint always returns `None`.
pub fn apply_pending_dag(
    session: &mut codelet_cli::session::Session,
    pending_dag: &Arc<std::sync::Mutex<Option<String>>>,
) -> Option<Vec<codelet_core::compaction::DagNodeMeta>> {
    match pending_dag.lock() {
        Ok(mut pending) => {
            if pending.take().is_some() {
                session.block_compaction_inference("uncaptured pending DAG rejected".into());
            }
        }
        Err(error) => {
            session.block_compaction_inference(format!("pending DAG lock unavailable: {error}"))
        }
    }
    None
}

/// Preserve the legacy application signature without publishing raw candidates.
///
/// [`apply_pending_dag`] always returns `None`, so no event is emitted here.
/// Production completion follows durable commit through `TurnCompaction::apply`.
pub fn apply_pending_dag_and_emit(
    session: &mut codelet_cli::session::Session,
    pending_dag: &Arc<std::sync::Mutex<Option<String>>>,
    original_tokens: u32,
    emit: &dyn Fn(StreamChunk),
) -> Option<Vec<codelet_core::compaction::DagNodeMeta>> {
    let dag_nodes = apply_pending_dag(session, pending_dag)?;

    let compacted_tokens = u32::try_from(session.token_tracker.input_tokens).unwrap_or(u32::MAX);
    emit_post_injection_events(emit, original_tokens, compacted_tokens);

    Some(dag_nodes)
}
