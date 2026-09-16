@done
@MUX-009
@tui
@ui-enhancement
@keyboard-navigation
@dialog
@board-view
@mux
@mux-009
Feature: Board 'M' key opens the Mux config dialog + top-of-screen Mux hint

  """
  New `Action::OpenMuxConfigDialog` variant (components/mod.rs Action enum, near the MuxConfigApplied variants) emitted by BoardView::handle_event on modifier-free 'm'/'M' (guard: no CONTROL modifier, mirroring the a/c/f/d guards). App::dispatch adds an arm (app/dispatch_mux_config.rs) that calls the existing handle_open_mux_config_dialog() — idempotent, seeds from navigator.mux.config(). BoardView::handle_event gains the binding in the same match as 'f'/'c' (RPC-356/RPC-364 pattern). views/board/keybinding_shortcuts.rs appends ' ◆ M Mux' to the literal chord string. No new state; MuxConfig serde shape unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R2: the binding is modifier-free only — Ctrl+M (or any Ctrl-chorded m/M) must NOT open the dialog and falls through to the existing App-level handling, consistent with the existing 'a'/'c'/'f'/'d' guard pattern in BoardView::handle_event.
  #   2. R1: pressing modifier-free 'm' or 'M' in the single Board view (ViewMode::Board) opens the MuxConfigDialog via the existing App::handle_open_mux_config_dialog path (MUX-004: idempotent, seeded from the live mux config). The key is always consumed (no fall-through).
  #   3. R4: the BoardView's top-of-screen keybinding hint chord (views/board/keybinding_shortcuts.rs, row 3 of the board header: 'C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search') appends the new binding, rendering as 'C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux'. The hint stays a single plain primary-fg span (no per-chord styling change).
  #   4. R3: when mux is active (ViewMode::Mux) and the focused pane is the Board pane, the same modifier-free 'm'/'M' opens the MuxConfigDialog (the dialog overlays the grid per MUX-001 R9; Board pane keyboard input routes through BoardView::handle_event via views/multiplex/keys.rs, so the single binding serves both surfaces).
  #
  # EXAMPLES:
  #   1. In the single Board view (mux off), the user presses 'm'. The MuxConfigDialog opens (Priority::Foreground, overlaying the board) seeded from the live mux config — 'Enabled: Off', 'Orientation: Horizontal', 'Pane 1: Board', 'Pane 2: Agent' — exactly as /mux opens it from the AgentView. The board view is still visible underneath; mux is not yet active.
  #   2. In the single Board view, the user presses Ctrl+M. The MuxConfigDialog does NOT open — Ctrl-chorded m/M falls through the BoardView binding untouched (the modifier-free-only guard mirrors the existing a/c/f/d guards).
  #   3. With mux active (Board | Agent grid) and the Board pane focused, the user presses 'M'. The MuxConfigDialog opens over the whole grid (the dialog overlays the mux per MUX-001 R9) with the live enabled layout — 'Enabled: On' and the current pane rows. Pressing 's' saves + closes and the grid keeps the committed layout; pressing Esc cancels and the live grid is untouched.
  #   4. In the single Board view, the board header's top chord row reads 'C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux' — the new 'M Mux' chord is appended after the existing chords, same plain foreground styling.
  #
  # ========================================

  Background: User Story
    As a developer working from the Board view
    I want to open the mux configuration dialog with the 'M' key and see it advertised in the Board's top hint chord
    So that turn mux on/off and reconfigure the layout without switching to an agent session

  @MUX-009 @component @feature-group
  Scenario: M opens the Mux config dialog from the single Board view
    Given I am in the single Board view with mux disabled
    When I press the 'm' key
    Then the MuxConfigDialog is open overlaying the board
    And the dialog is seeded from the live mux config showing Enabled Off and the current pane rows
    And the board view is still visible underneath
    And mux is still not active

  @MUX-009 @component @feature-group
  Scenario: Ctrl+M does not open the Mux config dialog
    Given I am in the single Board view with mux disabled
    When I press Ctrl+M
    Then the MuxConfigDialog is not open
    And the key falls through to the App-level handling

  @MUX-009 @component @feature-group
  Scenario: M opens the Mux config dialog from the focused Board pane in mux mode
    Given I am in mux mode with the Board | Agent grid and the Board pane focused
    When I press the 'M' key
    Then the MuxConfigDialog is open overlaying the whole grid
    And the dialog shows the live enabled layout with Enabled On and the current pane rows
    And pressing 's' applies and persists the draft and closes the dialog with the grid keeping the committed layout
    And pressing Esc closes the dialog without applying or saving

  @MUX-009 @component @feature-group
  Scenario: The Board header top chord row advertises the M Mux binding
    Given I am in the single Board view
    When I look at the board header's keybinding chord row
    Then the chord row reads "C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux"
    And the chord is painted as a single plain foreground span
