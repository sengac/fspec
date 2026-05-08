# AST Research — Clamping logic in `parse_dag_nodes`

**Tool:** AstGrep (rust)
**Target:** `codelet/core/src/compaction/model.rs`

---

## Function signature

Located via:
```
AstGrep pattern: pub fn parse_dag_nodes($$$ARGS) -> $RET { $$$BODY }
```

Result:
```
codelet/core/src/compaction/model.rs:431:1
pub fn parse_dag_nodes(dag_content: &str, message_count: Option<usize>) -> Vec<DagNodeMeta>
```

Single definition. `parse_dag_nodes` is re-exported in `compaction/mod.rs` and
is the only public DAG-node parser surface.

## Current clamping logic (lines 445–481)

```rust
let max_turn = message_count.map(|c| if c > 0 { c - 1 } else { 0 });

// inside captures_iter().filter_map(|cap| { ... })
let turn_start: usize = cap[2].parse().ok()?;
let mut turn_end: usize = cap[3].parse().ok()?;
let label = &cap[4];

// CMPCT-035 / FV-003-a — reject reverse ranges (PRE-clamping).
if turn_start > turn_end { return None; }

// Clamp turn_end to message count if provided.
if let Some(max) = max_turn {
    if turn_end > max {
        turn_end = max;
    }
}

Some(DagNodeMeta { depth, turn_start, turn_end, label: label.into() })
```

## Gap

`turn_start` is never bounded against `message_count`. When
`turn_start >= message_count`:

- `max_turn` is `Some(message_count - 1)` (or `Some(0)` for `Some(0)`).
- `turn_end` is clamped down to `max_turn`, possibly below `turn_start`.
- The node is emitted with an inverted range.

`Some(0)` is a degenerate edge: `max_turn = 0`, and *every* node has
`turn_start >= 0 = message_count`, so every node should be dropped.

## Call sites of `parse_dag_nodes`

```
AstGrep pattern: parse_dag_nodes($$$ARGS)
```

Production: invoked from compaction trimmer / annotation pipeline (re-exported
via `compaction::parse_dag_nodes`). All callers pass a possibly-`None`
`message_count`.

Tests: `__tests__/dag_node_parsing.test.rs`, `__tests__/dag_node_proptest.test.rs`.

## Required change

Insert a `turn_start >= mc` rejection step BEFORE the existing clamping step:

```rust
if let Some(mc) = message_count {
    if turn_start >= mc {
        tracing::warn!(turn_start, message_count = mc,
            "Skipping dag-node with turn_start beyond message_count");
        return None;
    }
    // turn_start < mc here; clamping turn_end to mc-1 cannot invert.
    let max = mc.saturating_sub(1);
    if turn_end > max { turn_end = max; }
}
```

This:
- Preserves the FV-003-a invariant (input `start <= end` already enforced).
- Adds the FV-003-c invariant (output `start <= end` after clamping).
- Treats `Some(0)` as "drop everything" (correct: empty conversation).
- Composes cleanly with existing same-depth overlap rejection downstream.

## Tests to update

- `__tests__/dag_node_proptest.test.rs::limitation_clamping_can_invert_range`
  — REMOVE (limitation is now closed).
- Add unit tests matching the new feature scenarios.
- Add proptest: `turn_start <= turn_end` and `turn_end < message_count` for
  every output when `message_count` is `Some(_)`.
