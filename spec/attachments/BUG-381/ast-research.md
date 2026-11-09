# BUG-381 AST research

- `grep -rn "fn handle_enter_work_unit" rust/fspec-tui/src/app` -> `dispatch_work_unit_entry.rs:6` (sole handler,
  routed from `app/dispatch.rs:74`).
- `grep -rn "fn lookup_work_unit_context"` -> `app/dispatch_work_unit_binding.rs:109` (scans `COLUMN_ORDER`
  columns of `board_store`).
- `grep -rn "EnterWorkUnit(" rust/fspec-tui/src/components` -> emitter `components/board_card/events.rs:35`.
- `git log -- rust/fspec-tui/src/app/dispatch_work_unit_entry.rs` -> 626eee1ec replaced direct creation with the
  creation dialog and introduced the silent `else` gap.
