# PROV-050: Split-Safe Compaction — Implementation Guide

## Problem

Compaction can split a tool call from its tool result, leaving orphaned entries in the compacted history. This can confuse the model in subsequent turns (seeing a tool result without a corresponding call, or a call without a result).

## VTCode Reference

### HistoryValidationReport (`vtcode-core/src/core/agent/state.rs` lines 39–88)

```rust
/// Items that participate in call/output pairing for validation
pub enum PairableHistoryItem {
    ToolCall { call_id: ToolCallId, tool_name: String },
    ToolOutput { call_id: ToolCallId, status: OutputStatus },
}

/// Record of a missing output in conversation history
pub struct MissingOutput {
    pub call_id: ToolCallId,
    pub tool_name: String,
}

/// Validation report for conversation history state
pub struct HistoryValidationReport {
    pub missing_outputs: Vec<MissingOutput>,    // Calls without results
    pub orphan_outputs: Vec<ToolCallId>,         // Results without calls
}

impl HistoryValidationReport {
    pub fn is_valid(&self) -> bool {
        self.missing_outputs.is_empty() && self.orphan_outputs.is_empty()
    }
}
```

### Safe split point (`vtcode-core/src/core/agent/runner/summarize.rs` lines 24–34)

```rust
pub(super) fn summarize_conversation_if_needed(
    &self,
    session_state: &mut AgentSessionState,
    preserve_recent_turns: usize,
    utilization: f64,
) {
    // ... threshold checks ...

    let preferred_split_at = session_state.conversation.len()
        .saturating_sub(preserve_recent_turns);

    // Context Manager: Find a safe split point that doesn't break
    // tool call/output pairs.
    let split_at = session_state.find_safe_split_point(preferred_split_at);

    if split_at == 0 {
        return; // Can't safely split — don't compact
    }

    // ... proceed with summarization at safe split_at ...
}
```

### Comment on safe split concept

VTCode's `find_safe_split_point()` walks backward from the preferred split position to find a boundary where all tool calls before the split have their corresponding results also before the split (or vice versa — no call/result pair spans the boundary).

## Proposed Implementation for fspec

### 1. Message analysis helper

```rust
// codelet/core/src/compaction/split_safety.rs (new file)

use rig::message::{Message, UserContent, AssistantContent};

/// Check if a proposed split point would break any tool call/result pairs.
/// Returns the adjusted (safe) split point, which may be earlier than proposed.
///
/// A "safe" split point is one where:
/// - No tool call before the split has its result after the split
/// - No tool result before the split has its call after the split
pub fn find_safe_split_point(messages: &[Message], proposed: usize) -> usize {
    if proposed == 0 || proposed >= messages.len() {
        return proposed;
    }

    // Collect all tool call IDs before and after the proposed split
    let mut calls_before: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut results_before: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut calls_after: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut results_after: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (idx, msg) in messages.iter().enumerate() {
        let (calls, results) = if idx < proposed {
            (&mut calls_before, &mut results_before)
        } else {
            (&mut calls_after, &mut results_after)
        };

        match msg {
            Message::Assistant { content } => {
                for item in content.iter() {
                    if let AssistantContent::ToolCall(tc) = item {
                        calls.insert(tc.id.clone());
                    }
                }
            }
            Message::User { content } => {
                for item in content.iter() {
                    if let UserContent::ToolResult(tr) = item {
                        results.insert(tr.id.clone());
                    }
                }
            }
        }
    }

    // Check if any call before split has result after split
    let broken_calls: Vec<_> = calls_before.intersection(&results_after).collect();
    // Check if any result before split has call after split
    let broken_results: Vec<_> = results_before.intersection(&calls_after).collect();

    if broken_calls.is_empty() && broken_results.is_empty() {
        return proposed; // Safe as-is
    }

    // Walk backward to find a safe point
    let mut safe = proposed;
    while safe > 0 {
        safe -= 1;

        // Recheck with new split point
        // (simplified: just walk back past any tool call/result that spans the boundary)
        let msg = &messages[safe];
        let is_tool_boundary = match msg {
            Message::Assistant { content } => {
                content.iter().any(|c| matches!(c, AssistantContent::ToolCall(_)))
            }
            Message::User { content } => {
                content.iter().any(|c| matches!(c, UserContent::ToolResult(_)))
            }
        };

        if !is_tool_boundary {
            // Found a message that's not part of a tool call/result pair
            // Check if this is a clean user→assistant boundary
            if safe > 0 {
                let prev = &messages[safe - 1];
                if matches!(prev, Message::Assistant { .. }) && matches!(msg, Message::User { .. }) {
                    // Clean boundary between assistant response and next user message
                    return safe;
                }
            }
        }
    }

    0 // Can't find a safe split — don't compact
}
```

### 2. Integrate into execute_compaction

```rust
// In codelet/core/src/compaction/mod.rs or interactive_helpers.rs

// Before splitting messages for compaction:
let proposed_split = messages.len().saturating_sub(preserve_count);
let safe_split = find_safe_split_point(&messages, proposed_split);

if safe_split == 0 {
    warn!("Cannot find safe split point for compaction — all messages are in tool call/result pairs");
    return Err(anyhow!("Compaction would break tool call/result pairs"));
}

// Use safe_split instead of proposed_split
let messages_to_summarize = &messages[..safe_split];
let messages_to_keep = &messages[safe_split..];
```

### 3. Validation helper (diagnostic)

```rust
/// Validate that conversation history has no orphaned tool calls or results.
/// Returns issues found (empty = valid).
pub fn validate_tool_pairing(messages: &[Message]) -> Vec<String> {
    let mut calls: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut results: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut issues = Vec::new();

    for (idx, msg) in messages.iter().enumerate() {
        match msg {
            Message::Assistant { content } => {
                for item in content.iter() {
                    if let AssistantContent::ToolCall(tc) = item {
                        calls.insert(tc.id.clone(), idx);
                    }
                }
            }
            Message::User { content } => {
                for item in content.iter() {
                    if let UserContent::ToolResult(tr) = item {
                        results.insert(tr.id.clone(), idx);
                    }
                }
            }
        }
    }

    // Find calls without results
    for (id, idx) in &calls {
        if !results.contains_key(id) {
            issues.push(format!("Tool call {} at message {} has no result", id, idx));
        }
    }

    // Find results without calls
    for (id, idx) in &results {
        if !calls.contains_key(id) {
            issues.push(format!("Tool result {} at message {} has no call", id, idx));
        }
    }

    issues
}
```

### 4. Tests

```rust
#[test]
fn safe_split_at_clean_boundary() {
    let messages = vec![
        user_message("hello"),
        assistant_text("hi"),
        user_message("do something"),
        assistant_text("done"),
    ];
    assert_eq!(find_safe_split_point(&messages, 2), 2);
}

#[test]
fn safe_split_walks_back_past_tool_pair() {
    let messages = vec![
        user_message("hello"),
        assistant_text("hi"),
        assistant_tool_call("call_1", "read_file", "{}"),
        user_tool_result("call_1", "file contents"),
        assistant_text("I see the file"),
    ];
    // Proposed split at 3 would break call_1 pair
    let safe = find_safe_split_point(&messages, 3);
    assert!(safe <= 2); // Must be before the tool call
}

#[test]
fn validate_tool_pairing_clean() {
    let messages = vec![
        assistant_tool_call("call_1", "read_file", "{}"),
        user_tool_result("call_1", "contents"),
    ];
    assert!(validate_tool_pairing(&messages).is_empty());
}

#[test]
fn validate_tool_pairing_orphaned_call() {
    let messages = vec![
        assistant_tool_call("call_1", "read_file", "{}"),
        // No result for call_1
    ];
    let issues = validate_tool_pairing(&messages);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].contains("call_1"));
}
```

## Estimated Effort: 5 story points
