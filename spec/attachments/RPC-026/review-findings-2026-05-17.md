# Epic Review: RPC-021 — Parallel slices RPC-024 / RPC-025 / RPC-026

**Date:** 2026-05-17
**Reviewer:** Claude Code (fspec review skill)
**Scope:** RPC-025 + RPC-026 (per supervisor request — NOT full epic sweep)

## Summary

| Work Unit | Status | 🔴 Critical | 🟡 Warning | 🟢 Observation |
|-----------|--------|------------|-----------|----------------|
| RPC-025   | PASS   | 0          | 5 (style) | 6 (positive)    |
| RPC-026   | FAIL   | 3          | 6         | 5               |

---

## RPC-025 — Shift+↑/↓ history recall + persistence_*_history RPCs

### Status: PASS

### 🔴 Critical Issues
None.

### 🟡 Warnings (all stylistic / non-blocking — left as observations)
1. `app_dispatch_history_rpc025.rs:295-307` — Setup dispatches (three `Action::HistoryPrev` to reach `recall_index = Some(2)`) are not annotated with their own `@step Given …` comment. Functional behaviour is correct.
2. `history.rs:162` — `tracing::warn!` used without explicit `use tracing` import (relies on crate prelude).
3. `history.rs:261-279` — Double-`Result` pattern from `with_store(|s| s.add(entry))?` is subtle; brief doc would help future readers.
4. `history.rs:96-110` — `HistoryEntry::with_session_id_str(uuid, session_id)` parameter ordering is non-obvious.
5. `dispatch_rpc025.rs:59-61, 169-171` — Silent no-op when `Handle::try_current().is_err()`. A `tracing::debug!` would make the silent drop observable.

### Verdict
RPC-025 meets all card requirements. 32/32 scenarios covered with verbatim `@step` comments. `cargo build` clean and tests pass. **No fixes required** — all warnings are cosmetic / out of scope.

---

## RPC-026 — /resume session picker + /search history palette

### Status: FAIL

### 🔴 Critical Issues (Must Fix)

**1. End-to-end key routing for `resume_popup` and `search_popup` is NOT wired.**
- File: `codelet/fspec-tui/src/views/agent/dispatch.rs:38-81`
- `handle_popup_key` only routes events through `self.slash_popup` and `self.file_popup`. There is no branch for `self.resume_popup.as_mut()` or `self.search_popup.as_mut()`.
- When a user opens `/resume` or `/search` and presses ↑/↓/Enter/Esc/printable chars, those keys fall through to `MultiLineInput` and the popup never reacts.
- Directly violates **architecture note [1]**: *"handle_event in views/agent/dispatch.rs is extended via a new helper that routes keys through the resume / search popups BEFORE the slash / file popups"*.

**2. No view-layer wiring of popup `Outcome` → `Action` for RPC-026 popups.**
- Repo-wide grep shows zero call sites that translate:
  - `ResumePickerOutcome::Selected(id)` → `Action::AttachToSession(id)`
  - `SearchPaletteOutcome::Selected(text)` → `Action::InsertIntoInput(text)`
  - `SearchPaletteOutcome::FilterChanged(q)` → `Action::SearchHistory(q)`
- **Rule [4]**, **Rule [5]**, **Rule [7]** explicitly require this routing in the view layer.
- Without it, examples [1], [2], [5], [7]–[10] cannot occur in production. Only programmatic `app.dispatch(...)` calls (used by tests) trigger the handlers.

**3. End-to-end behaviour described in examples [1]–[2], [5], [7]–[10] is untested.**
- Tests in `tests/app_dispatch_resume_search_rpc026.rs` call `app.dispatch(Action::AttachToSession(...))` directly — they never drive a `KeyEvent` through `AgentView::handle_event` while a `resume_popup` / `search_popup` is open.
- Source-shape tests don't catch this because no shape test asserts that `views/agent/dispatch.rs` matches on the new popups.

### 🟡 Warnings (Should Fix)

1. `dispatch_rpc026.rs:62-77` — `handle_attach_to_session` uses an awkward `while` loop with manual delta arithmetic to step the session index. A direct setter (or computing the delta once) would be clearer.
2. `dispatch_rpc026.rs:29-31` and `:94-97` — Defensive runtime check `tokio::runtime::Handle::try_current().is_err()` is duplicated. Minor DRY violation.
3. `resume_picker.rs:150` and `search_palette.rs:184` — Silent `.take(10)` truncation; the user has no way to see/scroll beyond 10 rows. Not regression-blocking. (OUT OF SCOPE — not in card requirements.)
4. Architecture note [3] says *"Resume → self.dispatch(Action::OpenResumePicker) by emit"* but `dispatch_rpc020.rs` calls the handler directly. (Doc drift, not behavioural.)
5. Architecture note [5] references `rpc026-dispatch.feature`; actual file is `rpc026-app-dispatch.feature`. (Doc drift.)
6. Rule [3] count mismatch — says "Five NEW Action enum variants" then lists seven. (Doc drift.)

### 🟢 Observations
1. Widget unit tests (`resume_picker_widget_rpc026.rs`, `search_palette_widget_rpc026.rs`) are excellent.
2. Source-shape tests provide strong structural guarantees for `dispatch_rpc026.rs`.
3. Cross-transport parity has 2 scenarios — thin but sufficient.
4. Mock backend usage is clean throughout.
5. Render order in `agent.rs:228-237` correctly paints resume/search on top of slash/file.

### Fix Plan
1. Move RPC-026 back to `implementing`.
2. Extend `handle_popup_key` in `views/agent/dispatch.rs` to route through `resume_popup` and `search_popup` BEFORE `slash_popup` / `file_popup`.
3. Translate each Outcome to the correct Action and drop the popup as needed.
4. Add an end-to-end key-routing test that drives `AgentView::handle_event` through both popups.
5. Re-link coverage and re-advance to `done`.

---

## Fix Results (2026-05-17)

### RPC-026

- 🔴 Critical 1 (key routing for resume_popup / search_popup not wired) → ✅ **Fixed**.
  Extended `codelet/fspec-tui/src/views/agent/dispatch.rs` with new helpers
  `handle_resume_popup_key` and `handle_search_popup_key`. `handle_popup_key`
  now routes through them BEFORE the slash / file popups (matches architecture
  note [1]). File grew from 181 to 245 LoC — still under the 300 ceiling.

- 🔴 Critical 2 (no view-layer Outcome → Action translation) → ✅ **Fixed**.
  `handle_resume_popup_key` emits `Action::AttachToSession(id)` on
  `ResumePickerOutcome::Selected`, drops the popup on `Dismiss`, consumes
  `Continued`, and falls through on `Ignored`. `handle_search_popup_key`
  emits `Action::SearchHistory(q)` on `FilterChanged`, emits
  `Action::InsertIntoInput(text)` on `Selected` (and drops the popup),
  drops on `Dismiss`, consumes on `Continued`, falls through on `Ignored`.

- 🔴 Critical 3 (end-to-end behaviour untested) → ✅ **Fixed**.
  New test file `codelet/fspec-tui/tests/view_agent_popup_routing_rpc026.rs`
  adds 10 integration tests that drive raw `KeyEvent` values through
  `AgentView::handle_event` while a popup is open and assert that the
  correct Action lands on the bus (or that none does, where appropriate).
  Covers Enter, Esc, ↑/↓, printable chars, Backspace, and the
  resume-vs-slash precedence ordering required by architecture note [1].

- 🟡 Warnings 1–6 → Deferred (out of strict card scope — cosmetic /
  documentation drift / UX truncation; functionally correct as-is).

### Final Verification
- All 10 new view-routing tests pass: ✅
- All 11 existing `app_dispatch_resume_search_rpc026` tests pass: ✅
- Full `cargo test -p codelet-fspec-tui` (all 50+ suites) pass: ✅
- `cargo build` workspace-wide: ✅
- `fspec validate` on all 921 feature files: ✅
- File LoC budget held: `dispatch.rs` 245 LoC (< 300).

### Files Changed by Fix
- `codelet/fspec-tui/src/views/agent/dispatch.rs` — extended `handle_popup_key`; added `handle_resume_popup_key` + `handle_search_popup_key` helpers; new imports (`ResumePickerOutcome`, `SearchPaletteOutcome`); module header updated to RPC-020/RPC-026.
- `codelet/fspec-tui/tests/view_agent_popup_routing_rpc026.rs` — NEW supplementary integration test (10 tests).

