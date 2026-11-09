# BUG-366 root cause

The workspace denies `clippy::unwrap_used`, `clippy::expect_used` and `clippy::panic`. Most fspec-tui integration tests opt out with a crate-level `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`: 303 of the 385 files in `rust/fspec-tui/tests/` carry that exact line. Their helpers also drop errors. For example, `drain_pending` (copied into many files and support modules, including `exec_stdin_push_chunk_bug171_support`) discards task join errors with `let _ = handle.await;`.

Fix direction: in batches under child cards, make the tests return `TestResult` (the `tests/common` alias used by BUG-209's support module) and use `?`, make the helpers propagate join errors, and then delete the allow line.
