@done
@tui-component
@slash-command
@history-search
@search
@agent-view
@rust
@RPC-064
Feature: /search slash command end-to-end (UI view)

  """
  [A] Debounce strategy: emit `FilterChanged(query)` immediately from the widget on every keystroke (preserve existing RPC-026 behaviour and tests). The DEBOUNCE lives in `App::handle_search_history` — when called, schedule a tokio sleep(150ms) task that, after the wait, fires `backend.persistence_search_history(query)`. A new keystroke aborts the prior in-flight tokio JoinHandle (stored on `App` or on a new `SearchDebounceState` field) so only the final query reaches the backend within a fast burst. Synchronous test-runtime path (`Handle::try_current().is_err()`) skips the debounce and silently no-ops, matching existing dispatch_rpc026.rs pattern.
  [B] Stale result discarding: widen `Action::HistorySearchResults` from `Vec<HistoryMatch>` to a struct-style variant `{ query: String, matches: Vec<HistoryMatch> }`. The spawned task captures the query it sent and emits it back with the matches. `handle_history_search_results` compares the response query to `search_view.query()` and only folds when they match. Update existing rpc026 tests that construct `Action::HistorySearchResults(...)` to use the new variant form. The `SearchHistoryView` gains a `set_matches_for(query: &str, matches: Vec<HistoryMatch>)` helper OR the dispatcher does the equality check before calling `set_matches`.
  [C] Result highlighting: `SearchHistoryView::render_body` switches from `Paragraph::new(Line::from(Span::styled(label, style)))` to building a `Vec<Span>` where each case-insensitive occurrence of `self.query` inside `m.text` is split into BOLD vs plain spans. A small helper `highlight_query(text: &str, query: &str, row_style: Style)` returns the `Vec<Span<'static>>` (or `Vec<Span<'_>>`). Selected-row inversion (REVERSED) is applied on top of the per-span BOLD so the highlighted-row visual still indicates selection. Empty query takes the existing no-highlight path.
  [D] j/k navigation: `SearchHistoryView::handle_key` adds `KeyCode::Char('j')` → move_by(+1) and `KeyCode::Char('k')` → move_by(-1) BEFORE the generic `KeyCode::Char(c)` branch that appends to the query. Both must be no-modifier (Ctrl/Alt rejected at the top of handle_key). This means uppercase J/K typed into the search query still works because `KeyModifiers::SHIFT` will be set for capital letters and the j/k branch checks for empty modifiers. Update the existing rpc026_search_history_view test `typing_characters_emits_filter_changed` confirmation that lowercase j/k now navigate instead of filter (alternative: pick different test chars to keep filter test intact). Resolution: the existing test types `g`, `i`, `t` — none of which are j/k — so it's unaffected.
  [E] File layout: changes confined to (1) `codelet/fspec-tui/src/views/agent/search_history_view.rs` — add j/k branch, add `highlight_query` helper, extend handle_key tests; (2) `codelet/fspec-tui/src/app/dispatch_rpc026.rs` — add debounce + stale-discard logic; (3) `codelet/fspec-tui/src/components/mod.rs` — widen `Action::HistorySearchResults` variant; (4) new integration test file `codelet/fspec-tui/tests/search_view_rpc064.rs` covering debounce + stale + insert + dismiss + j/k + highlight. NO new dispatch_rpc064.rs file expected (logic fits in existing dispatch_rpc026.rs). All affected files must stay under 300 lines — search_history_view.rs is at 297 lines currently, so the j/k + highlight changes will push it past the budget; extract the renderer body to a new sibling file `search_history_view_render.rs` if needed.
  [F] Backwards-compat impact: widening `Action::HistorySearchResults` from a tuple variant `(Vec<HistoryMatch>)` to a struct variant breaks every test that constructed it. Affected tests today: `rpc026_app_dispatch.rs` does NOT construct it directly (the spawned task does). Only the dispatcher path constructs it via `action_tx.send(Action::HistorySearchResults(matches))` in `dispatch_rpc026.rs` line 158. So the impact is limited to one production-code line plus the new test file. Confirmed by `Grep HistorySearchResults` — only 2 hits.
  [G] App state for debounce: add `last_search_debounce: Option<tokio::task::JoinHandle<()>>` to `App` (in `src/app/state.rs`). Each `handle_search_history` call aborts the previous handle (if any), then spawns a new tokio task that does `tokio::time::sleep(Duration::from_millis(150)).await` then calls `backend.persistence_search_history(query)` and dispatches `Action::HistorySearchResults { query, matches }`. The spawn ALSO pushes the new handle into `App::pending_tasks` so the existing test harness drains it. The 150ms constant lives as `const SEARCH_DEBOUNCE_MS: u64 = 150;` in `dispatch_rpc026.rs`.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Both `/search` (picked from the slash palette) and `Ctrl+R` (chord) open the SearchHistoryView via `Action::OpenSearchView` with no backend call until the user types
  #   2. Typing into the open SearchHistoryView debounces keystrokes by ~150ms before dispatching `backend.persistence_search_history(query)` (rapid typing fires at most one RPC per debounce window with the latest text)
  #   3. Each rendered result row highlights every case-insensitive occurrence of the current query inside the result text using `Modifier::BOLD` (rest of the text renders plain)
  #   4. Selection navigation accepts both arrow keys (↑/↓) AND vim-style j/k (lowercase, no modifiers) so the result list matches the TS frontend's keybindings
  #   5. Enter on a highlighted result dispatches `Action::InsertIntoInput(text)` and `Action::CloseSearchView`, leaving the input pre-filled (but not auto-submitted) so the user can edit before pressing Enter again
  #   6. Esc closes the view via `Action::CloseSearchView` without inserting anything into the input
  #   7. Stale results from an in-flight RPC whose query no longer matches the view's current query are discarded — the dispatcher folds `HistorySearchResults` only when the response's query equals the view's current query
  #   8. Empty query state renders the existing `(type to search history)` placeholder and never fires a backend call (no implicit recent-history fetch — matches the TS behaviour)
  #
  # EXAMPLES:
  #   1. User picks /search from the palette while no popup or mode-view is open — AgentView.search_view becomes Some(SearchHistoryView with empty query) and AgentView.slash_popup is reset to None; no backend call has fired yet
  #   2. User presses Ctrl+R while no popup or mode-view is open — `Action::OpenSearchView` is dispatched and SearchHistoryView appears empty, same as the palette path
  #   3. User types `g`, `i`, `t` within 150ms of each other — only one `backend.persistence_search_history("git")` call fires (the previous timer is cancelled on each keystroke); the view's `query` reflects the latest typed text
  #   4. User types `g`, waits 200ms (longer than the debounce window), then types `i`, waits 200ms, then types `t` — three separate backend round-trips fire with queries `g`, `gi`, `git`
  #   5. Backend returns the result list for query `g` 250ms after the user has already typed up to `git` — the stale `g` response is discarded and the view continues to show the prior matches (or empty placeholder) until the `git` response lands
  #   6. Result row text is `git status now` and the current query is `git` — the substring `git` is rendered with `Modifier::BOLD`, the trailing ` status now` is rendered plain
  #   7. Result row text is `GIT add then git push` and the current query is `git` (lowercase) — both `GIT` and `git` are rendered with `Modifier::BOLD` (case-insensitive match, original casing preserved)
  #   8. User presses `j` with two results loaded — `selected_index` advances from 0 to 1; pressing `k` moves back to 0; pressing `j` again wraps from 1 to 0 (matching the existing arrow-key wrap semantics)
  #   9. User presses Enter on the highlighted match `git status` — `Action::InsertIntoInput("git status")` is dispatched, the input's value becomes `git status`, the search_view is removed, and the input is NOT auto-submitted
  #   10. User presses Esc with `git` typed and a match highlighted — `Action::CloseSearchView` is dispatched, the search_view is removed, and the input remains unchanged (whatever it was before /search opened)
  #   11. User opens /search with an empty query — the placeholder `(type to search history)` is rendered in the body and zero backend calls have fired; pressing Esc immediately closes without RPC traffic
  #
  # ========================================

  Background: User Story
    As a fspec user with a long command history
    I want to open the /search view, type to filter, see matches highlighted, and pick one with Enter to insert into the input
    So that I can quickly re-run a previous prompt without retyping it

  Scenario: Picking /search from the palette opens the SearchHistoryView empty with no backend call
    Given AgentView has no popups or mode views open
    When the user picks "/search" from the slash command palette
    Then AgentView.slash_popup is None
    And AgentView.search_view is Some(SearchHistoryView with empty query)
    And backend.persistence_search_history has not been invoked

  Scenario: Pressing Ctrl+R opens the SearchHistoryView empty with no backend call
    Given AgentView has no popups or mode views open
    And no search_view, resume_view, slash_popup, or file_popup is active
    When the user presses Ctrl+R
    Then Action::OpenSearchView is dispatched
    And AgentView.search_view is Some(SearchHistoryView with empty query)
    And backend.persistence_search_history has not been invoked

  Scenario: Rapid typing within 150ms fires a single debounced backend call with the final query
    Given search_view is open with an empty query
    When the user types "g" then "i" then "t" within 150ms of each other
    Then only one backend.persistence_search_history call has fired
    And the last_history_query observed by the mock backend equals "git"
    And the view's query equals "git"

  Scenario: Typing slower than the debounce window fires one backend call per keystroke
    Given search_view is open with an empty query
    When the user types "g", waits longer than 150ms, types "i", waits longer than 150ms, then types "t"
    Then backend.persistence_search_history has fired three times
    And the queries sent in order are "g", "gi", "git"

  Scenario: Stale results from an older query are discarded
    Given search_view is open with the current query "git"
    And an in-flight backend response is pending for the older query "g"
    When the older response Action::HistorySearchResults { query = "g", matches = [HistoryMatch("git log")] } arrives
    Then search_view.matches remains unchanged from its previous state
    And the view does NOT fold the stale "g" response into the visible matches

  Scenario: Fresh results matching the current query are folded into the view
    Given search_view is open with the current query "git"
    When Action::HistorySearchResults { query = "git", matches = [HistoryMatch("git status"), HistoryMatch("git push")] } is dispatched
    Then search_view.matches has length 2
    And search_view.selected_index equals 0

  Scenario: Result row highlights the matching substring in bold
    Given search_view is open with query "git"
    And the loaded matches contain "git status now"
    When AgentView paints the search_view body
    Then the substring "git" inside the row is rendered with Modifier::BOLD
    And the substring " status now" inside the row is rendered without Modifier::BOLD

  Scenario: Result row highlights every case-insensitive occurrence in bold
    Given search_view is open with the lowercase query "git"
    And the loaded matches contain "GIT add then git push"
    When AgentView paints the search_view body
    Then the rendered row preserves the original casing "GIT add then git push"
    And both substrings "GIT" and "git" inside the row are rendered with Modifier::BOLD
    And the substring " add then " between them is rendered without Modifier::BOLD

  Scenario: Pressing j moves selection down and k moves it up with wrap-around
    Given search_view has two loaded matches and selected_index equals 0
    When the user presses "j"
    Then selected_index equals 1
    When the user presses "k"
    Then selected_index equals 0
    When the user presses "j" twice
    Then selected_index equals 0
    And the j/k keystrokes did NOT modify the query buffer

  Scenario: Enter on a highlighted match inserts the text and closes the view
    Given search_view is open with query "git"
    And two matches are loaded with "git status" at selected_index 0
    When the user presses Enter
    Then Action::InsertIntoInput("git status") is dispatched
    And AgentView.search_view is None
    And AgentView.input.value() equals "git status"
    And the input was NOT auto-submitted

  Scenario: Esc closes the view without inserting and leaves the input unchanged
    Given the input contained "draft text" before /search was opened
    And search_view is open with query "git" and a highlighted match "git status"
    When the user presses Esc
    Then Action::CloseSearchView is dispatched
    And AgentView.search_view is None
    And AgentView.input.value() equals "draft text"

  Scenario: Empty query state renders the placeholder and fires no backend calls
    Given the user just opened /search and has not typed anything
    When AgentView paints the search_view body
    Then the body contains the placeholder "(type to search history)"
    And backend.persistence_search_history has not been invoked
    When the user presses Esc
    Then AgentView.search_view is None
    And backend.persistence_search_history has still not been invoked
