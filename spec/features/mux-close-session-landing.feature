@done
@bug
@keyboard-navigation
@bug-175
@tui
@mux
@BUG-175
Feature: Mux enabled=true in persisted config leaks into non-mux view routing — closing a session strands the user on a stale full-screen AgentView

  """
  View-routing decisions that gate on mux mode (BackToBoard landing + EnterWorkUnit flip) MUST read the LIVE view state (navigator.active_view == ViewMode::Mux), not mux.config().enabled. The persisted config flag survives across processes; the live view does not, so only the live view can answer 'are we in the grid right now'. BackToBoard: in the grid -> focus the Board pane within the grid (BUG-164 retained); any single view -> flip to the single Board view. The duplicated landing rule in App::dispatch and Navigator::apply_action MUST stay behaviorally identical (both gated on the live view).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Bootstrap MUST NOT leave the mux layout in an enabled state without entering ViewMode::Mux — loading a persisted tui.mux config with enabled=true must force enabled=false until the user explicitly re-enters mux mode
  #   2. Any view-routing decision that reads the mux enabled flag MUST base it on the live view state (ViewMode::Mux) rather than the persisted config flag, so the persisted enabled=true cannot leak routing behavior into non-mux single views
  #
  # EXAMPLES:
  #   1. Restart TUI with tui.mux.enabled=true persisted; user is on the single Board view; Esc on an Agent session with nothing open → BackToBoard MUST flip to the single Board view even though the persisted mux flag says enabled
  #   2. A persisted tui.mux block of enabled=true plus a saved Board|Agent 50/50 layout: on a fresh start the TUI boots on the single Board view with the mux layout DISABLED, and running /mux on re-enables the same saved 50/50 grid (the enabled flag is a saved layout preference, not a runtime mode)
  #
  # ========================================

  Background: User Story
    As a TUI user who saved a mux grid
    I want to close an agent session from single-view mode after a restart
    So that land on the board instead of a blank, unresponsive Agent view

  # =====================================================================
  # BUG-175: the persisted tui.mux.enabled flag is a saved layout
  # preference, not a runtime mode — it must never drive view routing
  # outside the mux grid.
  # =====================================================================

  @bug-175
  Scenario: bootstrap force-disables a persisted mux grid and keeps the saved layout for /mux on
    Given a fresh TUI bootstrap with a saved tui.mux config of Board and Agent at 50/50 with enabled true
    When the TUI starts
    Then the TUI is on the single Board view
    And the mux layout is disabled
    And when I submit the slash command "/mux on" the grid is the saved 50/50 Board | Agent layout

  @bug-175
  Scenario: back-to-board lands on the single Board view when the persisted mux flag is on but the grid is not active
    Given the TUI is on the single Agent view with one agent session open
    And the persisted tui.mux enabled flag is on
    When the agent session is closed
    Then the TUI is on the single Board view
    And Esc on the board opens the exit-fspec confirmation dialog

  @bug-175
  Scenario: Enter on a board work unit enters the single Agent view when the persisted mux flag is on but the grid is not active
    Given the TUI is on the single Board view with work unit AUTH-001 selected
    And the persisted tui.mux enabled flag is on
    When I press Enter
    Then the TUI is on the single Agent view

  @bug-175
  Scenario: /mux default enters the grid with the enabled flag in lockstep with the live view
    Given the TUI is on the single Board view with one agent session open
    When I submit the slash command "/mux default"
    Then the TUI is in mux mode
    And the mux enabled flag is on
