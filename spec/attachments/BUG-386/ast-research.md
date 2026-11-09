# BUG-386 AST research

- `grep -n "flatten()\|if let Ok\|\.ok()?\|None => continue" rust/fspec-core/src/commands/update_work_unit_status/temporal.rs`
  -> `find_test_files` (entries.flatten, read_to_string), `file_mtime_millis` (.ok()? x3), the `check_temporal_ordering`
  loop (`None => continue`).
- `file_mtime_iso` uses `file_mtime_millis`-style conversions for the error message only.
- Callers: `check_temporal_ordering` from `run` (testing -> implementing) and `check_testing_entry` (review.rs).
