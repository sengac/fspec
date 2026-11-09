# AST research
- `fspec astgrep --pattern 'let Ok($X) = $E else { $$$B };' --lang rust --path rust/fspec-core/src/commands/update_work_unit_status`: `step_docstrings.rs:203`, `:214`, `:217`.
- Callers of `matching_features`: `grep -rn "matching_features(" rust/fspec-core/src`. Each caller must handle the new Result.
