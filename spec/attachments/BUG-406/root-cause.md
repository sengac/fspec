# BUG-406 root cause

`codelet_common::set_data_directory` sets a process-global `Mutex<Option<PathBuf>>`. The test files listed in
ast-research.md call it as `let _ = ..set_data_directory(..)`, dropping its `Result` (no-fallbacks rule), and some pass
a per-test tempdir that is deleted when that test ends while other tests in the same binary still use the global
path. That is the BUG-397 failure mode (tests fail together or by order, pass alone). Fix per binary: one
`OnceLock<TempDir>` shared by every test, set once, with its `Result` handled (the BUG-397 pattern in
`rust/sessions/tests/rpc410_hitl_wire_protocol_parity_support/mod.rs`).
