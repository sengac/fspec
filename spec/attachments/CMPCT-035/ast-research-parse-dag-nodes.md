# AST Research — CMPCT-035 (parse_dag_nodes turn-range validation)

## Target function

- `pub fn parse_dag_nodes(dag_content: &str, message_count: Option<usize>) -> Vec<DagNodeMeta>`
- Source: `codelet/core/src/compaction/model.rs:414`

## Existing tracing usage in module

- `tracing::warn!(...)` already used at `codelet/core/src/compaction/model.rs:463` for overlap detection.
- The same macro must be used to log inverted-range rejection (consistency with existing warn pattern).

## Call sites of `parse_dag_nodes` (impact analysis)

| File | Line | Context |
|------|------|---------|
| `codelet/cli/src/compaction_dag.rs` | 155 | CLI dag command — consumes `Vec<DagNodeMeta>` |
| `codelet/napi/src/inject_summary_handler.rs` | 317 | NAPI handler with `message_count` |
| `codelet/napi/src/inject_summary_handler.rs` | 500 | NAPI handler without `message_count` |
| `codelet/core/src/compaction/__tests__/dag_node_parsing.test.rs` | many | Existing happy-path tests (must keep passing) |
| `codelet/core/src/compaction/__tests__/dag_node_proptest.test.rs` | 229 | `limitation_parser_does_not_validate_start_le_end` — TO BE DELETED |
| `codelet/core/src/compaction/__tests__/dag_node_proptest.test.rs` | 245 | `limitation_parser_does_not_reject_overlap` — keep |
| `codelet/core/src/compaction/__tests__/dag_node_proptest.test.rs` | 261 | `limitation_clamping_can_invert_range` — KEEP (FV-003-c, out of scope) |

## Conclusion

- All call sites already accept `Vec<DagNodeMeta>` and tolerate empty results — rejection is safe.
- `tracing::warn!` is the established channel for parse-time diagnostics.
- The fix must be **pre-clamping** so the limitation `limitation_clamping_can_invert_range`
  (FV-003-c) remains accurate, since FV-003-c is explicitly out of scope per
  the architecture note attached to CMPCT-035.
