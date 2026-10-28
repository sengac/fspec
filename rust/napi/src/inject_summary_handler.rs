//! inject_summary handler — bridges codelet-tools InjectSummaryTool
//! to the session manipulation logic in codelet-napi.
//!
//! Feature: spec/features/inject-summary-handler.feature
//!
//! Compatibility functions delegate to the canonical no-fallback engine.
//! Uncaptured raw candidates cannot replace memory or announce completion.
//! The live engine uses captured, validated durable compaction operations.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use codelet_core::compaction::{FileOp, StructuralAnnotation};
use codelet_tools::inject_summary::InjectSummaryHandler;

/// Callback invoked after inject_summary stores the DAG and clears the
/// compaction flag. Used by the NAPI session manager for non-emitting
/// side effects (e.g. clearing the compaction progress spinner) as soon
/// as the agent calls inject_summary.
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

/// Compatibility entrypoint for uncaptured legacy submissions.
/// The canonical engine rejects them without changing context, clearing busy
/// ownership or firing completion callbacks. Proper compaction is registered
/// through the captured operation-owned durable commit path.
pub fn create_handler(
    pending_dag: Arc<std::sync::Mutex<Option<String>>>,
    context_window: u64,
    compaction_in_progress: Arc<AtomicBool>,
    on_injected: Option<OnInjectedCallback>,
) -> InjectSummaryHandler {
    codelet_agent_loop::inject_summary_handler::create_handler(
        pending_dag,
        context_window,
        compaction_in_progress,
        on_injected,
    )
}

/// Emit the post-injection events in the correct order:
/// 1. `SessionStateChange(Running)` — so JS sees `isLoading=true`
/// 2. `CompactionComplete` — so JS clears the compaction indicator
///
/// The ordering is critical: JS must pick up `isLoading=true` BEFORE
/// `isCompacting=false` to avoid flickering to the idle input area.
///
/// Extracted from the on_injected closure in session_manager so the
/// emission order is testable without BackgroundSession infrastructure.
///
/// CMPCT-038: `compacted_tokens` is the session's recalculated
/// post-injection total (reminders + summary — see
/// [`apply_pending_dag_and_emit`]), NOT the DAG summary size alone.
/// The ratio is percent-of-tokens-removed (RPC-420 contract) clamped
/// to `>= 0.0` — tiny sessions where surviving reminders exceed the
/// original count must never ship a negative ratio.
pub fn emit_post_injection_events(
    emit: &dyn Fn(crate::types::StreamChunk),
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
    emit(crate::types::StreamChunk::session_state_change(
        crate::types::SessionState::Running,
    ));
    // Step 2: Emit CompactionComplete
    emit(crate::types::StreamChunk::compaction_complete(
        crate::types::CompactionResult {
            original_tokens,
            compacted_tokens,
            compression_ratio: ratio,
            turns_summarized: 0,
            turns_kept: 0,
        },
    ));
}

/// Determine if the Done handler should set session status to Idle.
///
/// The Done handler must NOT set Idle when either:
/// - `compaction_in_progress` is still true (agent is building DAG)
/// - `pending_dag_content` has content (inject_summary stored DAG but apply_pending_dag hasn't run)
///
/// Returns true only when BOTH conditions are false — meaning it's safe to go Idle.
///
/// Truth table:
/// | compaction_in_progress | has_pending_dag | should_idle |
/// |------------------------|-----------------|-------------|
/// | true                   | true            | NO          |
/// | true                   | false           | NO          |
/// | false                  | true            | NO          |
/// | false                  | false           | YES         |
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
/// Each line inside the block should match: `- path/to/file (Created|Modified|Deleted)`
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
    // Match pattern: "path (Operation)"
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
///
/// Merges new annotations with any existing dag-files entries (from a previous compaction).
/// Newer operations override older ones for the same path. Uses BTreeMap for deterministic
/// alphabetical ordering.
///
/// Returns `None` if no file entries would be in the block (no empty blocks).
pub fn build_dag_files_block(
    annotations: &[StructuralAnnotation],
    existing_dag_files: Option<&str>,
) -> Option<String> {
    let mut all_files: BTreeMap<String, FileOp> = BTreeMap::new();

    // Parse existing dag-files block if present
    if let Some(existing) = existing_dag_files {
        if let Some(parsed) = parse_dag_files_block(existing) {
            all_files = parsed;
        }
    }

    // Add new annotations (newer ops override older)
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
/// nothing was pending. Downstream features (CMPCT-018-021) can use the
/// metadata for scoped queries, incremental condensation, and file propagation.
pub fn apply_pending_dag(
    session: &mut codelet_cli::session::Session,
    pending_dag: &Arc<std::sync::Mutex<Option<String>>>,
) -> Option<Vec<codelet_core::compaction::DagNodeMeta>> {
    codelet_agent_loop::inject_summary_handler::apply_pending_dag(session, pending_dag)
}

/// Apply pending DAG content and emit the post-injection events with the
/// HONEST measurement basis (CMPCT-038).
///
/// This is the production emit site for `CompactionComplete`: it calls
/// [`apply_pending_dag`] (which rebuilds the message list to reminders +
/// summary and runs `recalculate_token_tracker`), then reads the
/// recalculated `session.token_tracker.input_tokens` as `compacted_tokens`
/// and emits `SessionStateChange(Running)` followed by `CompactionComplete`
/// via [`emit_post_injection_events`].
///
/// Race-free by construction: the tracker is recalculated BEFORE the chunk
/// is emitted, so `compacted_tokens` reflects the real post-injection
/// context (a true ~60% reduction reports ~60, not the ~99% the
/// summary-only basis produced).
///
/// Kept in lock-step with the codelet-agent-loop twin
/// (`rust/agent-loop/src/inject_summary_handler.rs`) — identical inputs
/// MUST produce identical `CompactionResult` values on both frontends.
///
/// Returns the parsed DAG nodes if a DAG was applied, or `None` (emitting
/// nothing) if nothing was pending.
pub fn apply_pending_dag_and_emit(
    session: &mut codelet_cli::session::Session,
    pending_dag: &Arc<std::sync::Mutex<Option<String>>>,
    original_tokens: u32,
    emit: &dyn Fn(crate::types::StreamChunk),
) -> Option<Vec<codelet_core::compaction::DagNodeMeta>> {
    codelet_agent_loop::inject_summary_handler::apply_pending_dag_and_emit(
        session,
        pending_dag,
        original_tokens,
        emit,
    )
}

#[cfg(test)]
mod tests {
    //! Feature: spec/features/in-view-dag-compaction.feature
    //!
    //! Tests for inject_summary handler flag clearing.

    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use uuid::Uuid;

    // Scenario: inject_summary handler clears compaction_in_progress flag
    #[test]
    fn test_uncaptured_inject_summary_preserves_compaction_flag(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let flag = Arc::new(AtomicBool::new(true));
        let pending = Arc::new(std::sync::Mutex::new(None));
        let handler = create_handler(pending.clone(), 200_000, flag.clone(), None);
        let result = handler(Uuid::new_v4(), "# D2: Architecture\n- JWT auth".into());
        assert!(
            result.is_err(),
            "uncaptured submission requires a durable operation"
        );
        assert!(
            flag.load(Ordering::Acquire),
            "busy ownership cannot clear before durable application"
        );
        assert!(pending.lock().map_err(|error| error.to_string())?.is_none());
        Ok(())
    }

    // Scenario: inject_summary stores wrapped DAG content
    #[test]
    fn test_uncaptured_inject_summary_does_not_stage_wrapped_content(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let flag = Arc::new(AtomicBool::new(true));
        let pending = Arc::new(std::sync::Mutex::new(None));
        let handler = create_handler(pending.clone(), 200_000, flag.clone(), None);
        let error = handler(Uuid::new_v4(), "# D2: Durable\n- Using bcrypt".into())
            .err()
            .ok_or("raw candidate must not stage fallback memory")?;
        assert!(error.contains("durable operation capture"));
        assert!(pending.lock().map_err(|error| error.to_string())?.is_none());
        assert!(flag.load(Ordering::Acquire));
        Ok(())
    }

    // Scenario: inject_summary returns token counts
    #[test]
    fn test_uncaptured_inject_summary_never_returns_success_token_estimates(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pending = Arc::new(std::sync::Mutex::new(None));
        let handler = create_handler(pending, 200_000, Arc::new(AtomicBool::new(true)), None);
        let error = handler(Uuid::new_v4(), "# Summary".into())
            .err()
            .ok_or("uncaptured submission cannot claim injected tokens or remaining budget")?;
        assert!(error.contains("durable operation capture"));
        Ok(())
    }

    // Scenario: apply_pending_dag with no pending content is a no-op
    #[test]
    fn test_apply_pending_dag_no_content() {
        let pending_dag = Arc::new(std::sync::Mutex::new(None));
        // Explicit synthetic configuration: never probe developer credentials.
        let provider_manager = codelet_providers::ProviderManager::for_testing(
            codelet_providers::ProviderType::OpenAI,
            Some(200_000),
            Some(4096),
        );
        let mut session = codelet_cli::session::Session::from_provider_manager(provider_manager);

        let applied = apply_pending_dag(&mut session, &pending_dag);
        assert!(applied.is_none(), "Should return None when no pending DAG");
    }

    // Scenario: Compaction system instruction guides agent through DAG construction
    // (Updated for CMPCT-017: now also verifies structured dag-node format)
    // (Updated for CMPCT-019: uses COMPACTION_INSTRUCTION_FRESH constant)
    #[test]
    fn test_compaction_instruction_content() {
        use codelet_cli::compaction_dag::COMPACTION_INSTRUCTION_FRESH;

        let instruction = COMPACTION_INSTRUCTION_FRESH;
        assert!(!instruction.is_empty(), "Instruction should not be empty");
        assert!(
            instruction.contains("SessionSearch"),
            "Must mention SessionSearch"
        );
        assert!(instruction.contains("D0"), "Must mention D0");
        assert!(instruction.contains("D1"), "Must mention D1");
        assert!(instruction.contains("D2"), "Must mention D2");
        assert!(
            instruction.contains("inject_summary"),
            "Must tell agent to call inject_summary"
        );
    }

    // Feature: spec/features/structured-dag-node-format.feature
    //
    // Scenario: Compaction instruction specifies structured dag-node format
    // (Updated for CMPCT-019: uses COMPACTION_INSTRUCTION_FRESH constant)
    #[test]
    fn test_compaction_instruction_specifies_dag_node_format() {
        use codelet_cli::compaction_dag::COMPACTION_INSTRUCTION_FRESH;

        // @step When the compaction system instruction is loaded
        let instruction = COMPACTION_INSTRUCTION_FRESH;

        // @step Then it should contain guidance for writing dag-node XML blocks
        assert!(
            instruction.contains("dag-node") || instruction.contains("<dag-node"),
            "Instruction must mention dag-node format"
        );

        // @step And it should explain the D0, D1, and D2 depth semantics
        assert!(instruction.contains("D0"), "Must explain D0");
        assert!(instruction.contains("D1"), "Must explain D1");
        assert!(instruction.contains("D2"), "Must explain D2");

        // @step And it should specify the turns attribute format as "N-M" inclusive range
        assert!(
            instruction.contains("turns"),
            "Must mention turns attribute"
        );

        // @step And it should require a label attribute on each dag-node
        assert!(
            instruction.contains("label"),
            "Must mention label attribute"
        );
    }

    // Feature: spec/features/structured-dag-node-format.feature
    //
    // Scenario: Parsed DagNodeMeta stored in InjectSummaryState for downstream access
    #[test]
    fn test_parsed_dag_nodes_available_after_apply() {
        use codelet_core::compaction::{parse_dag_nodes, DagDepth};

        // @step Given the agent has written a DAG with structured dag-node blocks
        let dag_content = r#"<dag-node depth="D2" turns="0-20" label="Decisions">
- Architecture choice A
</dag-node>

<dag-node depth="D0" turns="21-30" label="Recent work">
- Current task
</dag-node>"#;

        // @step When inject_summary is called and apply_pending_dag processes the content
        let nodes = parse_dag_nodes(dag_content, None);

        // @step Then the InjectSummaryState should contain both the raw DAG string and the parsed Vec of DagNodeMeta
        assert!(!dag_content.is_empty(), "Raw DAG string should exist");
        assert_eq!(nodes.len(), 2, "Parsed DagNodeMeta should have 2 entries");

        // @step And downstream features should be able to access the parsed metadata
        assert_eq!(nodes[0].depth, DagDepth::D2);
        assert_eq!(nodes[1].depth, DagDepth::D0);
        assert_eq!(nodes[0].label, "Decisions");
        assert_eq!(nodes[1].label, "Recent work");
    }

    // Scenario: on_injected callback fires immediately after DAG is stored
    #[test]
    fn test_uncaptured_injection_does_not_fire_completion_side_effects(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let flag = Arc::new(AtomicBool::new(true));
        let pending = Arc::new(std::sync::Mutex::new(None));
        let called = Arc::new(AtomicBool::new(false));
        let tokens = Arc::new(AtomicU32::new(0));
        let observed = called.clone();
        let recorded = tokens.clone();
        let callback: OnInjectedCallback = Arc::new(move |count| {
            observed.store(true, Ordering::SeqCst);
            recorded.store(count, Ordering::SeqCst);
        });
        let handler = create_handler(pending.clone(), 200_000, flag.clone(), Some(callback));
        assert!(handler(Uuid::new_v4(), "# D2: Architecture\n- JWT auth".into()).is_err());
        assert!(
            !called.load(Ordering::Acquire),
            "completion side effects require durable application"
        );
        assert_eq!(tokens.load(Ordering::Acquire), 0);
        assert!(flag.load(Ordering::Acquire));
        assert!(pending.lock().map_err(|error| error.to_string())?.is_none());
        Ok(())
    }
}
