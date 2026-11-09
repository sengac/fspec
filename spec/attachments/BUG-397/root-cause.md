# BUG-397 root cause

`rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs`: five tests each do

```rust
let data_dir = tempfile::tempdir().expect("tempdir");
let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
```

`set_data_directory` sets a process-global path (`common/src/data_dir.rs`), so tests running in parallel overwrite
each other's directory, and each `TempDir` is deleted when its test ends while another test may still be using it.
Session creation then fails, and `fresh_session` uses the legacy `SessionManagerHandle::create_session`, which logs the
error and returns an empty `SessionId` (BUG-399), so the test only fails later at
`manager.get_session(..).expect("session must exist")` (line 189 in `question_without_options_surfaces_empty_options_array`).
The `let _ =` also discards the setter's result.

Fix: one process-wide data directory shared by the binary's tests (or `serial_test` + a directory that outlives every
test), handle the setter's `Result`, and create sessions with `create_session_checked` so a failure names its cause.

**Confirmed (2026-11-03, during BUG-396's run):** `question_without_options_surfaces_empty_options_array` passes when
run alone, and with `--test-threads=1` four of the seven tests fail (lines 149, 189, 265, 331, each at
`get_session(..).expect("session must exist")`): every test after the first finds the data directory pointing at an
earlier test's deleted tempdir.
