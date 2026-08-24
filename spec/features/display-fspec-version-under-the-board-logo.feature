@done
@rust
@tui
@ui
@board-view
@header
@ui-enhancement
@BOARD-021
Feature: Display fspec version under the board logo

  """
  Implementation lives in rust/fspec-tui/src/views/board/logo.rs: the 4th entry of LOGO_ROWS becomes format!("v{}", env!("CARGO_PKG_VERSION")) and render() gains a theme parameter so the version row paints with theme.dim while the glyph rows keep Style::default(). Call site is views/board/header.rs (logo::render(left, buf, theme)).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The version string is sourced from env!("CARGO_PKG_VERSION") at compile time (the workspace version, e.g. 0.10.3) — no runtime file reads, no RPC/backend calls, no new BoardStore field.
  #   2. The version is painted on row index 3 (the 4th row) of the logo block — the currently blank row — as 'v' + version (e.g. 'v0.10.3'), left-aligned at the logo's left edge, styled with the theme's dim color.
  #   3. The existing 3 logo glyph rows, the right-hand header widgets (checkpoint status, divider, keybinding chord), and all BoardView row geometry are unchanged — the change is confined to the logo widget's 4th row.
  #   4. The version text is clipped to the logo block width (12 cells) — it must never overflow into the right-hand header column (checkpoint status / divider / keybinding chord).
  #
  # EXAMPLES:
  #   1. The version line appears on the same terminal row as the right-hand keybinding chord ('C Checkpoints ◆ F Changed Files ...'), i.e. the 4th row of the header strip, left-aligned at the logo's x position.
  #   2. With workspace version 0.10.3, the board header's 4th logo row contains the substring 'v0.10.3' while rows 1-3 still show the '┏┓┏┓┏┓┏┓┏┓' / '┣ ┗┓┃┃┣ ┃' / '┻ ┗┛┣┛┗┛┗┛' glyphs.
  #
  # ========================================

  Background: User Story
    As a developer using the fspec TUI
    I want to see the installed fspec build version in the board header
    So that confirm at a glance which binary is running without leaving the TUI or running fspec --version

  Scenario: Board header paints the build version on the 4th logo row
    Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    When the App renders BoardView against a 120x24 TestBackend
    Then the 4th row of the header strip (the row that also carries the keybinding chord) contains the substring "v" + env!("CARGO_PKG_VERSION")
    And that substring is centered within the 12-cell logo block, mirroring the centered glyph rows above it

  Scenario: Logo glyph rows are unchanged when the version row is painted
    Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring "┏┓┏┓┏┓┏┓┏┓"
    And the rendered buffer contains the substring "┣ ┗┓┃┃┣ ┃"
    And the rendered buffer contains the substring "┻ ┗┛┣┛┗┛┗┛"
    And the rendered buffer contains the substring "Checkpoints: None"
    And the rendered buffer contains the substring "C Checkpoints"

  Scenario: The version row is styled with the theme's dim color
    Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    And the Theme is the default dark variant
    When the App renders BoardView against a 120x24 TestBackend
    Then the buffer cells spelling the version string on the 4th logo row carry the theme's dim foreground color
    And the buffer cells spelling the 3 logo glyph rows carry the default (non-dim) foreground color

  Scenario: The version text never overflows the 12-cell logo block
    Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    When the App renders BoardView against a 120x24 TestBackend
    Then the version string occupies at most 12 cells starting at the logo's left edge
    And the keybinding chord on the 4th header row begins at the same x position as before (right after the 12-cell logo block)
