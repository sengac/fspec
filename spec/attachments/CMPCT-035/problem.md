# FV-003-a — `parse_dag_nodes` accepts reverse turn ranges

**Severity:** Low
**Source:** `codelet/core/src/compaction/mod.rs` — `parse_dag_nodes`
**Surfaced by:** Formal verification cross-check (FV-003)
**Pinned test:** `codelet/core/src/compaction/__tests__/dag_node_proptest.test.rs::limitation_parser_does_not_validate_start_le_end`

---

## Problem

`parse_dag_nodes` extracts the `turns="N-M"` attribute from `<dag-node>` blocks
using a regex that captures two integers. There is **no validation that
`turn_start <= turn_end`** — the parser silently accepts inverted ranges.

### Reproduction

```rust
use codelet_core::compaction::parse_dag_nodes;

let input = r#"<dag-node depth="D0" turns="50-10" label="x">content</dag-node>"#;
let nodes = parse_dag_nodes(input, None);

assert_eq!(nodes[0].turn_start, 50);
assert_eq!(nodes[0].turn_end, 10);   // <-- start > end, accepted silently
```

### Why this matters

The Alloy model for FV-003 (`codelet/core/spec/compaction/dag_compaction.als`)
assumes every `DagNode` satisfies `turn_start <= turn_end`. Downstream
consumers that compute coverage, sort by range, or slice conversation history
may behave unexpectedly if an inverted range slips through.

In production today this is benign: the LLM generates dag-node blocks via a
template that produces well-formed ranges. But this is a **gap between the
formal model's assumption and the parser's actual contract**, and any future
prompt change, provider switch, or bypass of the template would silently
propagate ill-formed ranges instead of being rejected at the parse boundary.

---

## Expected behaviour

`parse_dag_nodes` should reject (skip + log) any `<dag-node>` block whose
`turns` attribute has `start > end`.

```rust
// Pseudocode
if turn_start > turn_end {
    tracing::warn!(
        turn_start, turn_end,
        "Skipping dag-node with inverted turn range"
    );
    continue;
}
```

The pinned `limitation_*` test in `dag_node_proptest.test.rs` documents the
**current** loose behaviour. When this story is fixed, that test must be
**deleted** (or converted to a positive test asserting the node is rejected),
and a new positive test added to confirm that valid ranges still parse.

---

## Definition of Done

1. `parse_dag_nodes` rejects `<dag-node>` blocks where parsed `turn_start > turn_end`.
2. New unit test: rejection produces a logged warning and excludes the node.
3. New proptest assertion: for every parsed `DagNode`, `node.turn_start <= node.turn_end`.
4. Existing `limitation_parser_does_not_validate_start_le_end` is **removed** (the
   limitation is no longer accurate).
5. **Update `docs/FORMAL_VERIFICATION.md`:**
   - Remove the FV-003-a row from the "Findings (open observations)" table.
   - Update the FV-003 row in the "Proofs" status table from
     `✅ Cross-checked + 3 limitations pinned` to
     `✅ Cross-checked + 2 limitations pinned` (or `+ N limitations pinned`
     reflecting the post-fix count).
6. All Alloy assertions and proptests still pass.
