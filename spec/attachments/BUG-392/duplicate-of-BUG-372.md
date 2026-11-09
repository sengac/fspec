# BUG-392 closed as a duplicate of BUG-372

BUG-392 reports the same defect as BUG-372: `read_tree_or_file` in
`rust/test-helpers/src/source_composition.rs` dropped a `modules::read_tree` error and returned the bare file. BUG-372
fixed it once: a `.rs` path now returns the module-tree error, and only paths that are not `.rs` files are read as
plain text. Scenarios: `spec/features/split-test-source-composition.feature` (@BUG-372). Every caller's test target was
re-run with the fix; none relied on the fallback.
