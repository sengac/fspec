# RPC-064 — AST Research: /search slash command end-to-end

**Card:** RPC-064 · **Parent:** RPC-030 · **Phase:** 6.4 · **Date:** 2026-05-25

## Purpose

Confirm the current Rust shape of the `/search` path before extending it with
debounce + result highlighting + stale-result discard + `j`/`k` navigation.

## Key existing entities (AST-confirmed)

### `SearchHistoryView` — `codelet/fspec-tui/src/views/agent/search_history_view.rs`

- `pub struct SearchHistoryView { query, matches, selected_index, scroll_offset, wheel }`
- `pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers, visible_rows: usize) -> SearchHistoryViewOutcome` (line 162)
  - Currently rejects Ctrl/Alt at top.
  - `KeyCode::Char(c)` appends to query and emits `FilterChanged(self.query.clone())`.
  - **Will gain**: `KeyCode::Char('j')` → move_by(+1) + `KeyCode::Char('k')` → move_by(-1) inserted BEFORE the generic char branch.
- `render_body(&self, area, buf)` (line 239) renders rows via
  `Paragraph::new(Line::from(Span::styled(label, style)))`.
  - **Will change**: build a `Vec<Span>` per row so the BOLD highlight applies only
    to substrings matching `self.query` (case-insensitive).

### `App::handle_search_history` — `codelet/fspec-tui/src/app/dispatch_rpc026.rs`

- `pub(crate) fn handle_search_history(&mut self, query: String)` (line 150)
- Currently: bare `tokio::spawn` → `backend.persistence_search_history(query)` →
  `action_tx.send(Action::HistorySearchResults(matches))`.
- **Will change**:
  1. Abort prior `App::last_search_debounce_handle` (new field on `App`).
  2. Spawn a new task that sleeps `SEARCH_DEBOUNCE_MS = 150ms` (with
     `tokio::time::sleep`) BEFORE calling the backend.
  3. The spawned task captures the **query** it sent and emits
     `Action::HistorySearchResults { query, matches }` (variant widened).
- Synchronous test-runtime path (`Handle::try_current().is_err()`) keeps the
  same no-op fallback used by other dispatchers (RPC-026 / RPC-052 precedent).

### `Action::HistorySearchResults` — `codelet/fspec-tui/src/components/mod.rs`

- Current variant: `HistorySearchResults(Vec<HistoryMatch>)` (line 317).
- Only ONE production caller constructs it: the spawn inside `handle_search_history`
  (`dispatch_rpc026.rs:158`).
- Only ONE production dispatch site reads it: `dispatch.rs:276` →
  `self.handle_history_search_results(m.clone())`.
- Test references: `tests/rpc026_source_shape.rs:224` asserts source-substring
  presence — keep the variant name `HistorySearchResults`. The variant SHAPE
  changes from tuple to struct: `HistorySearchResults { query: String, matches: Vec<HistoryMatch> }`.
- **Impact (`AstGrep`-confirmed):** zero call-sites construct the tuple variant
  outside the one spawn — risk surface is tiny.

### App state for debounce — `codelet/fspec-tui/src/app/state.rs`

- `App` already carries `pending_input_save_handle: Option<JoinHandle<()>>`
  (RPC-052) as precedent for single-in-flight debounced tasks.
- **Will add**: `last_search_debounce_handle: Option<JoinHandle<()>>` initialised
  to `None` in `with_action_bus`.

### `handle_history_search_results` — `codelet/fspec-tui/src/app/dispatch_rpc026.rs:166`

- Currently: `if let Some(v) = self.navigator.agent.search_view.as_mut() { v.set_matches(matches); }`.
- **Will change**: signature widens to take `query: String, matches: Vec<HistoryMatch>`.
  The dispatcher folds the matches ONLY when `v.query() == query` — stale
  responses for older queries are silently dropped.

### `dispatch.rs` routing — `codelet/fspec-tui/src/app/dispatch.rs:276`

- Pattern: `Action::HistorySearchResults(m) => self.handle_history_search_results(m.clone())`.
- **Will change** to the struct-pattern form `Action::HistorySearchResults { query, matches } => self.handle_history_search_results(query.clone(), matches.clone())`.

## Existing tests to preserve

- `tests/rpc026_search_history_view.rs` — widget unit tests. The j/k addition
  must NOT break `typing_characters_emits_filter_changed` (typed chars are
  `g`, `i`, `t` — none of which are `j`/`k`).
- `tests/rpc026_app_dispatch.rs::search_history_spawns_backend_call` —
  expects ONE backend call after `Action::SearchHistory("g")` is dispatched.
  Must continue to pass with the debounce in place (test awaits the task).
- `tests/rpc026_source_shape.rs` — substring-grep for `HistorySearchResults`
  remains satisfied (only the variant shape changes; the identifier stays).

## New test file plan

- `codelet/fspec-tui/tests/search_view_rpc064.rs`:
  1. Debounce: two `SearchHistory` dispatches inside the debounce window
     should result in ONE backend call with the latest query.
  2. Stale-discard: dispatch `HistorySearchResults { query: "g", matches: [...] }`
     while the view's `query` is `git` — `view.matches` must NOT update.
  3. Insert + dismiss: existing parity assertions for Enter + Esc.
  4. `j`/`k` navigation: type two chars, set matches, press `j` → selected_index 1,
     press `k` → selected_index 0.
  5. Highlight: render the view with `query="git"` and a row `"git status"` —
     buffer cell at the start of the highlighted substring must have
     `Modifier::BOLD` set; cells after the substring must not.

## Conclusion

The plan in the work-unit's architecture notes is consistent with the actual
source shape — no surprises. Implementation can proceed.
