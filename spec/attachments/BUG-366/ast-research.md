# BUG-366 AST research

## grep -l 'allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)' rust/fspec-tui/tests/*.rs | wc -l
     303

Files written without the allow use `TestResult` (`tests/exec_stdin_freshness_support/mod.rs`, `exec_stdin_probe_freshness_bug209.rs`).
