# PROV-050: Refactoring Guide — Split-Safe Compaction + Compaction Module Cleanup

## Refactoring Opportunity

PROV-050 adds split-safety to compaction. This is the vehicle for cleaning up the scattered compaction code and establishing clear module boundaries.

## Current Compaction Code Smell: Feature Envy

`stream_loop.rs` contains ~200 lines of compaction orchestration that **should not be there**:

| Section | Lines | What It Does | Where It Should Be |
|---------|-------|-------------|-------------------|
| Pre-prompt check | 654–705 | Estimate tokens, trigger compaction | `compaction_orchestrator` |
| signal_compaction_needed() | 421–429 | Set flag on TokenState | Already fine (thin helper) |
| Post-loop compaction | 2013–2301 | Detect hook trigger, execute, retry | `compaction_orchestrator` |
| Compaction retry stream | 2093–2275 | Create new stream after compaction | Should be a `process_stream()` call |

The stream loop has **feature envy** toward the compaction system — it knows too much about compaction internals (token estimation, turn counting, retry timing, compression ratio calculation).

## New Module: `compaction/split_safety.rs` (~100 lines)

```rust
use rig::message::{Message, AssistantContent, UserContent};

/// Finds a safe compaction split point that doesn't orphan tool call/result pairs.
///
/// Given a proposed split index (boundary between "summarize" and "keep"),
/// walks backward to find the nearest clean boundary where no tool call
/// on one side has its result on the other.
///
/// Returns the adjusted split index, or 0 if no safe split exists
/// (meaning all messages are interleaved tool pairs).
pub fn find_safe_split_point(messages: &[Message], proposed_split: usize) -> usize {
    if proposed_split == 0 || proposed_split >= messages.len() {
        return proposed_split;
    }

    // Collect all tool_call IDs and tool_result IDs on each side
    let mut split = proposed_split;

    loop {
        let (before, after) = messages.split_at(split);
        let calls_before = collect_tool_call_ids(before);
        let results_after = collect_tool_result_ids(after);
        let calls_after = collect_tool_call_ids(after);
        let results_before = collect_tool_result_ids(before);

        // Check: any call in "before" with result in "after"?
        let orphaned_calls = calls_before.iter().any(|id| results_after.contains(id));
        // Check: any result in "before" with call in "after"? (shouldn't happen but check)
        let orphaned_results = results_before.iter().any(|id| calls_after.contains(id));

        if !orphaned_calls && !orphaned_results {
            return split; // Safe!
        }

        // Walk backward to find a clean assistant→user boundary
        if split == 0 {
            return 0; // No safe split exists
        }
        split -= 1;

        // Skip to the nearest message boundary (prefer assistant→user transitions)
        while split > 0 {
            if is_user_message(&messages[split]) && split > 0 && is_assistant_message(&messages[split - 1]) {
                break; // Clean boundary: ...assistant][user...
            }
            split -= 1;
        }

        if split == 0 {
            return 0; // Couldn't find any safe boundary
        }
    }
}

/// Diagnostic: find orphaned tool calls or results in a message sequence.
/// Returns pairs of (message_index, tool_id) that have no matching counterpart.
pub fn validate_tool_pairing(messages: &[Message]) -> Vec<OrphanedTool> {
    let mut calls: HashMap<String, usize> = HashMap::new(); // id → message_index
    let mut results: HashMap<String, usize> = HashMap::new();

    for (i, msg) in messages.iter().enumerate() {
        for id in extract_tool_call_ids(msg) { calls.insert(id, i); }
        for id in extract_tool_result_ids(msg) { results.insert(id, i); }
    }

    let mut orphans = Vec::new();
    for (id, idx) in &calls {
        if !results.contains_key(id) {
            orphans.push(OrphanedTool { index: *idx, tool_id: id.clone(), kind: OrphanKind::CallWithoutResult });
        }
    }
    for (id, idx) in &results {
        if !calls.contains_key(id) {
            orphans.push(OrphanedTool { index: *idx, tool_id: id.clone(), kind: OrphanKind::ResultWithoutCall });
        }
    }
    orphans
}

pub struct OrphanedTool {
    pub index: usize,
    pub tool_id: String,
    pub kind: OrphanKind,
}

pub enum OrphanKind {
    CallWithoutResult,
    ResultWithoutCall,
}

fn collect_tool_call_ids(messages: &[Message]) -> HashSet<String> { /* ... */ }
fn collect_tool_result_ids(messages: &[Message]) -> HashSet<String> { /* ... */ }
```

## Integration Point: `interactive_helpers.rs`

In `execute_compaction()`, before calling `reset_session_to_reminders()`:

```rust
use crate::compaction::split_safety::{find_safe_split_point, validate_tool_pairing};

// PROV-050: Validate tool pairing before compaction
let orphans = validate_tool_pairing(&session.messages);
if !orphans.is_empty() {
    warn!("PROV-050: Found {} orphaned tool call/result pairs before compaction", orphans.len());
    // Log but don't block — orphans may already exist from previous incomplete turns
}

// PROV-050: Adjust split point to avoid breaking tool pairs
// The split point is determined by partition_for_compaction,
// but we need to ensure the "keep latest reminders" boundary
// doesn't bisect a tool call/result pair.
let proposed_split = find_reminder_boundary(&session.messages);
let safe_split = find_safe_split_point(&session.messages, proposed_split);

if safe_split != proposed_split {
    info!("PROV-050: Adjusted compaction split from {} to {} to preserve tool pairs", proposed_split, safe_split);
}

if safe_split == 0 {
    warn!("PROV-050: No safe compaction split point found — aborting compaction");
    output.emit_compaction_failed("No safe split point: all messages contain interleaved tool pairs");
    return Ok(()); // Don't compact, let context fill further
}
```

## Compaction Orchestration Extraction (Opportunistic)

While touching the compaction path, consider extracting the stream_loop compaction logic:

### Current (scattered across stream_loop.rs)

```
Pre-prompt check: 654-705 (51 lines)
Post-loop handler: 2013-2301 (288 lines)
Total: 339 lines in stream_loop.rs
```

### Target: `compaction_orchestrator.rs`

```rust
pub struct CompactionOrchestrator {
    threshold: u64,
    context_window: u64,
    compaction_in_progress: Arc<AtomicBool>,
}

impl CompactionOrchestrator {
    /// Check if context needs compaction before adding a new prompt.
    pub fn should_compact_before_prompt(&self, session: &Session, prompt: &str) -> bool { ... }

    /// Execute compaction and prepare for retry.
    pub async fn execute_and_prepare_retry(
        &self, session: &mut Session, prompt: &str
    ) -> Result<CompactionResult> { ... }
}

pub enum CompactionResult {
    Compacted { original_tokens: u64, compacted_tokens: u64 },
    Skipped { reason: &'static str },
    Failed { error: String },
}
```

This would remove ~300 lines from stream_loop.rs. However, this is a larger refactoring that may be better as a separate card. The PROV-050 card should focus on split-safety and only extract if time permits.

## Tests

```rust
#[test]
fn safe_split_at_clean_boundary() {
    let messages = vec![
        user_msg("Hello"),
        assistant_msg("Hi"),
        user_msg("Do X"),
        assistant_msg("Done"),
    ];
    assert_eq!(find_safe_split_point(&messages, 2), 2);
}

#[test]
fn adjust_split_to_avoid_orphaned_tool_call() {
    let messages = vec![
        user_msg("Hello"),
        assistant_tool_call("tc-1", "bash", "ls"),  // call in "before"
        user_tool_result("tc-1", "file1 file2"),     // result in "after"
        assistant_msg("Done"),
    ];
    // Proposed split at 2 would orphan tc-1's call from its result
    let safe = find_safe_split_point(&messages, 2);
    assert!(safe < 2 || safe > 2); // Must move to avoid the split
}

#[test]
fn no_safe_split_returns_zero() {
    let messages = vec![
        assistant_tool_call("tc-1", "bash", "ls"),
        user_tool_result("tc-1", "ok"),
    ];
    assert_eq!(find_safe_split_point(&messages, 1), 0);
}

#[test]
fn validate_finds_orphaned_call() {
    let messages = vec![
        assistant_tool_call("tc-1", "bash", "ls"),
        // No tool result for tc-1!
        assistant_msg("Done"),
    ];
    let orphans = validate_tool_pairing(&messages);
    assert_eq!(orphans.len(), 1);
    assert!(matches!(orphans[0].kind, OrphanKind::CallWithoutResult));
}
```

## Estimated Impact

- **New module**: `compaction/split_safety.rs` (~100 lines)
- **Modified**: `interactive_helpers.rs` (~20 lines in execute_compaction)
- **Lines removed from stream_loop.rs**: 0 directly (but sets up for future compaction_orchestrator extraction)
- **Bug prevented**: No more orphaned tool calls/results after compaction
