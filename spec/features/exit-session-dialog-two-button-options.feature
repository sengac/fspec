@done
@bug
@BUG-205
@menu-bar
@agent-view
@dialog
@tui
@regression
Feature: The 'Close Agent?' dialog offers exactly two options (Close / Cancel) with keyboard semantics
  """
  BUG-205 removed the pointless 'Detach' option from the 'Close Agent?'
  ExitConfirmationDialog (opened by the agent bar's 'Close' button and by
  the ESC cascade level 7). Detach only dispatched Action::BackToBoard — i.e.
  navigate away while the session keeps running — which plain navigation
  (menu bar Board item, Shift+Left, mux board-pane focus) already does.

  The dialog is reduced to the two flat options [Close, Cancel] with
  Close pre-selected (index 0), matching the board 'Exit fspec?'
  BoardExitConfirmationDialog convention (destructive option pre-selected,
  Esc cancels). This feature pins the keyboard-side behavior: dialog open,
  cyclic Left/Right navigation, Enter on the pre-selected Close,
  Right+Enter committing Cancel, and ESC == Cancel.

  Merged into agentview-esc-exit-confirmation-dialog (RPC-098) by COV-075 (product owner, 2026-11-05): its five scenarios restated scenarios RPC-098 already specifies on the same tests, so they were removed here; the implementation mappings moved with them.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   R1: The 'Close Agent?' ExitConfirmationDialog offers exactly two flat
  #       options in order [Close, Cancel]. The Detach option is
  #       removed — it only dispatched Action::BackToBoard, which plain
  #       navigation already does. ExitChoice::Detach is deleted from the
  #       ExitChoice enum.
  #   R2: Close is the default/pre-selected option (index 0), matching
  #       the board 'Exit fspec?' convention of pre-selecting the destructive
  #       option. Left from Close wraps to Cancel; Right from Cancel
  #       wraps to Close. The dialog's busy/idle description variants,
  #       yellow accent, footer, and ESC==Cancel semantics are unchanged.
  #
  # EXAMPLES:
  #   1. User presses ESC in an idle AgentView (L7) → the 'Close Agent?'
  #      dialog shows exactly two buttons [Close, Cancel] with
  #      'Close' pre-selected (blue bg/white/bold), 'Cancel' in gray;
  #      the 'Detach' label is nowhere painted.
  #   2. Dialog open with Close pre-selected → Enter destroys the
  #      backend session, clears the BoardStore attachment, and returns to
  #      the Board view.
  #
  # ========================================
  Background: User Story
    As a TUI user closing an agent session
    I want to see a two-button exit dialog (Close / Cancel)
    So that the exit dialog no longer offers a pointless option that duplicates plain navigation
