@done
@RPC-026
@rust
@tui
@agent-view
@command-history
Feature: RPC-026 SearchPalette popup — typeahead over command history

  """
  RPC-026 (search palette slice) — a centred floating overlay rendered
  above AgentView's MultiLineInput. Shown when the user picks the
  /search slash command. The palette typeahead-filters across the
  command history matches returned by the backend's
  persistence_search_history RPC method (lifted in RPC-025).

  The widget itself owns three pieces of state:
    - query: String (the filter text; chars/backspace edit it).
    - matches: Vec<HistoryMatch> (refreshed via set_matches).
    - selected_index: usize (clamped to matches.len()).

  Each typing event emits `set_query(new)` and the App layer is
  expected to react by spawning a fresh persistence_search_history
  call. Selection ↑/↓ wraps around; Enter emits
  SearchPaletteOutcome::Selected(text) with the highlighted match's
  text; Esc dismisses.

  File: codelet/fspec-tui/src/views/agent/search_palette.rs (< 300 LoC).
  Owned by AgentView as `Option<SearchPalette>`.

  Tests: codelet/fspec-tui/tests/search_palette_widget_rpc026.rs.
  """

  Background: User Story
    As a fspec TUI user
    I want to type a query in the /search palette and pick a past command
    So that I can recall something I submitted earlier without scrolling through history

  Scenario: A new SearchPalette has empty query and no matches
    Given a fresh SearchPalette
    Then search_palette.query() equals ""
    And search_palette.match_count() equals 0
    And search_palette.selected_index() equals 0
    And search_palette.selected() returns None

  Scenario: set_query updates the filter text and resets selection to the first row
    Given a fresh SearchPalette
    When set_query("git") is called
    Then search_palette.query() equals "git"
    And search_palette.selected_index() equals 0

  Scenario: set_matches populates the typeahead rows and clamps selection
    Given a SearchPalette where set_query("git") has been called
    When set_matches is called with three HistoryMatch values [text="git status", text="git push", text="git diff"]
    Then search_palette.match_count() equals 3
    And search_palette.selected_index() equals 0
    And search_palette.selected() returns Some(HistoryMatch with text "git status")

  Scenario: set_matches with fewer rows than the current selection clamps the index
    Given a SearchPalette with three matches and selected_index == 2
    When set_matches is called with one match [text="git status"]
    Then search_palette.match_count() equals 1
    And search_palette.selected_index() equals 0

  Scenario: Down arrow advances selection and wraps around at the end
    Given a SearchPalette populated with three matches
    When the user presses Down
    Then search_palette.selected_index() equals 1
    When the user presses Down
    Then search_palette.selected_index() equals 2
    When the user presses Down
    Then search_palette.selected_index() equals 0

  Scenario: Up arrow walks backward and wraps to the last row
    Given a SearchPalette populated with three matches
    When the user presses Up
    Then search_palette.selected_index() equals 2

  Scenario: Typing a printable character appends it to the query and emits FilterChanged
    Given a fresh SearchPalette
    When the user presses 'g'
    Then handle_key returns SearchPaletteOutcome::FilterChanged("g")
    And search_palette.query() equals "g"

  Scenario: Backspace removes the last character from the query and emits FilterChanged
    Given a SearchPalette where set_query("git") has been called
    When the user presses Backspace
    Then handle_key returns SearchPaletteOutcome::FilterChanged("gi")
    And search_palette.query() equals "gi"

  Scenario: Backspace on an empty query is a no-op
    Given a fresh SearchPalette with empty query
    When the user presses Backspace
    Then handle_key returns SearchPaletteOutcome::Continued
    And search_palette.query() equals ""

  Scenario: Enter on a highlighted match emits Selected with the match text
    Given a SearchPalette populated with [text="git status", text="git push"]
    When the user presses Down
    And the user presses Enter
    Then handle_key returns SearchPaletteOutcome::Selected("git push")

  Scenario: Enter on zero matches is ignored
    Given a SearchPalette where set_query("xyzzy") has been called and matches is empty
    When the user presses Enter
    Then handle_key returns SearchPaletteOutcome::Ignored

  Scenario: Esc on the popup returns Dismiss
    Given a SearchPalette populated with one match
    When the user presses Esc
    Then handle_key returns SearchPaletteOutcome::Dismiss

  Scenario: Modifier-prefixed keys are propagated so AgentView can route Shift+arrow chords
    Given a SearchPalette populated with two matches
    When the user presses Shift+Down
    Then handle_key returns SearchPaletteOutcome::Ignored
    And search_palette.selected_index() is unchanged at 0

  Scenario: Empty query renders the "(type to search history)" placeholder
    Given a fresh SearchPalette with empty query
    When the popup is rendered
    Then the rendered body contains the literal string "(type to search history)"

  Scenario: Non-empty query with zero matches renders the "(no history matches "<query>")" placeholder
    Given a SearchPalette where set_query("xyzzy") has been called and matches is empty
    When the popup is rendered
    Then the rendered body contains the literal string "(no history matches \"xyzzy\")"

  Scenario: Populated matches render one row per HistoryMatch with the navigation hint
    Given a SearchPalette populated with [text="git status", text="git push"]
    When the popup is rendered
    Then the rendered body contains a row referencing "git status"
    And the rendered body contains a row referencing "git push"
    And the rendered body contains the navigation hint "↑↓ Navigate │ Enter Insert │ Esc Close"
