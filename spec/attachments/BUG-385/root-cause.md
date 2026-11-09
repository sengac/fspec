# BUG-385 root cause

`update-work-unit-status` (split into `rust/fspec-core/src/commands/update_work_unit_status/` by REFAC-048) reads
work-unit fields with silent defaults instead of failing on malformed data:

- `update_work_unit_status.rs` `run`: `wu.r#type.clone().unwrap_or_else(|| "story".to_string())` - a missing type
  becomes a story, so story-only gates run (or task exemptions are lost).
- `review.rs` `check_testing_entry`: `linkedFeatures` that is not an array -> `.unwrap_or(false)` (no links).
- `board_state.rs` `apply_status_change`: a non-array `stateHistory` is replaced with `[]` (history lost);
  `if let Some(parsed) = status_from_str(..)` ignores an unparsable status.
- `queries.rs`: `array_len` / `string_array` / `has_estimate` read a non-array or wrongly typed field as empty.

## Fix direction
Distinguish "absent" (legitimately empty) from "present but wrong type" (error naming the work unit and field), make
`status_from_str` failure an error, and decide whether a missing `type` is legal (schema says?) - if not, error. One
test per malformed field through `dispatch_command`.
