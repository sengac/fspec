@done
@bug
@BUG-205
@menu-bar
@agent-view
@dialog
@tui
@regression
Feature: Left-click activates the 'Close Agent?' dialog's Close and Cancel buttons
  """
  BUG-205 left the 'Close Agent?' ExitConfirmationDialog mouse-activated:
  left-clicking 'Close' commits the close (identical to
  Right,Enter-with-Close-Session-preselected) and left-clicking 'Cancel'
  dismisses the dialog without destroying the session. These scenarios pin
  the post-removal two-button click behavior (TUI-112 dialog-click
  activation, superseded by BUG-205 for the Close Agent dialog).

  Merged into left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc (TUI-112) by COV-075 (product owner, 2026-11-05): its two scenarios restated the 'Close Agent' click scenarios there on the same tests and were removed here.
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
  #   R2: Close is the default/pre-selected option (index 0). The
  #       dialog's yellow accent, footer, and ESC==Cancel semantics are
  #       unchanged.
  #
  # EXAMPLES:
  #   5. User left-clicks 'Close' in the 'Close Agent?' dialog →
  #      the close is committed, the session is destroyed, and the dialog
  #      is removed.
  #   6. User left-clicks 'Cancel' → the dialog dismisses and the session
  #      keeps running.
  #
  # ========================================
  Background: User Story
    As a TUI user closing an agent session
    I want left-click to activate the two-button exit dialog
    So that mouse users get the same close/cancel choices as the keyboard
