# Epic Review: RPC-422 — Session Persistence Integration

**Date:** 2026-07-18
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (RPC-422)

## Summary
- 🔴 Critical: 2 issues across 1 work unit
- 🟡 Warnings: 3 issues across 1 work unit
- 🟢 Observations: 2

## Work Unit Results

### RPC-422: Session Persistence Integration — FAIL

## 🔴 Critical Issues (Must Fix)

1. **Duplicate tracing log line in `session_manager.rs` (lines 1233-1237)**
   The `destroy_session` method has an identical `tracing::info!` call duplicated back-to-back:
   ```rust
   tracing::info!(
       session_id = %uuid,
       session_removed = session.is_some(),
       "destroy_session: shift_remove completed"
   );

   tracing::info!(
       session_id = %uuid,
       session_removed = session.is_some(),
       "destroy_session: shift_remove completed"
   );
   ```
   This is clearly a copy-paste error. Remove the duplicate.

2. **Example map rule [1] contradicts the actual implementation**
   The work unit's example map states:
   > Rule [1]: Session destruction MUST call codelet_core::persistence::delete_session() to remove the on-disk manifest after removing from in-memory map

   But the implementation explicitly does NOT call `delete_session` from `destroy_session`. The comment says:
   > "PARITY FIX: Do NOT delete the session manifest from disk."

   This is a fundamental mismatch between the specified acceptance criteria and the implemented behavior. Either:
   - The rule needs to be updated to reflect the new parity behavior, OR
   - The implementation needs to call `delete_session` to match the rule

   The feature file was updated to reflect the new behavior, but the example map rules were not.

## 🟡 Warnings (Should Fix)

1. **Example map examples [1] and [2] are now stale**
   - Example [1]: "When a session is created in the Rust TUI, the session manifest file is written to disk immediately so it survives process restart" — This is still correct.
   - Example [2]: "When a session is destroyed in the Rust TUI, the session manifest file is removed from disk so it no longer appears in the session list" — This contradicts the new behavior where destroy does NOT remove the manifest.

2. **Test file `resume_session_removal_bug.rs` is 421 lines**
   The file exceeds the 300-line limit. Consider splitting into smaller test modules. The file contains 4 test functions that could be split into separate files or at least into a sub-module.

3. **Missing test for "Session creation fails gracefully when persistence fails" scenario**
   The feature file has a scenario (lines 51-54) about session creation failing when persistence fails, but the test file `rpc422_session_persistence.rs` tests this via `create_session_fails_gracefully_when_persistence_fails()` which creates a file where the sessions directory should be. This is a valid approach but the test doesn't verify that the error propagates correctly — it only checks that no session was created. Consider adding an assertion on the error returned.

## 🟢 Observations (Nice to Have)

1. **Tracing logs are verbose but well-structured**
   The added tracing logs in `manifest.rs` and `dispatch_resume_search_views.rs` follow the project's pattern of including `session_id`, structured fields, and descriptive messages. Good observability.

2. **E2E test file uses real persistence without mocks**
   The `resume_session_removal_bug.rs` test follows the project's philosophy of "integration over mocks" — it uses real `SessionManager`, real persistence, and real `App::dispatch`. This is excellent test quality.

## Coverage Verification
- Feature file: `spec/features/session-persistence-integration.feature` — ✅ Updated with correct scenarios
- Test file(s): `codelet/sessions/tests/rpc422_session_persistence.rs` — ✅ Updated, `codelet/fspec-tui/tests/resume_session_removal_bug.rs` — ✅ New
- Impl file(s): `codelet/sessions/src/session_manager.rs` — ✅ Modified, `codelet/core/src/persistence/manifest.rs` — ✅ Modified
- Scenario coverage: 7/7 scenarios covered

## Files Reviewed
- `codelet/core/src/persistence/manifest.rs` (staged diff)
- `codelet/sessions/src/session_manager.rs` (full file + staged diff)
- `codelet/fspec-tui/src/app/dispatch_resume_search_views.rs` (staged diff)
- `codelet/fspec-tui/tests/resume_session_removal_bug.rs` (staged, new file)
- `codelet/sessions/tests/rpc422_session_persistence.rs` (full file + staged diff)
- `spec/features/session-persistence-integration.feature` (full file + staged diff)
