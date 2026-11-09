# BUG-384 AST research

- `grep -n "features.first()" rust/fspec-core/src/commands/update_work_unit_status/prefill.rs` -> `check_prefill`.
- `grep -rn "linked_feature_files" rust/fspec-core/src/commands/update_work_unit_status` -> prefill.rs (definition,
  check_prefill), review.rs (scenario existence, bug linked feature), temporal.rs (feature mtime ordering).
- `detect_prefill(content: &str) -> Vec<PrefillMatch>` works on one file's text; PrefillMatch has no path field.
