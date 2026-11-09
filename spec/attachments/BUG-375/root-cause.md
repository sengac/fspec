# BUG-375 root cause

Seen during REFAC-046 (2026-11-02) while shifting `unified_exec/tool.rs` coverage mappings:

- `spec/features/unified-exec-tool.feature.coverage` (before REFAC-046) mapped the four `run` scenarios to
  `tool.rs` 136-219, which held the end of `Tool::call` and the start of `handle_run` (167); the write scenarios
  pointed at 222-258 / 222-391 while `handle_write` was at 290-328; poll/list/close pointed 30-70 lines above their
  handlers. REFAC-046 shifted every mapping line-for-line, so the same drift is preserved in
  `tool.rs` / `tool/handlers.rs`.
- `spec/features/exec-stdin-prompt.feature.coverage` maps `rust/tools/tests/exec_stdin_prompt_p4.rs` lines 29-98 for
  the Bash delegation scenario; the test function is 31-104.

## Why
Coverage line ranges are written once by `fspec link-coverage` and never revalidated. Edits above a mapped range
shift the code, and nothing notices. `fspec audit-coverage` only checks that the mapped files exist ("All mappings
valid"). Re-running `link-coverage` with new test lines adds a second mapping instead of updating the old one.

## Fix
1. Re-point the drifted mappings in these two files to the functions they name.
2. Make `audit-coverage` (or `validate`) check that each test range still contains the scenario's `@step` comments,
   and report ranges that do not.
3. Let `link-coverage` replace an existing mapping for the same test file (flag or default) instead of appending one.
