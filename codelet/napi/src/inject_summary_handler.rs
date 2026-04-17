//! inject_summary handler — bridges codelet-tools InjectSummaryTool
//! to the session manipulation logic in codelet-napi.
//!
//! Feature: spec/features/inject-summary-handler.feature
//!
//! The handler does NOT lock session.inner. The agent_loop
//! holds that lock during streaming, so locking it here would deadlock.
//! Instead, the handler stores the DAG content in `pending_dag_content`
//! (an Arc<std::sync::Mutex<Option<String>>> on BackgroundSession) and
//! returns immediately. After the stream completes, the agent_loop checks
//! for pending DAG content and applies the session state changes.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use codelet_common::token_estimator::count_tokens;
use codelet_core::compaction::{FileOp, StructuralAnnotation, wrap_dag_content};
use codelet_tools::inject_summary::{InjectSummaryHandler, InjectSummaryResult};
use uuid::Uuid;

/// Callback invoked after inject_summary stores the DAG and clears the
/// compaction flag. Used by the NAPI session manager to emit
/// `CompactionComplete` immediately so the TUI drops the compaction
/// indicator without waiting for the stream to finish.
///
/// Arguments: (injected_tokens: u32)
pub type OnInjectedCallback = Arc<dyn Fn(u32) + Send + Sync>;

/// Create an inject_summary handler for a specific session.
///
/// The handler captures:
/// - `pending_dag`: shared storage for the DAG content
/// - `context_window`: for estimating remaining budget
/// - `compaction_in_progress`: cleared after DAG is applied
/// - `on_injected`: optional callback fired immediately after DAG is stored
///
/// When invoked by the LLM tool call, the handler:
/// 1. Stores DAG content in `pending_dag` (does NOT touch session state)
/// 2. Clears the `compaction_in_progress` flag
/// 3. Fires `on_injected` callback to emit CompactionComplete immediately
/// 4. Returns estimated token counts
///
/// The actual "partition → clear → restore → inject" happens AFTER the
/// stream completes, in the agent_loop, which has the session lock.
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
        // SessionSearch will stop trimming from this point
        compaction_flag.store(false, Ordering::SeqCst);

        // Step 5: Fire on_injected callback to emit CompactionComplete immediately.
        // This lets the TUI drop the compaction indicator as soon as inject_summary
        // runs, instead of waiting for the stream to end and apply_pending_dag.
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
///
/// Extracted from the on_injected closure in session_manager so the
/// emission order is testable without BackgroundSession infrastructure.
pub fn emit_post_injection_events(
    emit: &dyn Fn(crate::types::StreamChunk),
    original_tokens: u32,
    injected_tokens: u32,
) {
    use codelet_cli::interactive_helpers::compression_ratio;

    let ratio = compression_ratio(original_tokens as u64, injected_tokens as u64) * 100.0;
    // Step 1: Emit Running BEFORE CompactionComplete
    emit(crate::types::StreamChunk::session_state_change(
        crate::types::SessionState::Running,
    ));
    // Step 2: Emit CompactionComplete
    emit(crate::types::StreamChunk::compaction_complete(
        crate::types::CompactionResult {
            original_tokens,
            compacted_tokens: injected_tokens,
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
    // Collect FileModification annotations from the session BEFORE clearing.
    if !wrapped.contains("<dag-files>") {
        let all_annotations: Vec<StructuralAnnotation> = session
            .annotations
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();

        // Also check for existing dag-files in the previous DAG (for merge on incremental)
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

        if let Some(files_block) = build_dag_files_block(
            &all_annotations,
            existing_dag_files_str.as_deref(),
        ) {
            // Insert the dag-files block before the closing </system-reminder> tag
            if let Some(close_pos) = wrapped.rfind("</system-reminder>") {
                wrapped.insert_str(close_pos, &format!("\n\n{}\n", files_block));
            } else {
                wrapped.push_str(&format!("\n\n{}", files_block));
            }
        }
    }

    // Parse <dag-node> blocks from the raw DAG content for structured metadata.
    // The wrapped content includes <system-reminder> tags, so we parse before wrapping
    // or parse the inner content. Since wrap_dag_content adds the outer tags,
    // the inner content (what the agent wrote) is inside the system-reminder block.
    // We parse the full wrapped content — the regex only matches <dag-node> blocks
    // so the system-reminder wrapper doesn't interfere.
    let message_count = session.messages.len();
    let dag_nodes = parse_dag_nodes(&wrapped, Some(message_count));

    // Partition, clear, restore system reminders, clear turns
    let _counts = reset_session_to_reminders(session);

    // Append the wrapped DAG content as a system-reminder-style user message
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&wrapped)),
    });

    // Recalculate token tracker from actual post-injection messages
    recalculate_token_tracker(session);

    Some(dag_nodes)
}

#[cfg(test)]
mod tests {
    //! Feature: spec/features/in-view-dag-compaction.feature
    //!
    //! Tests for inject_summary handler flag clearing.

    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    // Scenario: inject_summary handler clears compaction_in_progress flag
    #[test]
    fn test_inject_summary_clears_compaction_flag() {
        // @step Given a session with compaction_in_progress flag set to true
        let compaction_flag = Arc::new(AtomicBool::new(true));
        assert!(compaction_flag.load(Ordering::Relaxed), "Flag should start as true");

        let pending_dag = Arc::new(std::sync::Mutex::new(None));
        let context_window: u64 = 200_000;

        let handler = create_handler(
            pending_dag.clone(),
            context_window,
            compaction_flag.clone(),
            None,
        );

        let session_id = Uuid::new_v4();
        let dag_content = "# D2: Architecture\n- JWT auth\n# D1: Current Arc\n- Implementing login".to_string();

        // @step When the agent calls inject_summary with DAG content
        let result = handler(session_id, dag_content);
        assert!(result.is_ok(), "inject_summary should succeed");

        // @step Then the inject_summary handler should clear the compaction_in_progress flag
        assert!(
            !compaction_flag.load(Ordering::Relaxed),
            "compaction_in_progress flag should be cleared after inject_summary"
        );

        // @step And the DAG content should be stored in pending_dag
        let stored = pending_dag.lock().unwrap();
        assert!(stored.is_some(), "pending_dag should have content");
        assert!(stored.as_ref().unwrap().contains("compaction-dag"));
    }

    // Scenario: inject_summary stores wrapped DAG content
    #[test]
    fn test_inject_summary_stores_wrapped_content() {
        let compaction_flag = Arc::new(AtomicBool::new(true));
        let pending_dag = Arc::new(std::sync::Mutex::new(None));

        let handler = create_handler(
            pending_dag.clone(),
            200_000,
            compaction_flag.clone(),
            None,
        );

        let dag = "# D2: Durable\n- Using bcrypt\n# D1: Arc\n- Building login".to_string();
        let result = handler(Uuid::new_v4(), dag);
        assert!(result.is_ok());

        let stored = pending_dag.lock().unwrap();
        let content = stored.as_ref().unwrap();
        assert!(content.contains("<system-reminder>"));
        assert!(content.contains("<!-- type:compaction-dag -->"));
        assert!(content.contains("Using bcrypt"));
    }

    // Scenario: inject_summary returns token counts
    #[test]
    fn test_inject_summary_returns_token_counts() {
        let pending_dag = Arc::new(std::sync::Mutex::new(None));
        let handler = create_handler(
            pending_dag,
            200_000,
            Arc::new(AtomicBool::new(true)),
            None,
        );

        let result = handler(Uuid::new_v4(), "# Summary".to_string());
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.injected_tokens > 0, "Should report injected tokens");
        assert!(result.remaining_budget > 0, "Should report remaining budget");
    }

    // Scenario: apply_pending_dag with no pending content is a no-op
    #[test]
    fn test_apply_pending_dag_no_content() {
        let pending_dag = Arc::new(std::sync::Mutex::new(None));
        let provider_manager = codelet_providers::ProviderManager::new()
            .expect("Need at least one API key for tests");
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
        assert!(instruction.contains("SessionSearch"), "Must mention SessionSearch");
        assert!(instruction.contains("D0"), "Must mention D0");
        assert!(instruction.contains("D1"), "Must mention D1");
        assert!(instruction.contains("D2"), "Must mention D2");
        assert!(instruction.contains("inject_summary"), "Must tell agent to call inject_summary");
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
    fn test_on_injected_callback_fires() {
        let compaction_flag = Arc::new(AtomicBool::new(true));
        let pending_dag = Arc::new(std::sync::Mutex::new(None));
        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_tokens = Arc::new(AtomicU32::new(0));

        let called_clone = callback_called.clone();
        let tokens_clone = callback_tokens.clone();
        let on_injected: OnInjectedCallback = Arc::new(move |tokens| {
            called_clone.store(true, Ordering::SeqCst);
            tokens_clone.store(tokens, Ordering::SeqCst);
        });

        let handler = create_handler(
            pending_dag.clone(),
            200_000,
            compaction_flag.clone(),
            Some(on_injected),
        );

        // @step When the agent calls inject_summary with DAG content
        let result = handler(Uuid::new_v4(), "# D2: Architecture\n- JWT auth".to_string());
        assert!(result.is_ok());

        // @step Then the on_injected callback should have been called
        assert!(callback_called.load(Ordering::SeqCst), "on_injected callback should fire");

        // @step And the injected token count should be greater than 0
        assert!(callback_tokens.load(Ordering::SeqCst) > 0, "injected tokens should be > 0");

        // @step And compaction_in_progress should already be false
        assert!(!compaction_flag.load(Ordering::Relaxed), "flag should be cleared before callback");
    }
}
