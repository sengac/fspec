//! inject_summary handler (RPC-072 lift from
//! `codelet/napi/src/inject_summary_handler.rs`).
//!
//! Bridges codelet-tools InjectSummaryTool to the session manipulation
//! logic that previously lived napi-side. Now lifted into the
//! NAPI-free agent-loop crate so the canonical `agent_loop` body can
//! call it directly.
//!
//! Feature: spec/features/inject-summary-handler.feature
//!
//! The handler does NOT lock session.inner. The agent_loop holds that
//! lock during streaming, so locking it here would deadlock. Instead,
//! the handler stores the DAG content in `pending_dag_content` (an
//! `Arc<std::sync::Mutex<Option<String>>>` on BackgroundSession) and
//! returns immediately. After the stream completes, the agent_loop
//! checks for pending DAG content and applies the session state
//! changes via [`apply_pending_dag`].

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use codelet_common::token_estimator::count_tokens;
use codelet_core::compaction::{wrap_dag_content, FileOp, StructuralAnnotation};
use codelet_rpc_types::{CompactionResult, SessionState, StreamChunk};
use codelet_tools::inject_summary::{InjectSummaryHandler, InjectSummaryResult};
use uuid::Uuid;

/// Callback invoked after inject_summary stores the DAG and clears the
/// compaction flag. Used by the session manager / agent_loop to emit
/// `CompactionComplete` immediately so the TUI drops the compaction
/// indicator without waiting for the stream to finish.
///
/// Arguments: (injected_tokens: u32)
pub type OnInjectedCallback = Arc<dyn Fn(u32) + Send + Sync>;

/// Create an inject_summary handler for a specific session.
///
/// Lifted verbatim from `codelet/napi/src/inject_summary_handler.rs:46-86`.
pub fn create_handler(
    pending_dag: Arc<std::sync::Mutex<Option<String>>>,
    context_window: u64,
    compaction_in_progress: Arc<AtomicBool>,
    on_injected: Option<OnInjectedCallback>,
) -> InjectSummaryHandler {
    Arc::new(move |_session_id: Uuid, content: String| {
        let compaction_flag = compaction_in_progress.clone();

        // Step 1: Wrap DAG content
        let wrapped = wrap_dag_content(&content);

        // Step 2: Estimate token counts (no session lock needed)
        let injected_tokens = count_tokens(&wrapped) as u64;
        // Rough estimate — actual budget recalculated when agent_loop applies the DAG
        let remaining_budget = context_window.saturating_sub(injected_tokens);

        // Step 3: Store DAG content for deferred application by agent_loop
        if let Ok(mut guard) = pending_dag.lock() {
            *guard = Some(wrapped);
        } else {
            return Err("Failed to acquire pending_dag lock".to_string());
        }

        // Step 4: Clear compaction_in_progress flag
        compaction_flag.store(false, Ordering::SeqCst);

        // Step 5: Fire on_injected callback to emit CompactionComplete immediately.
        if let Some(ref cb) = on_injected {
            cb(injected_tokens as u32);
        }

        Ok(InjectSummaryResult {
            injected_tokens,
            remaining_budget,
        })
    })
}

/// Emit the post-injection events in the correct order:
/// 1. `SessionStateChange(Running)` — so JS sees `isLoading=true`
/// 2. `CompactionComplete` — so JS clears the compaction indicator
///
/// The ordering is critical: JS must pick up `isLoading=true` BEFORE
/// `isCompacting=false` to avoid flickering to the idle input area.
pub fn emit_post_injection_events(
    emit: &dyn Fn(StreamChunk),
    original_tokens: u32,
    injected_tokens: u32,
) {
    use codelet_cli::interactive_helpers::compression_ratio;

    let ratio = compression_ratio(original_tokens as u64, injected_tokens as u64) * 100.0;
    // Step 1: Emit Running BEFORE CompactionComplete
    emit(StreamChunk::session_state_change(SessionState::Running));
    // Step 2: Emit CompactionComplete
    emit(StreamChunk::compaction_complete(CompactionResult {
        original_tokens,
        compacted_tokens: injected_tokens,
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

/// Apply pending DAG content to a session.
///
/// Called by the agent_loop AFTER the stream completes, while it still holds
/// the session lock. This is where the actual "clear → restore → inject" happens.
///
/// Returns the parsed `Vec<DagNodeMeta>` if a DAG was applied, or `None` if
/// nothing was pending.
pub fn apply_pending_dag(
    session: &mut codelet_cli::session::Session,
    pending_dag: &Arc<std::sync::Mutex<Option<String>>>,
) -> Option<Vec<codelet_core::compaction::DagNodeMeta>> {
    use codelet_cli::interactive_helpers::{recalculate_token_tracker, reset_session_to_reminders};
    use codelet_core::compaction::parse_dag_nodes;
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;

    // Take the pending DAG content (if any)
    let dag_content = match pending_dag.lock() {
        Ok(mut guard) => guard.take(),
        Err(_) => None,
    };

    let mut wrapped = dag_content?;

    // CMPCT-021: Auto-append <dag-files> block if agent omitted it.
    if !wrapped.contains("<dag-files>") {
        let all_annotations: Vec<StructuralAnnotation> = session
            .annotations
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();

        let existing_dag_files_str = codelet_cli::compaction_dag::detect_existing_dag(
            &session.messages,
        )
        .and_then(|(content, _)| {
            if content.contains("<dag-files>") {
                Some(content)
            } else {
                None
            }
        });

        if let Some(files_block) =
            build_dag_files_block(&all_annotations, existing_dag_files_str.as_deref())
        {
            if let Some(close_pos) = wrapped.rfind("</system-reminder>") {
                wrapped.insert_str(close_pos, &format!("\n\n{}\n", files_block));
            } else {
                wrapped.push_str(&format!("\n\n{}", files_block));
            }
        }
    }

    let message_count = session.messages.len();
    let dag_nodes = parse_dag_nodes(&wrapped, Some(message_count));

    let _counts = reset_session_to_reminders(session);

    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&wrapped)),
    });

    recalculate_token_tracker(session);

    Some(dag_nodes)
}
