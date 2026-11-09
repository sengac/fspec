# BUG-380 root cause (investigation handover, 2026-11-03)

## Symptom
`rust/fspec-tui/tests/bug271_session_storage_diagnostics.rs::creation_failures_are_not_missing_model_declines`
fails deterministically (2/2 runs at 3b8004b2a, ~0.03 s each), panicking at line 103:
`visible.contains("persist") || visible.contains("sessionsdir")` is false. The screen is the plain board with no
session storage error dialog. Seen first in the BUG-361 regression run; BUG-361 did not touch this path (it uses
the real embedded/websocket backends from `tests/common/session_storage_fixture.rs`, not MockBackend).

Repro: `cargo test --profile ci-test -p codelet-fspec-tui --test bug271_session_storage_diagnostics -- creation_failures_are_not_missing_model_declines`

## What is known
- The fixture points `codelet_common::set_data_directory` at a temp dir and replaces `<data>/sessions` with a file,
  then dispatches each of `CreateSessionSubmitted`, `EnterWorkUnit("BUG-271")` and `ModelSelected(..)` on a fresh
  App per backend and drains.
- The expected failure text comes from `sessions/src/session_manager/manager_impl_3.rs:157`
  ("Failed to persist session manifest: ..."), which must reach the TUI as `Action::SessionOperationFailed` and be
  painted by `SessionStorageDialog` (`app/session_storage.rs`).
- The assertion message does not say which backend or action failed. The ~0.03 s runtime suggests the first
  iteration (embedded backend, `CreateSessionSubmitted`) fails, i.e. either creation never reaches
  `save_session` (an earlier gate returns without an error action) or the error never becomes a visible action.

## Next steps
1. Specifying: tag `spec/features/session-storage-diagnostics.feature` with `@BUG-380` (feature level), add the
   rule/example, then in testing make the assertion name the backend index and the action so the red run is
   specific.
2. Trace `CreateSessionSubmitted` through `app/dispatch_create_session_dialog.rs` and
   `app/session_creation_tasks.rs` (`post_create_session_action`) for a branch that returns no action, and check
   recent changes to session creation (042d3974b "harden sessions and Codex device authorization", e486b5bf9) and
   persistence (BUG-327/332/333 commits under `core/src/persistence/`).
3. Fix, re-run the whole `bug271_session_storage_diagnostics` binary plus the creation regressions.

## Root cause (confirmed 2026-11-03)
The red run with the assertion naming backend and action failed on `backend 0 (embedded) action
EnterWorkUnit("BUG-271")`; the dialog path passed. 626eee1ec ("isolate session creation intent") changed
`App::handle_enter_work_unit` (`app/dispatch_work_unit_entry.rs`) from creating a session directly to opening the
creation dialog for a card that is on the board (`lookup_work_unit_context`). The fixture's `EnterWorkUnit` ran on an
empty board, so nothing was opened and nothing was created: the test drove a creation path that no longer exists.

## Fix
The fixture's creation paths now follow the current user flow: the app's board shows BUG-271
(`Action::WorkUnitsLoaded`), the work-unit path enters the card and confirms the creation dialog with Enter, and each
assertion names the transport and the path. `drain` returns a panicked/cancelled task as an error instead of dropping
it, and the fixture's drop logs a failed data-directory restore. The silent no-op for an id missing from the board is
filed as BUG-381.
