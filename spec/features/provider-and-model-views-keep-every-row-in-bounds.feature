@done
@tui
@provider-settings
@bug-fix
@rust
@BUG-578
Feature: Provider and model views keep every row in bounds
  """
  Provider Settings (views/provider_settings) and /model (views/model_selector) in codelet-fspec-tui. F18: body_render sets visible_rows from the list rect after the status line; render_list now also takes the filter row off before the window is reconciled (adjust_scroll at render time, as /model does). F19: one clamp after every nav rebuild plus a defensive clamp in the nav renderer. F20: both views route Drag/Up to the ScrollbarDrag while it is active, and a drag moves the selection into the new window instead of letting ensure_visible pull the window back. F21: row_render fits a long label with an ellipsis, keeping a profile's BUG-547 missing-limits tail; profile_form_render scrolls the field lines around the focused field and pins the status. F22 (product owner, 2026-11-09): the Summary/OAuthNotice detail views, RefreshProviderModels, TestProviderConnection and test_result are deleted with their scenarios. F23: model_selector Esc in browse mode clears a committed filter first; an empty filtered list says No models match '<filter>'.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. F18: the Provider Settings list window counts only the rows painted for the list, after the filter row and the status line, and is reconciled when it renders, so the selected row is always painted
  #   2. F19: a reload that shrinks the Provider Settings tree clamps the scroll offset and the selection into the new tree; rendering never slices past its end
  #   3. F20: in Provider Settings and /model, dragging the scrollbar thumb moves the window with the thumb and keeps it there: the selection moves to the nearest row inside the window (a model row in /model); a drag that started on the thumb goes on when the pointer leaves the gutter column, until the button is released
  #   4. F21: a row label too long for its row ends in '…'; a profile row keeps its '— missing …' limits whole and shortens the name and URL part instead
  #   5. F21: the profile and settings forms scroll so the focused field stays visible on a short terminal, and the status line (a save error) stays pinned under the fields
  #   6. F22 (product owner, 2026-11-09): the dead Provider Settings paths are deleted with the scenarios that cover them: the Summary and OAuthNotice detail views, RefreshProviderModels, TestProviderConnection and test_result
  #   7. F23: in /model browse mode with a committed filter, Esc clears the filter first and a second Esc closes; a filter with no matches shows 'No models match <filter>'. (The filter reset on open and the /model cursor clamp were done by BUG-576.)
  #
  # EXAMPLES:
  #   1. 80x12 with the filter 'o' committed and the cursor moved to the last row: the selected row is painted on screen
  #   2. The cursor on the last painted row when '✓ saved' arrives: the next frame still paints the selected row
  #   3. 40 nav rows with the window at offset 30, then a reload with 5 rows: the next render paints without a panic, the offset is 0 and the selection is on the last row
  #   4. 60 Provider Settings rows, selection on row 0: pressing on the thumb and dragging to the bottom of the gutter leaves the window at the bottom after the next render, with the selection inside it
  #   5. /model with 60 rows: dragging the thumb down moves the window and the selection lands on a model row inside it
  #   6. A drag that started on the thumb and moves one column left of the gutter still scrolls; releasing the button anywhere ends it
  #   7. Profile 'local → http://192.168.1.50:8080/v1 — missing context window, max output tokens' at width 60 shows '— missing context window, max output tokens' whole, with the URL part ending in '…'
  #   8. The profile form at 80x15 with the last field focused and the status 'Context window must be a number': both the focused field and the status are on screen
  #   9. In /model, type /gpt and Enter, then Esc: the filter is cleared and /model stays open; Esc again closes it
  #   10. In /model, the filter 'zzz' matches nothing: the body shows No models match 'zzz'
  #
  # ========================================
  Background: User Story
    As a user of the Provider Settings and /model views
    I want to have both views keep the cursor and every row painted, scroll by the scrollbar, and show no dead paths
    So that the views never hide the cursor, crash on a reload, cut off what matters or offer actions that do nothing

  Scenario: The selected row stays painted with a committed filter
    Given Provider Settings at 80x12 with more rows than fit and the filter "o" committed
    When the user moves the cursor to the last row
    Then the selected row is painted on screen

  Scenario: The selected row stays painted when a status line arrives
    Given Provider Settings at 80x12 with the cursor on the last painted row
    When the status "✓ saved" arrives
    Then the next frame still paints the selected row

  Scenario: A reload that shrinks the tree never slices past its end
    Given Provider Settings with 40 rows, the window at offset 30 and the cursor on row 35
    When a reload leaves 5 rows
    Then the next render paints without a panic
    And the window offset is 0 and the selection is on the last row

  Scenario: Dragging the Provider Settings scrollbar thumb moves the window and keeps it there
    Given Provider Settings with 60 rows and the selection on row 0
    When the user presses on the scrollbar thumb and drags it to the bottom of the gutter
    Then after the next render the window shows the last rows
    And the selection is inside the window

  Scenario: Dragging the /model scrollbar thumb moves the window and selects a model row inside it
    Given /model with 60 rows and the selection on the first model
    When the user presses on the scrollbar thumb and drags it to the bottom of the gutter
    Then after the next render the window shows the last rows
    And the selection is a model row inside the window

  Scenario: A drag that leaves the gutter column goes on until the button is released
    Given a scrollbar drag started on the Provider Settings thumb
    When the pointer moves one column left of the gutter and further down
    Then the window still follows the pointer
    And releasing the button there ends the drag

  Scenario: A long profile row keeps its missing limits whole
    Given the openai profile "local" with base URL "http://192.168.1.50:8080/v1" and no context window or max output tokens
    When Provider Settings renders its row at width 60
    Then the row shows "— missing context window, max output tokens" whole
    And the name and URL part ends in "…"

  Scenario: A short terminal shows the focused profile field and the save error
    Given the profile form at 80x15 with the last field focused
    And the status "Context window must be a number"
    When the form renders
    Then the focused field is on screen
    And the status is on screen under the fields

  Scenario: Esc in /model clears a committed filter before it closes
    Given /model with the filter "gpt" committed in browse mode
    When the user presses Esc
    Then the filter is cleared and /model stays open
    And a second Esc closes /model

  Scenario: A filter that matches nothing says so
    Given /model with models loaded
    When the user filters by "zzz"
    Then the body shows "No models match 'zzz'"

  Scenario: The dead Provider Settings paths are gone
    Given the fspec-tui source outside tests
    When it is scanned for the Provider Settings paths nothing reaches
    Then it has no Summary or OAuthNotice detail view and no DetailStatus
    And it has no RefreshProviderModels, ProviderModelsRefreshed, TestProviderConnection or ProviderTestComplete action
    And it has no inline test_result decoration
    And the backend still offers test_provider_connection, which the Paseo plugin uses
    And Enter on Provider Settings with providers but no rows built stays in the list
