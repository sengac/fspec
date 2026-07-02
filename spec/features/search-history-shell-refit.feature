@done
@ui-refinement
@rust
@tui
@RPC-339
Feature: Refit SearchHistoryView onto the shared full-screen shell (title-renderer generalization)
  """
  render_full_screen_scaffold_with_title<T, B> lives in views/full_screen_shell.rs alongside the existing render_full_screen_scaffold and render_full_screen_scaffold_raw_title; body = Clear.render + Layout [Length(1),Length(1),Min(0),Length(1)] -> (split[0], split[2], split[3]); calls title_fn(title_area), body_fn(body_area), render_footer_hint(footer_area, hint), then overlay branch
  render_title_with_count is defined in views/agent/mode_view_render.rs (not the shell). The count wrapper re-expresses its title via the closure: |a,b| render_title_with_count(a,b,title,count,suffix)
  SearchHistoryView::render is &self (immutable), so the body/title closures capture &self with plain FnOnce; no &mut self needed (unlike ProviderSettingsView). title_fn = |a,b| render_title(self,a,b); body_fn = |a,b| render_body(self,a,b); local render_footer can be dropped in favor of the shell's render_footer_hint with the static hint string
  No insta snapshots cover SearchHistory; validation is by buffer-walking (tests/search_view_rpc064.rs walks Buffer cells for BOLD/REVERSED/placeholder). New/updated tests must assert the title row contains '(search):' + a REVERSED cell after the refit, plus body/footer parity
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The shell exposes render_full_screen_scaffold_with_title accepting a caller-supplied FnOnce(Rect, &mut Buffer) title closure, mirroring the Clear + [Length(1),Length(1),Min(0),Length(1)] split + optional ConfirmDialog overlay structure of render_full_screen_scaffold
  #   2. The count-title convenience wrapper render_full_screen_scaffold preserves the '{title} ({count} {suffix})' format (RPC-337 rule [4]) and is implemented in terms of the title-closure variant to avoid duplicating the Clear/split/overlay structure
  #   3. SearchHistoryView::render delegates to render_full_screen_scaffold_with_title; its render no longer hand-rolls Clear + the 4-constraint Layout
  #   4. The refit preserves SearchHistoryView's editable-query title: the title row still renders '(search): <query>' followed by an inverse (Modifier::REVERSED) cursor cell, NOT a count label
  #   5. SearchHistoryView's body and footer render identically after the refit: the body still shows the scroll-windowed match list / placeholder and the static footer hint 'Enter Select | ↑↓ Navigate | Esc Cancel'
  #   6. SearchHistoryView passes overlay = None to the shell (it has no destructive ConfirmDialog action)
  #   7. tests/rpc026_source_shape.rs's SearchHistory render-first-statement assertion is relaxed to accept the shell delegate (render_full_screen_scaffold_with_title) in addition to Clear.render, mirroring the resume_session relaxation; the 'deferred to RPC-339' comment is removed
  #   8. search_history_view.rs stays under 300 LoC after the refit (currently 264; the refit should shrink it by removing the hand-rolled split)
  #
  # EXAMPLES:
  #   1. Shell renders a view via the title-closure variant -> the supplied title_fn paints row 0, body_fn paints the body sub-rect, the static footer paints the last row, and no overlay is drawn when overlay is None
  #   2. Count-title wrapper still works: render_full_screen_scaffold called with title 'Resume Session', count 5, suffix 'available' produces a title row reading 'Resume Session (5 available)' (RPC-337 snapshot parity preserved)
  #   3. User types 'auth' into the search palette -> after the refit the title row still reads '(search): auth' with an inverse cursor cell immediately after
  #   4. SearchHistory with matching history entries -> body still renders the scroll-windowed list with the selected row REVERSED and the query substring BOLD-highlighted, identical to the pre-refit buffer output
  #   5. Empty query -> body still shows the centered placeholder '(type to search history)' and footer 'Enter Select | ↑↓ Navigate | Esc Cancel' after the refit
  #   6. Source-shape test runs after the refit -> it passes because SearchHistory's render first statement (the shell delegate) is now accepted, and search_history_view.rs is still under 300 LoC
  #
  # ASSUMPTIONS:
  #   1. search_history_view.rs keeps its local CHROME_ROWS const and visible_rows_for helper (value identical to shell's CHROME_ROWS=3); redirecting them to the shell is optional cleanup, not required for the refit
  #   2. render_full_screen_scaffold_raw_title (used by ModelSelector) is left unchanged; only render_full_screen_scaffold is re-expressed on top of the new closure variant
  #
  # ========================================
  Background: User Story
    As a Rust TUI developer
    I want to refit SearchHistoryView onto the shared full-screen shell via a title-renderer closure
    So that the third hand-rolled scaffold is unified without losing the editable query title

  Scenario: Refit preserves the editable query title
    Given a SearchHistoryView whose query is "auth"
    When the view is rendered through the shell title-closure variant
    Then the title row reads "(search): auth"
    And an inverse REVERSED cursor cell is painted immediately after the query

  Scenario: Refit preserves the body match list rendering
    Given a SearchHistoryView with matching history entries and a selected row
    When the view is rendered through the shell title-closure variant
    Then the body renders the scroll-windowed match list
    And the selected row is painted with the REVERSED modifier
    And the query substring is BOLD-highlighted within each matching row

  Scenario: Refit preserves the empty-query placeholder and footer
    Given a SearchHistoryView with an empty query
    When the view is rendered through the shell title-closure variant
    Then the body shows the centered placeholder "(type to search history)"
    And the footer row reads "Enter Select | ↑↓ Navigate | Esc Cancel"

  Scenario: Source-shape test accepts the refit delegate
    Given SearchHistoryView::render delegates to render_full_screen_scaffold_with_title as its first statement
    When the source-shape test in tests/rpc026_source_shape.rs runs
    Then the relaxed first-statement assertion accepts the shell delegate
    And search_history_view.rs remains under 300 lines of code
