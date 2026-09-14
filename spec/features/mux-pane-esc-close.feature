@done
@bug
@keyboard-navigation
@bug-183
@mux
@tui
Feature: Mux: Esc on the Files/Checkpoints pane does nothing — it must close that pane (transient, like agent session close)
  """
  In mux mode the focused Changed Files or Checkpoints pane could never be
  dismissed: the views returned their Close outcome but the mux forwarder
  (views/multiplex/keys.rs forward_to_pane) mapped every non-Ignored
  outcome to a plain consumed(), so Esc was a dead key in the lazy panes
  (only /mux could reconfigure the grid).

  Fix: the forwarder translates the views' Close outcome onto the action
  bus (CloseChangedFilesView / CloseCheckpointsView) exactly like the
  single-view Navigator path (views/navigator_events.rs); App::dispatch
  then removes the pane kind (last occurrence) from the LIVE rendered
  pane list ONLY — the saved tui.mux config is untouched (transient:
  same rule as the agent slot dropping on session close, so /mux off then
  /mux on restores the full layout). Focus clamps to a surviving pane;
  if the removal leaves NO rendered panes at all (all-agent grid + zero
  sessions + the one rendered lazy pane closed), the grid exits to the
  single Board view (the pre_mux_view fallback — the /mux off rule).
  With at least one surviving pane the mux STAYS active, including a
  lone full-width Board pane (user decision 2026-09-14: do not
  auto-exit in that case).

  Esc mid-initial-load stays Ignored (TUI-107 — the views already return
  Ignored for a reflex Esc during the list stage), in-view dialogs
  (restore/delete) keep their Esc-cancels-dialog precedence, and the
  board-pane exit dialog (BUG-165) + the agent-pane exit cascade + the
  single-view Esc-close are unchanged (R4).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: Esc on the focused Changed Files or Checkpoints mux pane closes THAT PANE from the live rendered grid only: the pane kind is removed (last occurrence) from the live rendered pane list, the saved tui.mux config (persisted layout preference) is UNTOUCHED — transient, same rule as the agent slot dropping on session close, so /mux off then /mux on still restores the full layout. The mux stays in ViewMode::Mux, focus stays on a surviving pane (clamped to the new list), and the remaining panes rescale over the freed space via the existing rect recompute.
  #   2. R2: Esc stays IGNORED on a Files/Checkpoints mux pane while the view is mid-initial-load (the TUI-107 rule the views already enforce in single-view mode — a reflex Esc must not close a pane that has no data yet); the view returns Ignored and the key falls through to the App-level shortcuts exactly as today. In-view dialogs (restore/delete confirm) keep their precedence: their Esc cancels the dialog, it does NOT close the pane.
  #   3. R3: When the pane removal leaves no more panes (degenerate — e.g. an all-agent grid plus one lazy pane with zero open sessions: the agent slots are dropped, only the lazy pane renders, Esc closes it and nothing is left), the grid exits ViewMode::Mux back to the single Board view (pre_mux_view fallback = Board, the same rule /mux off uses). The normal close keeps the grid: with at least one surviving pane the mux stays active, and with only the Board pane remaining the grid shows the full-width Board pane in Mux (user decision — do NOT auto-exit in that case).
  #   4. R4: single-view and the other mux panes are unchanged — Esc in the single ChangedFiles/Checkpoints view still closes to the Board (existing CloseChangedFilesView/CloseCheckpointsView flip), Esc on the focused Board mux pane still pushes the BoardExitConfirmationDialog (BUG-165), and Esc on a focused Agent pane still opens the agent exit-confirmation cascade (Detach/Close Session/Cancel). The new pane-close path fires ONLY in ViewMode::Mux with the Files or Checkpoints pane focused.
  #
  # EXAMPLES:
  #   1. Mux grid Board|Agent|Files|Checkpoints with the Files pane focused: press Esc → the Files pane is gone, the grid is Board|Agent|Checkpoints, the saved tui.mux still lists all four panes, focus clamps to a surviving pane, the Checkpoints pane now absorbs the Files pane's share, and the mux footer shows 'MUX 3 panes [Board|Agent|Checkpoints]'.
  #   2. Mux grid Board|Checkpoints with one agent session open (so the grid actually renders Board|Agent|Checkpoints) — the user closes the agent session (Close Session) and then focuses the Checkpoints pane and presses Esc → the grid is Board|Agent and the Checkpoints pane is gone; later /mux off then /mux on brings the Checkpoints pane back from the saved layout.
  #
  # ========================================
  Background: User Story
    As a developer supervising agents in the fspec TUI mux mode
    I want to close a Files or Checkpoints mux pane with Esc
    So that the live grid shrinks exactly like closing an agent session, without losing the saved layout

  # R1: the focused Files pane closes; the grid shrinks, the saved
  # layout survives, the mux stays active.
  @bug-183
  Scenario: Esc on the focused Files mux pane closes the pane and keeps the saved layout
    Given mux mode is active with Board, Agent, Files and Checkpoints panes
    And the Files pane is focused and its list has loaded
    When I press the Esc key
    Then the grid shows Board, Agent and Checkpoints (the Files pane is gone)
    And the surviving panes rescale to absorb the Files pane's share of the width
    And the saved mux layout still lists the Files pane so a later /mux off then /mux on restores it
    And the TUI is still in mux mode with a surviving pane focused

  @bug-183
  Scenario: Esc on the focused Checkpoints mux pane closes the pane and keeps the saved layout
  # R1: same rule for the Checkpoints pane.
    Given mux mode is active with Board, Agent and Checkpoints panes and one agent session is open
    And the Checkpoints pane is focused and its list has loaded
    When I press the Esc key
    Then the grid shows Board and Agent (the Checkpoints pane is gone)
    And the saved mux layout still lists the Checkpoints pane so a later /mux off then /mux on restores it
    And the TUI is still in mux mode with a surviving pane focused

  @bug-183
  Scenario: Esc on a Files pane mid-initial-load does not close the pane
  # R2: a reflex Esc mid-load must not close a pane that has no data yet
  # (the TUI-107 rule the views already enforce).
    Given mux mode is active with Board and Files panes
    And the Files pane is focused and still in its initial load (the loading dialog is showing)
    When I press the Esc key
    Then the Files pane is still in the grid (the pane does not close mid-load)
    And the initial load completes un-impeded

  @bug-183
  Scenario: closing the last rendered pane exits mux to the single Board view
  # R3: closing the last rendered pane exits mux to the single Board view
  # (degenerate grid: all-agent slots dropped, only the lazy pane left).
    Given mux mode is active with Agent and Checkpoints panes and no agent sessions are open
    And only the Checkpoints pane renders (the agent slot is dropped) and its list has loaded
    When I press the Esc key
    Then the TUI leaves mux mode and shows the single Board view
    And the saved mux layout is unchanged

  @bug-183
  @regression
  Scenario: Esc on the focused Board mux pane still shows the exit dialog
  # R4 regression guards: the pre-existing Esc semantics on the other
  # panes / modes are untouched by the new pane-close path.
    Given mux mode is active with Board and Files panes and no agent sessions are open
    And the Board pane is focused
    When I press the Esc key
    Then the BoardExitConfirmationDialog is shown over the full screen
    And the TUI is still in mux mode with the same panes

  @bug-183
  @regression
  Scenario: Esc on the focused Agent mux pane still shows the agent exit dialog
    Given mux mode is active with Board and Agent panes and one agent session is open
    And the Agent pane is focused
    When I press the Esc key
    Then the agent exit confirmation dialog (Detach / Close Session / Cancel) is shown
    And the Agent pane is still in the grid

  @bug-183
  @regression
  Scenario: Esc in the single Changed Files view still closes to the Board
    Given the TUI is showing the single Changed Files view with mux inactive
    When I press the Esc key
    Then the TUI is back on the single Board view
    And no mux footer row is painted
