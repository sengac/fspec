# REFAC-043 AST research

## fspec astgrep --pattern 'fresh_app($A)' --lang rust --path rust/fspec-tui/tests/exec_stdin_push_chunk_bug171.rs
rust/fspec-tui/tests/exec_stdin_push_chunk_bug171.rs:36:28:fresh_app(&["s-1"])
rust/fspec-tui/tests/exec_stdin_push_chunk_bug171.rs:99:27:fresh_app(&["s-1"])
rust/fspec-tui/tests/exec_stdin_push_chunk_bug171.rs:173:27:fresh_app(&["s-1", "s-2"])
rust/fspec-tui/tests/exec_stdin_push_chunk_bug171.rs:226:27:fresh_app(&["s-1"])

## Existing support-module pattern
`tests/exec_stdin_freshness_support/mod.rs` (BUG-209) is the same layout: a directory module, glob-imported by its single feature test file. Its helpers have different signatures (TestResult-returning `fresh_app()` with no session list), so the BUG-171 helpers are moved rather than replaced. Swapping them would change the test bodies, which this pure refactor does not do.
