@done
@bug
@BUG-205
@menu-bar
@agent-view
@dialog
@mux
@tui
@regression
Feature: Close Session in mux mode still retains the mux and focuses the board pane (post-BUG-205 regression)
  """
  BUG-205 regression pin: with the two-button 'Exit Session?' dialog
  (Close Session / Cancel), answering Close Session in mux mode destroys the
  session but must NOT flip the grid to a single view — the mux stays on with
  the same panes and layout, and focus lands on the Board pane. (Originally
  specified by BUG-164 in rust-mux-mode.feature; re-pinned here against the
  post-removal two-button dialog.)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   R5: Closing a session in mux mode retains the mux grid (same panes,
  #       same layout) and focuses the Board pane; no single-view flip to
  #       Board occurs.
  #
  # EXAMPLES:
  #   7. Mux mode with Board + Agent panes, two agent sessions open, Agent
  #      pane focused → answer the exit dialog with Close Session → the
  #      destroyed session is removed, the grid survives, Board pane focused.
  #
  # ========================================
  Background: User Story
    As a TUI user closing an agent session in mux mode
    I want the mux grid to survive the close
    So that I do not lose my layout when I close a session

  @regression
  Scenario: Close Session in mux mode still retains the mux and focuses the board pane
    Given mux mode is active with Board and Agent panes and two agent sessions are open
    And the Agent pane is focused
    When the exit dialog is answered with Close Session
    Then the destroyed session is removed from the open-session list
    And the TUI is still in mux mode with the same panes and layout
    And the Board pane is focused within the grid
