# BUG-386 root cause

`rust/fspec-core/src/commands/update_work_unit_status/temporal.rs` (temporal-ordering gate, specifying -> testing
for feature files and testing -> implementing for test files):

- `find_test_files`: `for entry in entries.flatten()` drops directory-entry errors and
  `if let Ok(content) = std::fs::read_to_string(&path)` skips unreadable test files, so a test file that cannot be
  read is never checked.
- `file_mtime_millis` turns metadata / `modified()` / pre-epoch errors into `None` (`.ok()?`) and
  `check_temporal_ordering` does `None => continue`, so a file whose time cannot be read passes the gate.

BUG-378 made `linked_feature_files` (the feature half of this gate) fail with a path-naming `FeatureScanError`; the
test-file half and the mtime read still pass vacuously.

## Fix direction
Return `Result` from `find_test_files` and `file_mtime_millis` with errors naming the path (NotFound on a scanned
root = nothing to check, as BUG-360 decided), propagate from `check_temporal_ordering`, and add scenarios for an
unreadable test file and an unreadable file time.
