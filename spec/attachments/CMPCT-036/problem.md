# FV-003-b — `parse_dag_nodes` accepts overlapping same-depth turn ranges

**Severity:** Info
**Source:** `codelet/core/src/compaction/mod.rs` — `parse_dag_nodes`
**Surfaced by:** Formal verification cross-check (FV-003)
**Pinned test:** `codelet/core/src/compaction/__tests__/dag_node_proptest.test.rs::limitation_parser_does_not_reject_overlap`

---

## Problem

When two `<dag-node>` blocks share the same `depth` (e.g., both `D1`) and
their `turns` ranges overlap, `parse_dag_nodes` emits a `tracing::warn!` but
**still returns both nodes** in the output vector.

### Reproduction

```rust
use codelet_core::compaction::parse_dag_nodes;

let input = r#"
<dag-node depth="D1" turns="0-10" label="a">…</dag-node>
<dag-node depth="D1" turns="5-15" label="b">…</dag-node>
"#;
let nodes = parse_dag_nodes(input, None);

assert_eq!(nodes.len(), 2);  // <-- both returned despite overlap
```

### Why this matters

The Alloy model for FV-003 declares non-overlapping ranges within a depth
tier as a **structural fact**:

> Within any single depth, ranges partition (or sub-cover) the turn space
> without overlap. Overlap means the same turn is summarised twice at the
> same depth, which violates the compaction algebra.

The current parser instead treats overlap as a soft warning. Downstream
consumers iterating same-depth nodes may double-count turns, produce
inconsistent coverage maps, or surface confusing UX (e.g., "what does it mean
that turns 5–10 appear in two different D1 summaries?").

This is the lowest-severity finding — the LLM template is unlikely to
produce overlap, and the warning at least leaves a breadcrumb. But the
behaviour is silently inconsistent with the formal model.

---

## Expected behaviour

`parse_dag_nodes` should **reject** any `<dag-node>` whose range overlaps
with a previously-parsed node at the same depth, logging a warning that
identifies both ranges and the dropped node.

```rust
// Pseudocode after sorting same-depth nodes by turn_start
for window in same_depth_nodes.windows(2) {
    if window[1].turn_start <= window[0].turn_end {
        tracing::warn!(
            depth = ?window[0].depth,
            first = ?window[0],
            overlapping = ?window[1],
            "Dropping overlapping same-depth dag-node"
        );
        // exclude window[1] from output
    }
}
```

Alternative: keep the **first** occurrence and drop subsequent overlaps,
matching the parser's existing left-to-right preference.

---

## Definition of Done

1. `parse_dag_nodes` excludes overlapping same-depth nodes from its output
   (keeps first, drops later overlaps).
2. Warning log includes both ranges, the depth, and the dropped label.
3. New unit test: two overlapping D1 ranges → only one node returned.
4. New proptest assertion: for every pair of same-depth nodes in the output,
   their ranges are disjoint.
5. Existing `limitation_parser_does_not_reject_overlap` is **removed**.
6. **Update `docs/FORMAL_VERIFICATION.md`:**
   - Remove the FV-003-b row from the "Findings (open observations)" table.
   - Decrement the limitation count in the FV-003 row of the "Proofs" status
     table.
7. All Alloy assertions and proptests still pass.

---

## Open question

Should overlap across **different depths** also be rejected? The model
permits a D2 range to cover the same span as multiple D1 ranges (that is the
hierarchical compaction story). This story scopes itself to **same-depth
overlap only** — cross-depth coverage is intentional and out of scope.
