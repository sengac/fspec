//! DAG compaction instructions, detection, extraction, and fallback injection.
//!
//! Extracted from interactive_helpers.rs (SRP refactoring for CMPCT-016 review).
//! Contains:
//! - Compaction instruction constants (FRESH, INCREMENTAL, ESCALATION)
//! - Existing DAG detection in session messages
//! - Partial dag-node extraction from assistant messages
//! - Force-inject fallback DAG (Level 3 convergence guarantee)

use crate::interactive_helpers::{collect_items, recalculate_token_tracker, reset_session_to_reminders};
use crate::session::Session;
use codelet_core::compaction::{parse_dag_nodes, wrap_dag_content};
use once_cell::sync::Lazy;
use regex::Regex;
use rig::message::{Message, UserContent};
use rig::OneOrMany;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::debug;

// ============================================================================
// Instruction Constants
// ============================================================================

/// Fresh compaction instruction — used when no existing DAG is in context.
///
/// This is the first-time compaction path. The agent builds a complete DAG
/// from scratch using SessionSearch to survey the entire session history.
///
/// Research: ACON (Kang et al., KAIST/Microsoft, arXiv:2510.00615) —
/// compression guidelines embedded in system instructions yield 26-54%
/// peak token reduction while maintaining task performance.
pub const COMPACTION_INSTRUCTION_FRESH: &str = "\
Your context window was getting full. Your conversation history has been \
preserved on disk and is fully searchable via SessionSearch. Build a \
hierarchical summary DAG of your session:

1. Search strategically (not linearly):
   - SessionSearch(show, max_turns: 10) for recent context
   - SessionSearch(search, query: \"error|failed|fix\") for error resolutions
   - SessionSearch(search, query: \"decision|chose|architecture\") for decisions
   - SessionSearch(search, query: \"TODO|blocker|question\") for open items

2. Write a structured summary using <dag-node> blocks with three required attributes:
   - depth: D2 (Durable), D1 (Arc), or D0 (Detailed)
   - turns: \"N-M\" inclusive range of turn indices this node summarizes
   - label: short identifier for the node (max ~80 chars)

   Depth semantics:
   - D2 (Durable): Architecture decisions, milestones that survive multiple compactions
   - D1 (Arc): Current work state — what was attempted, outcomes, open issues
   - D0 (Detailed): Exact files, errors, decisions from the most recent work

   Example format:
   <dag-node depth=\"D2\" turns=\"0-45\" label=\"Architecture Decisions\">
   - JWT + Redis + bcrypt for auth
   - Using Vitest, not Jest (project standard)
   </dag-node>

   <dag-node depth=\"D1\" turns=\"46-82\" label=\"Auth Implementation Arc\">
   - Completed auth handler (turns 46-70)
   - Started rate limiting (turns 71-82)
   </dag-node>

   <dag-node depth=\"D0\" turns=\"83-95\" label=\"Current: rate-limit tests\">
   - Working on src/middleware/rateLimit.ts
   - 2 tests failing: counter increment race condition
   [SessionSearch: turns 88-92]
   </dag-node>

   Turn ranges should be non-overlapping and collectively cover the session.
   Include [SessionSearch: turns X-Y] references inside nodes for future drilldown.

3. Include a <dag-files> section listing all files you modified during this session:
   <dag-files>
   - path/to/file.rs (Created)
   - path/to/other.rs (Modified)
   </dag-files>
   Operations: Created, Modified, or Deleted. This helps you retain file awareness across compactions.

4. Call inject_summary(content) with your complete DAG to pin it and \
continue working.";

/// Incremental compaction instruction — used when an existing DAG is in context.
///
/// Instead of rebuilding from scratch, the agent updates the existing DAG:
/// - Preserves D2 (Durable) nodes unchanged
/// - Reviews D1 (Arc) nodes, promoting solidified parts to D2
/// - Promotes D0 (Detailed) nodes to D1 (they're no longer the freshest)
/// - Creates new D0 nodes covering only turns since the last compaction
///
/// Template placeholders:
/// - `{existing_dag_content}`: The existing DAG content from the previous compaction
/// - `{last_compacted_turn}`: The max turn_end from the existing DAG's dag-node blocks
pub const COMPACTION_INSTRUCTION_INCREMENTAL: &str = "\
Your context window was getting full again. You have an existing DAG summary from a \
previous compaction. Update it incrementally — do NOT rebuild from scratch:

1. PRESERVE all existing D2 (Durable) nodes unchanged — these are settled decisions
2. REVIEW existing D1 (Arc) nodes — keep if still relevant, promote key parts to D2 if solidified
3. PROMOTE existing D0 (Detailed) nodes to D1 — they are no longer the freshest work
4. Search ONLY for turns since your last compaction:
   - SessionSearch(show, start_turn: {last_compacted_turn}, max_turns: 20)
   - SessionSearch(search, query: \"error|fix\", start_turn: {last_compacted_turn})
   - SessionSearch(search, query: \"decision|architecture\", start_turn: {last_compacted_turn})
5. Write NEW D0 nodes covering only the fresh turns
6. PRESERVE the <dag-files> section — update it with any new file modifications from fresh turns. \
Remove entries for files that were deleted. Add entries for newly created or modified files.

Your existing DAG is below — update it, don't rebuild from scratch:

{existing_dag_content}

Call inject_summary(content) with your complete updated DAG to pin it and \
continue working.";

/// Escalation message injected when the agent fails to call inject_summary
/// within the normal compaction window.
///
/// This is Level 2 of the three-level convergence guarantee:
/// - Level 1 (normal): Agent builds DAG normally (first stream attempt)
/// - Level 2 (escalation): This message tells agent to stop searching and finalize
/// - Level 3 (force-inject): Engine constructs fallback DAG deterministically
pub const COMPACTION_ESCALATION_MESSAGE: &str = "\
⚠️ COMPACTION TIMEOUT: You have not completed the DAG summary. \
Write a bullet-point summary NOW and call inject_summary immediately. \
Do NOT make any more SessionSearch calls. Summarize what you already know \
into <dag-node> blocks and call inject_summary(content) right now.";

// ============================================================================
// DAG Detection
// ============================================================================

/// Marker for compaction-dag system-reminder blocks.
const COMPACTION_DAG_MARKER: &str = "<!-- type:compaction-dag -->";

/// Detect an existing compaction-dag system-reminder in the message list.
///
/// Scans messages for a user message containing the `<!-- type:compaction-dag -->`
/// marker. If found, extracts the DAG content and determines the maximum
/// `turn_end` from parsed `DagNodeMeta` blocks.
///
/// Returns `Some((dag_content, max_turn_end))` if found, `None` otherwise.
/// If the DAG contains no parseable `<dag-node>` blocks, `max_turn_end` is 0.
pub fn detect_existing_dag(messages: &[Message]) -> Option<(String, usize)> {
    for msg in messages {
        let content_text = match msg {
            Message::User { content } => match content.first() {
                UserContent::Text(t) => t.text.clone(),
                _ => continue,
            },
            _ => continue,
        };

        if content_text.contains(COMPACTION_DAG_MARKER) {
            let nodes = parse_dag_nodes(&content_text, None);
            let max_turn_end = nodes.iter().map(|n| n.turn_end).max().unwrap_or(0);
            return Some((content_text, max_turn_end));
        }
    }

    None
}

// ============================================================================
// Partial DAG Extraction
// ============================================================================

/// Compiled regex for extracting `<dag-node ...>...</dag-node>` blocks.
static DAG_NODE_BLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)<dag-node\s[^>]*>.*?</dag-node>")
        .unwrap_or_else(|_| Regex::new("^$").expect("infallible fallback regex"))
});

/// Extract partial `<dag-node>` blocks from assistant messages.
///
/// Scans all messages for `<dag-node ...>...</dag-node>` blocks in assistant
/// responses. Used by the Level 3 force-inject to recover any partial work
/// the agent did before timing out.
///
/// Returns a Vec of complete `<dag-node>...</dag-node>` block strings.
pub fn extract_partial_dag_nodes(messages: &[Message]) -> Vec<String> {
    use rig::message::AssistantContent;

    let mut nodes = Vec::new();

    for msg in messages {
        let text = match msg {
            Message::Assistant { content, .. } => {
                if content.rest().is_empty() {
                    if let AssistantContent::Text(t) = content.first() {
                        t.text.clone()
                    } else {
                        continue;
                    }
                } else {
                    let mut combined = String::new();
                    let items = collect_items(content);
                    for item in items {
                        if let AssistantContent::Text(t) = item {
                            combined.push_str(&t.text);
                        }
                    }
                    combined
                }
            }
            _ => continue,
        };

        for m in DAG_NODE_BLOCK_RE.find_iter(&text) {
            nodes.push(m.as_str().to_string());
        }
    }

    nodes
}

// ============================================================================
// Force-Inject Fallback
// ============================================================================

/// Force-inject a DAG into the session, bypassing the agent.
///
/// This is Level 3 of the convergence guarantee. Called when the agent
/// has failed to call inject_summary after both normal and escalated attempts.
///
/// Performs the same pattern as `apply_pending_dag`:
/// 1. Reset session to system reminders only
/// 2. Wrap DAG content in compaction-dag system-reminder tags
/// 3. Push as user message
/// 4. Recalculate token tracker
/// 5. Clear compaction_in_progress flag
pub fn force_inject_fallback_dag(
    session: &mut Session,
    compaction_in_progress: &Arc<AtomicBool>,
    dag_content: &str,
) {
    let (reminder_count, compactable_count) = reset_session_to_reminders(session);
    debug!(
        "[force_inject_fallback_dag] reset: reminders={}, compactable={}",
        reminder_count, compactable_count
    );

    let wrapped = wrap_dag_content(dag_content);
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&wrapped)),
    });

    recalculate_token_tracker(session);
    compaction_in_progress.store(false, Ordering::SeqCst);

    debug!(
        "[force_inject_fallback_dag] Force-injected fallback DAG — messages_len={}, tokens={}",
        session.messages.len(),
        session.token_tracker.input_tokens
    );
}
