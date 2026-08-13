# FV-003-c — Clamping in `parse_dag_nodes` can produce inverted ranges

**Severity:** Low
**Source:** `codelet/core/src/compaction/mod.rs` — `parse_dag_nodes`
**Surfaced by:** Formal verification cross-check (FV-003)
**Pinned test:** `codelet/core/src/compaction/__tests__/dag_node_proptest.test.rs::limitation_clamping_can_invert_range`

---

## Problem

When `parse_dag_nodes` is called with a `message_count` argument, it clamps
`turn_end` to `message_count - 1` so that out-of-bounds ranges don't overflow
the conversation array. **Only `turn_end` is clamped** — `turn_start` is
left as-is. When `turn_start >= message_count`, the result is an **inverted
range** (`start > end`).

### Reproduction

```rust
use codelet_core::compaction::parse_dag_nodes;

let input = r#"<dag-node depth="D0" turns="200-300" label="x">…</dag-node>"#;
let nodes = parse_dag_nodes(input, Some(60));

assert_eq!(nodes[0].turn_start, 200);  // unclamped
assert_eq!(nodes[0].turn_end, 59);     // clamped to message_count - 1
// ⇒ start (200) > end (59), inverted range silently produced
```

### Why this matters

The Alloy model assumes every `DagNode` satisfies `turn_start <= turn_end`
(see also FV-003-a). Even when the *input* is well-formed (e.g.,
`turns="200-300"` is a valid increasing range from the LLM's perspective),
the *clamping logic* itself can violate that invariant.

This is a more subtle gap than FV-003-a because the parser is the producer
of the violation, not just a passive accepter. A downstream caller that
trusts the model's invariant will be surprised even though the input was
well-formed.

In production today this is benign because the LLM never produces ranges
outside the actual message count — but the same defensive clamping that's
meant to protect downstream code can itself create an invalid state.

---

## Expected behaviour

When `turn_start >= message_count`, the entire node should be **rejected**
(the range refers to non-existent turns). When `turn_start < message_count`
but `turn_end >= message_count`, clamping `turn_end` is fine because the
result still satisfies `turn_start <= turn_end`.

```rust
// Pseudocode
if let Some(mc) = message_count {
    if turn_start >= mc {
        tracing::warn!(
            turn_start, message_count = mc,
            "Skipping dag-node whose start is beyond message_count"
        );
        continue;
    }
    if turn_end >= mc {
        turn_end = mc - 1;  // safe: turn_start < mc, so turn_start <= turn_end
    }
}
```

This composes cleanly with FV-003-a's start≤end check: if FV-003-a is fixed
first, this case becomes "input was valid (start<=end) but clamping must
preserve that invariant".

---

## Definition of Done

1. `parse_dag_nodes` rejects nodes where `turn_start >= message_count` when
   `message_count` is provided.
2. Clamping `turn_end` only fires when `turn_start < message_count`, so it
   can never invert a range.
3. New unit test: `turns="200-300"` with `message_count=60` → node is dropped
   with a warning.
4. New unit test: `turns="50-300"` with `message_count=60` → node returned
   with `turn_end=59`, `turn_start=50`.
5. New proptest assertion: post-clamping, every output node satisfies
   `turn_start <= turn_end`.
6. Existing `limitation_clamping_can_invert_range` is **removed**.
7. **Update `docs/FORMAL_VERIFICATION.md`:**
   - Remove the FV-003-c row from the "Findings (open observations)" table.
   - If FV-003-a and FV-003-b are also fixed, change the FV-003 row in the
     "Proofs" status table from
     `✅ Cross-checked + 3 limitations pinned` to `✅ Cross-checked`.
8. All Alloy assertions and proptests still pass.

---

## Dependency note

This story is most cleanly resolved **after** CMPCT-035 (FV-003-a). With
start≤end already enforced at the input boundary, the clamping fix becomes
purely a "preserve the invariant" change rather than introducing a new
validation step.
