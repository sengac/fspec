# CMPCT-019: Incremental DAG Condensation

## What This Card Does

Splits the single compaction instruction into two variants — **FRESH** (first compaction, no existing DAG) and **INCREMENTAL** (re-compaction with an existing DAG in context). This prevents the agent from rebuilding the entire DAG from scratch on each compaction cycle, achieving LCM's "incremental condensation."

## Core Concept

Currently, every compaction uses the same `COMPACTION_SYSTEM_INSTRUCTION`. After 3 compaction cycles, the agent keeps re-summarizing everything from scratch. With incremental condensation:

1. **First compaction**: Agent uses FRESH instruction → builds full DAG from session history
2. **Second compaction**: Agent uses INCREMENTAL instruction → preserves D2/D1 nodes, promotes stale D0→D1, summarizes only new turns into D0

## Two Deliverables

### 1. Split Instruction Constants

**File:** `codelet/cli/src/interactive_helpers.rs`  
**Location:** Replace `COMPACTION_SYSTEM_INSTRUCTION` (lines ~245–263)

```rust
/// Used when no compaction-dag exists in the current context
pub const COMPACTION_INSTRUCTION_FRESH: &str = r#"
Your context window was getting full. Your conversation history has been preserved on disk
and is fully searchable via SessionSearch. Build a hierarchical summary DAG of your session:

1. Search strategically (not linearly):
   - SessionSearch(show, max_turns: 10) for recent context
   - SessionSearch(search, query: "error|failed|fix") for error resolutions
   - SessionSearch(search, query: "decision|chose|architecture") for decisions
   - SessionSearch(search, query: "TODO|blocker|question") for open items

2. Write a structured summary using <dag-node> blocks:
   ...
   (structured format from CMPCT-017)

3. Call inject_summary(content) with your complete DAG to pin it and continue working.
"#;

/// Used when a compaction-dag already exists in system reminders
pub const COMPACTION_INSTRUCTION_INCREMENTAL: &str = r#"
Your context window was getting full again. You have an existing DAG summary from a
previous compaction. Update it incrementally:

1. PRESERVE all existing D2 (Durable) nodes unchanged — these are settled decisions
2. REVIEW existing D1 (Arc) nodes — keep if still relevant, promote key parts to D2 if solidified
3. PROMOTE existing D0 (Detailed) nodes to D1 — they're no longer the freshest work
4. Search ONLY for turns since your last compaction:
   - SessionSearch(show, start_turn: {last_compacted_turn}, max_turns: 20)
   - SessionSearch(search, query: "error|fix", start_turn: {last_compacted_turn})
5. Write NEW D0 nodes covering only the fresh turns

Your existing DAG is below — update it, don't rebuild from scratch:
{existing_dag_content}

Call inject_summary(content) with the updated DAG.
"#;
```

### 2. Detection Logic in execute_compaction

**File:** `codelet/cli/src/interactive_helpers.rs`  
**Function:** `execute_compaction()` (lines ~281+)

Before composing the instruction, scan current messages for an existing compaction-dag:

```rust
fn detect_existing_dag(messages: &[Message]) -> Option<(String, usize)> {
    // Scan for system-reminder with type:compaction-dag
    // Return (dag_content, last_compacted_turn_index)
    for msg in messages {
        if let Some(content) = &msg.content {
            if content.contains("<!-- type:compaction-dag -->") {
                // Extract the DAG content between the system-reminder tags
                // Determine last_compacted_turn from DagNodeMeta max turn_end
                // (or from the position of the DAG message itself)
                return Some((extracted_content, max_turn_end));
            }
        }
    }
    None
}

// In execute_compaction():
let instruction = match detect_existing_dag(&current_messages) {
    Some((existing_dag, last_turn)) => {
        COMPACTION_INSTRUCTION_INCREMENTAL
            .replace("{existing_dag_content}", &existing_dag)
            .replace("{last_compacted_turn}", &last_turn.to_string())
    }
    None => COMPACTION_INSTRUCTION_FRESH.to_string(),
};
```

## How It Works End-to-End

```
Session: [reminder] [dag?] [turn83] [turn84] ... [turn120]
                                                    ↓ threshold hit
                                               Is there a <system-reminder type:compaction-dag>?
                                                   │
                                    ┌───── YES ────┘──── NO ─────┐
                                    ↓                            ↓
                          INCREMENTAL mode              FRESH mode
                          - Preserve D2 nodes           - Full SessionSearch
                          - Promote D0 → D1             - Build from scratch
                          - Search only turns 83+       - Create all D0/D1/D2
                          - Write new D0s               
                                    ↓                            ↓
                              inject_summary(updated)    inject_summary(fresh)
```

## Critical Invariant

The incremental instruction MUST include `start_turn` guidance so the agent doesn't re-scan turns it already summarized. This requires CMPCT-018 (turn range params) to be available.

## Testing Strategy

- Unit test: `detect_existing_dag()` finds DAG in messages
- Unit test: `detect_existing_dag()` returns None when no DAG
- Unit test: Instruction selection — fresh vs incremental
- Unit test: Template substitution correctness
- Integration test: First compaction → verify FRESH used, second compaction → verify INCREMENTAL used

## Dependencies

- **CMPCT-017** — Structured dag-node format (so we can extract max turn_end from existing DAG)
- **CMPCT-018** — Turn range queries (so incremental instruction can use start_turn in SessionSearch)
