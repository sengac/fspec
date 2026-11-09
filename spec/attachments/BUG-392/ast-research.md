# BUG-392 AST research

- `fspec astgrep --pattern 'if let Ok($A) = modules::read_tree($B) { $$$ }' --lang rust --path rust/test-helpers/src`
  → `source_composition.rs:34`, the only call site.
- `grep -rln "read_tree_or_file" rust/*/tests` lists the size-ceiling tests that depend on it.
