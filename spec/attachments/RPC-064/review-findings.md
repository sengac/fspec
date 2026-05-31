# Review: RPC-064 — /search slash command end-to-end (UI view)

**Date:** 2026-05-25
**Reviewer:** Claude Code (fspec review skill)
**Result:** ✅ PASS after fixes

---

## Summary

Performed deep ACDD compliance review of RPC-064 against:
- Feature: `spec/features/search-history-debounce-and-polish.feature` (12 scenarios)
- Tests: `codelet/fspec-tui/tests/search_view_rpc064.rs`
- Impl: `dispatch_rpc026.rs`, `search_history_view.rs`, `search_history_view_render.rs`, `components/mod.rs`, `views/agent/dispatch.rs`

All 12 scenarios pass tests, build is clean, coverage 100%, all source files under 300 lines.

---

## 🔴 Critical Issues (Must Fix)

None — but TWO issues were close to critical because the @step assertions did not actually verify what the feature scenario claimed.

---

## 🟡 Warnings (Found & Fixed)

### W-1: `rapid_typing` test bypassed the widget — @step "view's query equals git" was a lie

**File:** `codelet/fspec-tui/tests/search_view_rpc064.rs:153-194` (before fix)

The test dispatched `Action::SearchHistory("g")` / `("gi")` / `("git")` directly via `app.dispatch(...)`. The `SearchHistoryView` widget's own `query` field was never updated (only the widget's `handle_key()` mutates it). The test then asserted `mock.last_history_query() == Some("git")` TWICE under the @step "view's query equals git" — admitting in a code comment that it was substituting the backend's query for the view's query because "we drove App::dispatch directly".

**ACDD violation:** the @step comment must verify what the Gherkin step says. Tests dishonestly claiming to verify a scenario assertion are worse than no test at all.

**Fix applied:** rewrote the When clause to drive each keystroke through `view.handle_key(KeyCode::Char(ch), KeyModifiers::NONE, 20)` — exactly how production keys flow. Each `FilterChanged(q)` outcome is then dispatched as `Action::SearchHistory(q)` (the same wiring `AgentView::handle_search_view_key` performs). Now the widget's `query` advances `""` → `"g"` → `"gi"` → `"git"`, and the final assertion `view.query() == "git"` truthfully verifies the @step.

### W-2: `slower typing` test could not verify "queries sent in order are g, gi, git"

**File:** `codelet/fspec-tui/tests/search_view_rpc064.rs:201-237` (before fix)

The `MockBackend` only tracks `last_history_query` (a single `Mutex<Option<String>>`) — not a history of queries. The original test only waited for `search_history_calls() >= N` after each keystroke without checking which query landed at the backend at that point. The trailing `last_history_query == "git"` assertion only verifies the FINAL state, not the per-step sequence.

**Fix applied:** rewrote the test to (a) drive each keystroke through `view.handle_key` so the view's query advances correctly per char, then (b) `wait_until` BOTH the call counter AND `mock.last_history_query()` equal THIS keystroke's cumulative query before typing the next char. The in-order "g" → "gi" → "git" sequence is now pinned step-by-step at the backend boundary.

### W-3: `esc_closes` test asserted input == `""` while @step claimed input == `"draft text"`

**File:** `codelet/fspec-tui/tests/search_view_rpc064.rs:551-578` (before fix)

The scenario's final step reads:

```
And AgentView.input.value() equals "draft text"
```

But the test asserted `app.navigator().agent.input.value() == input_after_open` — where `input_after_open` was the input's value AFTER `Action::OpenSearchView` ran. Because `handle_open_search_view` was calling `self.navigator.agent.input.reset()`, that value was `""`. The test comment admitted it: "Esc must leave the input value unchanged from the moment /search opened" — silently weakening the @step.

**Root cause:** an implementation bug. RPC-064's TS-parity goal (per `spec/attachments/RPC-064/slash-search.md` "Enter inserts the selected result into the **live input**") requires preserving the live input draft across `/search` open → close. Example [10] on the card spells this out explicitly: "input remains unchanged (whatever it was before /search opened)". `handle_open_search_view` was clearing the live input on every open path, breaking that contract for the Ctrl+R chord.

The slash-palette path is already fine because `AgentView::handle_popup_key` (`PopupOutcome::Selected`) explicitly calls `self.input.reset()` BEFORE emitting `Action::SlashCommandSelected(...)`, so the typed `/search` text is still cleared when the user picks /search from the palette.

**Fix applied:**
- Removed `self.navigator.agent.input.reset()` from `handle_open_search_view` so the Ctrl+R chord no longer wipes the in-progress draft (slash-palette path is unaffected — `handle_popup_key` clears upstream).
- Updated the `esc_closes` test to literally assert `input.value() == "draft text"` and to drive `KeyCode::Esc` through `view.handle_key` so the test verifies the full widget → action chain rather than just the dispatcher's reaction to a hand-rolled `Action::CloseSearchView`.

### W-4: `enter_inserts` test bypassed the widget's Enter handling

**File:** `codelet/fspec-tui/tests/search_view_rpc064.rs:510-544` (before fix)

@step said "When the user presses Enter" but the test dispatched `Action::InsertIntoInput("git status")` directly. The widget's `KeyCode::Enter` → `Selected(text)` outcome was never exercised end-to-end.

**Fix applied:** drives `KeyCode::Enter` through `view.handle_key`, asserts the outcome is `Selected("git status")`, then dispatches `Action::InsertIntoInput("git status")` exactly as `AgentView::handle_search_view_key` would. Same end state, but the test now PROVES Enter on the highlighted match produces the correct action.

---

## 🟢 Observations (Nice to Have — Not Fixed, Out of Scope)

### O-1: `highlight_query` has a latent UTF-8 boundary risk

`search_history_view_render.rs::highlight_query` calls `text.to_lowercase()` to find case-insensitive query matches, then byte-slices the ORIGINAL `text` by offsets returned from the LOWERCASED string. For Unicode characters where `to_lowercase()` changes byte length (e.g. German `ß` → `SS` uppercase, Turkish dotted/dotless `İ`/`ı`), the offsets won't align, which can either return wrong spans or panic on a non-char-boundary byte slice. Not exercised by any scenario in this card (all tests use ASCII), so this is a future-proofing concern outside RPC-064's scope.

### O-2: `search_history_debounce_handle` is left set after a successful flush

`handle_search_history` parks an `AbortHandle` on `App`. After the task naturally completes (sleep → backend call → dispatch result), the `AbortHandle` is still present — the next keystroke calls `.abort()` on a completed task (no-op). Slightly untidy but functionally correct. Not in scope.

### O-3: `components/mod.rs` is 793 lines

The shared `Action` enum file is well over the 300-line ceiling. RPC-064 only added a few lines to it (widening one variant). Not a regression introduced by this card.

---

## Coverage Verification (after fixes)

- Feature file: `spec/features/search-history-debounce-and-polish.feature` — ✅ OK, valid Gherkin, 12 scenarios
- Test file: `codelet/fspec-tui/tests/search_view_rpc064.rs` — ✅ OK, all 12 tests pass, all @step comments match feature text
- Impl files:
  - `codelet/fspec-tui/src/app/dispatch_rpc026.rs` (282 lines) — ✅ OK
  - `codelet/fspec-tui/src/views/agent/search_history_view.rs` (264 lines) — ✅ OK
  - `codelet/fspec-tui/src/views/agent/search_history_view_render.rs` (179 lines) — ✅ OK
  - `codelet/fspec-tui/src/views/agent/dispatch.rs` — ✅ Ctrl+R chord wiring at L208-218
- Scenario coverage: **12 / 12** scenarios linked to test + impl with corrected line ranges

---

## Files Modified

- `codelet/fspec-tui/src/app/dispatch_rpc026.rs` — removed `input.reset()` from `handle_open_search_view` (fixes W-3 root cause); doc-string explains the TS-parity contract and where the palette path clears input upstream.
- `codelet/fspec-tui/tests/search_view_rpc064.rs` — fixed 4 tests (rapid typing, slower typing, enter, esc) to drive keystrokes through the widget so @step comments actually verify their scenario assertions.

## Files Reviewed (Read-Only)

- `spec/features/search-history-debounce-and-polish.feature`
- `codelet/fspec-tui/tests/search_view_rpc064.rs`
- `codelet/fspec-tui/tests/rpc026_search_history_view.rs`
- `codelet/fspec-tui/tests/common/mod.rs` (MockBackend)
- `codelet/fspec-tui/src/app/dispatch_rpc026.rs`
- `codelet/fspec-tui/src/app/state.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (SlashCommandAction::Search dispatcher)
- `codelet/fspec-tui/src/views/agent/search_history_view.rs`
- `codelet/fspec-tui/src/views/agent/search_history_view_render.rs`
- `codelet/fspec-tui/src/views/agent/dispatch.rs` (Ctrl+R chord + handle_search_view_key + handle_popup_key)
- `codelet/fspec-tui/src/components/mod.rs` (Action::HistorySearchResults variant)
- `spec/attachments/RPC-064/slash-search.md`
- `spec/attachments/RPC-064/ast-research-search-view.md`

## Verification Run

```
cargo build                              → clean
cargo test --test search_view_rpc064    → 12 passed
cargo test --test rpc026_search_history_view → 7 passed (no regression)
cargo test                               → all 870+ tests pass, no regressions
fspec validate spec/features/search-history-debounce-and-polish.feature → valid
fspec show-coverage search-history-debounce-and-polish → 100% (12/12)
```
