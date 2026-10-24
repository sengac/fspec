//! DAG compaction instructions, detection, extraction, and force-injection.
//!
//! Contains:
//! - Compaction instruction constants (FRESH, INCREMENTAL, ESCALATION)
//! - Existing DAG detection in session messages
//! - Partial dag-node extraction from assistant messages
//! - Force-inject DAG (Level 3 convergence guarantee)

use codelet_core::compaction::parse_dag_nodes;
use rig::message::{Message, UserContent};

pub const DAG_NODE_SYNTAX: &str = "\
Return a complete candidate, not prose, a patch, or an unfinished fragment.
Each <dag-node> requires exactly depth, turns and label attributes with double-quoted values,
a supported depth (D0, D1 or D2), numeric persisted turn coordinates and a nonempty label and body.
Close every <dag-node> with </dag-node>. Do not nest DAG structures inside node bodies.
Example: <dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">Evidence-based summary</dag-node>.
Use actual authoritative coordinates and evidence, never copy example bounds blindly.";

pub const DAG_RANGE_RULES: &str = "\
Persisted turn ranges are inclusive. Same-depth peers must be disjoint:
0-10 and 10-20 overlap; 0-10 and 11-20 are disjoint.
Cross-depth overlap is valid, including parent/child nesting.
Recheck same-depth conflicts after promotion, including preserved D2 nodes.
Regenerate a complete evidence-backed candidate when peers conflict; do not silently drop nodes,
clamp bounds, invent coverage or rewrite preserved durable payloads. Their union must cover the captured prefix.";

const FILE_METADATA_INSTRUCTION: &str = "\
File metadata syntax is strict. Each nonempty line inside <dag-files> must be
exactly a unique '- path (Operation)' entry, with Operation exactly Created, Modified, or Deleted:
<dag-files>
- path/to/file.rs (Created)
- path/to/other.rs (Modified)
</dag-files>
No prose, backticks, or explanations belong inside <dag-files>. Put explanations in dag-node bodies.
Do not write '(Created; explanation)' or a bullet saying no files changed.
If no files changed and no existing file identities must be preserved, use <dag-files></dag-files>.
Keep existing file identities unless an authoritative later operation updates them.";

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

   Search/show are previews, not exact evidence. Select original evidence deliberately:
   - SessionSearch(originals, session_id: <session UUID>, start_turn: <start>, end_turn: <end>, limit: 20, max_bytes: 61440)
   Follow next_turn with the same session/range until null. Originals preserve complete structured envelopes;
   missing or oversized evidence is a typed failure, never a reason to fabricate evidence from previews.

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

   Same-depth turn ranges must be disjoint and their union must cover the session.
   Different depths may overlap to express hierarchical coverage.
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
   Search/show are previews. For exact evidence select SessionSearch(originals, session_id: <session UUID>, start_turn: {last_compacted_turn}, limit: 20, max_bytes: 61440).
   Follow next_turn with the same session/range until null; never reconstruct missing originals from previews.
5. Write NEW D0 nodes covering only the fresh turns
6. PRESERVE the <dag-files> section — update it with any new file modifications from fresh turns. \
Keep deleted files listed with operation Deleted. Add entries for newly created or modified files.

Your existing DAG is below — update it, don't rebuild from scratch:

{existing_dag_content}

Call inject_summary(content) with your complete updated DAG to pin it and \
continue working.";

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
// CMPCT-044: Compactor sub-agent prompt builder
// ============================================================================

pub fn build_generate_compaction_prompt(
    target_session: uuid::Uuid,
    existing_dag: Option<(String, usize)>,
) -> String {
    let target = target_session.to_string();

    let base = match existing_dag {
        Some((dag_content, max_turn_end)) => {
            let last_compacted_turn = max_turn_end.saturating_add(1);
            COMPACTION_INSTRUCTION_INCREMENTAL
                .replace("{existing_dag_content}", &dag_content)
                .replace("{last_compacted_turn}", &last_compacted_turn.to_string())
        }
        None => COMPACTION_INSTRUCTION_FRESH.to_string(),
    };

    // "Your conversation history" → the explicit target session id.
    let scoped = base.replace(
        "Your conversation history has been preserved on disk",
        &format!("Session {target} has been preserved on disk"),
    );

    // Every SessionSearch call must target the target session explicitly —
    // the sub-agent's own session is its ephemeral one.
    let target_qualified = scoped
        .replace(
            "session_id: <session UUID>",
            &format!("session_id: \"{target}\""),
        )
        .replace(
            "SessionSearch(show, ",
            &format!("SessionSearch(session_id: \"{target}\", show, "),
        )
        .replace(
            "SessionSearch(search, ",
            &format!("SessionSearch(session_id: \"{target}\", search, "),
        );

    // Final step: output the DAG as the final response instead of calling
    // inject_summary (the sub-agent has no such tool). The FRESH and
    // INCREMENTAL instructions differ only in "updated" here — replace
    // both variants.
    let output_instruction = format!(
        "Output the complete DAG (all <dag-node> blocks + the \
<dag-files> section) as your FINAL RESPONSE. Do not call any other tool \
after outputting it — the engine pins the DAG to session {target} on your \
behalf."
    );
    let instruction = target_qualified
        .replace(
            "Call inject_summary(content) with your complete updated DAG to pin it and \
continue working.",
            &output_instruction,
        )
        .replace(
            "Call inject_summary(content) with your complete DAG to pin it and \
continue working.",
            &output_instruction,
        );
    format!(
        "{instruction}\n\n{DAG_NODE_SYNTAX}\n\n{DAG_RANGE_RULES}\n\n{FILE_METADATA_INSTRUCTION}"
    )
}

#[cfg(test)]
#[path = "compaction_dag_prompt_tests.rs"]
mod cmpct044_prompt_tests;
