# CMPCT-020: Compaction Convergence Guarantee — Watchdog and Escalation

## What This Card Does

Adds a **turn-count watchdog** that detects when the agent fails to complete DAG construction within a reasonable number of turns, with three-level escalation that guarantees compaction always converges.

## The Problem

Currently, if the agent gets confused during compaction (loops on SessionSearch, forgets to call inject_summary, or gets stuck reasoning), there's no safety net. The session stays in `compaction_in_progress` state indefinitely, blocked from going idle. The only recovery is the user manually intervening.

## Three-Level Escalation Model (from LCM paper)

### Level 1: Normal (Turns 1–4)
No intervention. The agent builds its DAG normally through SessionSearch + inject_summary.

### Level 2: Aggressive Escalation (Turn 5)
Inject a strong instruction after the agent's response:

```
⚠️ COMPACTION TIMEOUT: You've used 5 turns without completing the DAG summary.
Write a bullet-point summary NOW and call inject_summary immediately.
Do NOT make any more SessionSearch calls. Summarize what you know.
```

This is injected as a user message before the next API call.

### Level 3: Deterministic Force-Inject (Turn 7)
**Zero LLM calls.** The engine takes over:

1. Scan the last 6 messages for any `<dag-node>` blocks the agent partially wrote
2. If found: assemble them into a DAG and force-call inject_summary handler directly
3. If not found: construct a minimal fallback DAG:
   ```
   <dag-node depth="D1" turns="0-{last_turn}" label="Auto-recovered: compaction timeout">
   Session was auto-compacted due to convergence timeout.
   Use SessionSearch to recover context.
   </dag-node>
   ```
4. Call `apply_pending_dag()` directly — bypass the agent entirely
5. Clear `compaction_in_progress` flag
6. Log a warning about the force-inject

## Implementation

### 1. Watchdog Counter in Stream Loop

**File:** `codelet/cli/src/interactive/stream_loop.rs`

Add a compaction turn counter alongside the existing `compaction_needed` flag:

```rust
// In the stream loop state
let mut compaction_turn_count: usize = 0;
let mut compaction_in_progress = false;

// After each completed turn during compaction:
if compaction_in_progress {
    compaction_turn_count += 1;
    
    match compaction_turn_count {
        5 => {
            // Level 2: Inject escalation message
            inject_compaction_escalation(&mut messages);
        }
        7 => {
            // Level 3: Force-inject and exit compaction
            force_inject_fallback_dag(
                &messages,
                &inject_summary_handler,
                last_turn_index,
            );
            compaction_in_progress = false;
            compaction_turn_count = 0;
            break; // Exit compaction retry loop
        }
        _ => {} // Turns 1-4 and 6: no intervention
    }
}
```

**Reset:** Counter resets to 0 when `inject_summary` is called successfully (i.e., `apply_pending_dag()` runs).

### 2. Level 2 Escalation Function

**File:** `codelet/cli/src/interactive/stream_loop.rs` (or extract to helper)

```rust
fn inject_compaction_escalation(messages: &mut Vec<Message>) {
    messages.push(Message {
        role: Role::User,
        content: Some(COMPACTION_ESCALATION_MESSAGE.to_string()),
    });
}

const COMPACTION_ESCALATION_MESSAGE: &str = r#"
⚠️ COMPACTION TIMEOUT: You've used 5 turns without completing the DAG summary.
Write a bullet-point summary NOW and call inject_summary immediately.
Do NOT make any more SessionSearch calls. Summarize what you know.
"#;
```

### 3. Level 3 Force-Inject Function

**File:** `codelet/napi/src/inject_summary_handler.rs`

Extract a new public function callable by the engine:

```rust
pub fn force_inject_fallback_dag(
    recent_messages: &[Message],
    last_turn: usize,
) -> String {
    // 1. Try to extract <dag-node> blocks from recent messages
    let extracted_nodes = extract_dag_nodes_from_messages(recent_messages);
    
    if !extracted_nodes.is_empty() {
        // Assemble partial nodes into a DAG
        return extracted_nodes.join("\n\n");
    }
    
    // 2. Fallback: minimal recovery DAG
    format!(
        r#"<dag-node depth="D1" turns="0-{}" label="Auto-recovered: compaction timeout">
Session was auto-compacted due to convergence timeout.
Use SessionSearch to recover context.
</dag-node>"#,
        last_turn
    )
}
```

## Interaction with Existing Code

- **`should_idle_on_done()`** (inject_summary_handler.rs:135–160) — Already prevents idle during compaction. Level 3 force-inject clears the flag, so idle works again.
- **`compaction_in_progress` flag** — Stream loop already tracks this. We add a counter alongside it.
- **`execute_compaction()`** — Not modified. The watchdog monitors the _retry stream_ that runs after `execute_compaction()` sets up the context.

## Testing Strategy

- Unit test: Counter increments correctly during compaction
- Unit test: Level 2 escalation message injected at turn 5
- Unit test: Level 3 force-inject at turn 7 — with partial dag-nodes in messages
- Unit test: Level 3 force-inject at turn 7 — with no dag-nodes (fallback DAG)
- Unit test: Counter resets after successful inject_summary
- Integration test: Simulate agent that never calls inject_summary, verify Level 3 fires

## Dependencies

- **CMPCT-017** — `<dag-node>` block format is used by Level 3 to extract partial work
