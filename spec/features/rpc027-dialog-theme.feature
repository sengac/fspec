@done
@refactor
@rust
@ui-refinement
@tui
@dialog
@rpc
@RPC-027
Feature: RPC-027 — Shared dialog_theme renderer fundamentals
  """
  RPC-027 Section A — the shared dialog_theme.rs renderer module.

  Companion features split per migrated dialog (1 feature ↔ 1 test file):
  spec/features/rpc027-help-disconnect-thinking-dialogs.feature
  spec/features/rpc027-model-confirm-dialogs.feature
  spec/features/rpc027-slash-file-popups.feature
  spec/features/rpc027-refactor-invariants.feature

  See spec/attachments/RPC-027/dialog-theme-refactor.md for the full
  style spec and per-dialog migration plan.
  """

  Background: User Story
    As a developer maintaining the rust/fspec-tui Rust ratatui frontend
    I want a single shared renderer that owns the canonical rounded / accent / inverse-highlight look
    So that every dialog and popup renders identically and a change to the look lands in one place

  Scenario: dialog_theme module exposes the canonical public API
    Given the file rust/fspec-tui/src/components/dialog_theme.rs exists
    When I inspect its public exports
    Then it exposes the Accent enum with variants Cyan, Yellow and Red
    And it exposes the DialogRow struct with spans, selectable, and selected fields
    And it exposes the FspecDialog struct with accent, title, rows, footer, and min_width fields
    And it exposes the render_dialog function
    And it exposes the dialog_rect function
    And it exposes the label_description_row helper
    And it exposes the MARKER_SELECTED constant equal to "▸ "
    And it exposes the MARKER_UNSELECTED constant equal to "  "
    And it exposes the FOOTER_SEPARATOR constant equal to " │ "

  Scenario: render_dialog paints a rounded border in the accent color
    Given an FspecDialog with accent Yellow, title "Test", a single body row "hello", and footer "esc"
    When I render it onto an 80x24 TestBackend buffer
    Then the top-left cell of the dialog rect contains "╭"
    And the top-right cell contains "╮"
    And the bottom-left cell contains "╰"
    And the bottom-right cell contains "╯"
    And the border cells have foreground color Color::Yellow

  Scenario: render_dialog paints an opaque black background over the dialog rect
    Given an FspecDialog with accent Cyan, title "Test", a single body row "hello", and footer ""
    When I render it onto an 80x24 TestBackend buffer
    Then every cell inside the dialog rect has background color Color::Black

  Scenario: render_dialog paints the inner title as a bold accent-colored row
    Given an FspecDialog with accent Yellow, title "Thinking Level", an empty rows vec, and footer ""
    When I render it onto an 80x24 TestBackend buffer
    Then the first non-padding body row contains the text "Thinking Level"
    And every cell of that text has foreground color Color::Yellow
    And every cell of that text carries the BOLD modifier
    And the title text is NOT rendered into the top border row

  Scenario: render_dialog inserts one blank row between title and body
    Given an FspecDialog with title "T" and a single body row containing "row0"
    When I render it onto an 80x24 TestBackend buffer
    Then the body row immediately after the title row contains only spaces
    And the body row two positions after the title contains the text "row0"

  Scenario: render_dialog paints the selected row with the full-width inverse highlight
    Given an FspecDialog with accent Yellow and three body rows where index 1 has selected = true
    When I render it onto an 80x24 TestBackend buffer
    Then every cell of the row at index 1 has background color Color::Yellow
    And every cell of that row has foreground color Color::Black
    And every cell of that row carries the BOLD modifier
    And the row at index 0 has the default background
    And the row at index 2 has the default background

  Scenario: label_description_row prefixes selected rows with "▸ " and others with "  "
    Given the helper label_description_row("Low", "fast", true)
    Then the resulting DialogRow's first span content is "▸ "
    Given the helper label_description_row("Low", "fast", false)
    Then the resulting DialogRow's first span content is "  "

  Scenario: label_description_row dims the description text for unselected rows only
    Given the helper label_description_row("Low", "fast", false)
    Then the description span carries Modifier::DIM
    Given the helper label_description_row("Low", "fast", true)
    Then the description span has the default Style (no DIM)

  Scenario: render_dialog paints the footer with Modifier::DIM and centered alignment
    Given an FspecDialog with footer "↑↓ Navigate │ Enter Select │ Esc Close"
    When I render it onto an 80x24 TestBackend buffer
    Then the last body row contains the substring "↑↓ Navigate"
    And every non-blank cell in that row carries the DIM modifier
    And the text is horizontally centered within the inner body width

  Scenario: dialog_rect centers the dialog within the available area
    Given an FspecDialog with content that yields a natural rect on an 80x24 area
    When I call dialog_rect(area, &dialog)
    Then the returned rect has x = (80 - rect.width) / 2
    And the returned rect has y = (24 - rect.height) / 2

  Scenario: dialog_rect clamps to the parent area when content exceeds it
    Given an FspecDialog with content that would yield a 100x30 rect on a 60x20 area
    When I call dialog_rect(area, &dialog)
    Then the returned rect has width <= 60 and height <= 20
    And the returned rect remains fully within the area bounds
