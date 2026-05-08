# AST research — `parse_dag_nodes` for CMPCT-036 (FV-003-b)

This research locates every caller and the implementation site of
`parse_dag_nodes` so the overlap-rejection change can be made safely.

## Definition site

```
codelet/core/src/compaction/model.rs:421:1:
  pub fn parse_dag_nodes(dag_content: &str, message_count: Option<usize>) -> Vec<DagNodeMeta>
```

This is the only definition (re-exported from `codelet::compaction::mod`
at line 57).

## Production callers (must keep working unchanged)

```
codelet/cli/src/compaction_dag.rs:155:25
  parse_dag_nodes(&content_text, None)

codelet/napi/src/inject_summary_handler.rs:317:21
  parse_dag_nodes(&wrapped, Some(message_count))

codelet/napi/src/inject_summary_handler.rs:500:21
  parse_dag_nodes(dag_content, None)
```

Both NAPI call sites are inside the inject_summary_handler — they consume
parsed nodes for compaction state. Tighter rejection here is a strict
improvement (they never relied on the duplicate same-depth output) so no
caller code changes are needed.

The CLI call site is a debug/inspection path; it just renders nodes.

## Test callers (one is the limitation pin to remove)

```
codelet/core/src/compaction/__tests__/dag_node_proptest.test.rs:358:17
  parse_dag_nodes(content, None)        # limitation_parser_does_not_reject_overlap — REMOVE
codelet/core/src/compaction/__tests__/dag_node_proptest.test.rs:374:17
  parse_dag_nodes(content, Some(60))    # limitation_clamping_can_invert_range — KEEP (FV-003-c)
```

All other proptest/unit-test call sites in `dag_node_parsing.test.rs` and
`dag_node_proptest.test.rs` use either single-block input or
non-overlapping ranges, so they will not start failing once overlap
rejection is enabled. Quick manual scan:

| File | Line | Notes |
|------|------|-------|
| `dag_node_parsing.test.rs` | 45, 93, 117, 146, 175, 203, 274, 308, 327 | Single-block or empty fixtures — no overlap |
| `dag_node_proptest.test.rs` | 241, 260, 279, 300 | CMPCT-035 (FV-003-a) cases — single block or distinct ranges |
| `dag_node_proptest.test.rs` | 358 | **Limitation pin to remove** |
| `dag_node_proptest.test.rs` | 374 | Clamping limitation (FV-003-c) — single block, keep |

## Existing overlap code path to replace

`codelet/core/src/compaction/model.rs:482-492` currently runs:

```rust
nodes.sort_by_key(|n| n.turn_start);
for i in 1..nodes.len() {
    if nodes[i].turn_start <= nodes[i - 1].turn_end {
        tracing::warn!(...);
    }
}
```

This compares ANY adjacent nodes after sorting, ignoring depth. The
rewrite must:

1. Sort by `(depth, turn_start)`.
2. Scan with a per-depth "last accepted" marker — drop the later node when
   `turn_start <= last_accepted.turn_end` AT THE SAME DEPTH only.
3. Re-sort the surviving nodes by `turn_start` (P1 invariant).
4. Emit `tracing::warn!` only on actual drops, with kept + dropped labels
   and ranges plus the depth.

Cross-depth overlap (D1 spanning the same turns as D2) is intentional —
the AST scan confirms no caller depends on cross-depth uniqueness either.
