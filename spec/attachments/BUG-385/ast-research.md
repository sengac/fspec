# BUG-385 AST research

- `grep -n "unwrap_or\|unwrap_or_default\|unwrap_or_else\|if let Some(parsed)\|is_array()" -r
  rust/fspec-core/src/commands/update_work_unit_status/{queries,review,board_state}.rs
  rust/fspec-core/src/commands/update_work_unit_status.rs` -> the sites listed in root-cause.md.
- Work-unit typed fields live in `WorkUnit.extra: serde_json::Map` (`rust/fspec-core/src/types/work_unit.rs`), so the
  type check has to happen at read time.
