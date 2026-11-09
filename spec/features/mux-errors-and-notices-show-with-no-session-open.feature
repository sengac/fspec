@done
@tui
@mux
@bug-fix
@rust
@BUG-647
Feature: Stale coverage into views/board/keys.rs after TUI-129, plus /mux errors dropped with no session
  """
  App::push_mux_error and App::push_mux_notice (app/dispatch_mux.rs) push into the current session's scrollback (MUX-001 R7); /mux slash commands only run in the current session (BUG-552), and if none is current an error goes through Action::SessionOperationFailed and a notice through a NotificationDialog::info that stays until dismissed. The mux config dialog's failed Save (Action::MuxConfigAppliedAndSaved in app/dispatch_mux.rs) always goes through Action::SessionOperationFailed (the Session error dialog TUI-129 uses for its notices). Tests drive App over MockBackend with the process data directory rooted in a temp dir (the bug167 pattern); a user fspec-config.json that is not JSON makes the save fail (BUG-461).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. With a current session, a /mux error or notice is still one line in that session's scrollback (MUX-001 R7)
  #   2. /mux slash commands only run in the session that is current (BUG-552 refuses them otherwise), so their errors and notices go to that session's scrollback; if no session is current when one is pushed, an error goes through Action::SessionOperationFailed and a notice through a NotificationDialog::info that stays until dismissed, never dropped
  #   3. A failed save from the mux config dialog (MuxConfigAppliedAndSaved) is an app-level error through Action::SessionOperationFailed whether or not a session is open: the dialog is used from the board, where no scrollback is visible
  #   4. The stale views/board/keys.rs coverage after TUI-129 (board-m-key, board-actions-popup, board-menu-bar-reorganize, tui-new-bot) is relinked by the coverage-repair work running alongside (BUG-582/BUG-668); BUG-647 checks with fspec audit that those four features report no coverage problem
  #
  # EXAMPLES:
  #   1. No session open, the mux config dialog commits Save and the user config is not JSON: the Session error dialog lists '/mux: could not save config: …'
  #   2. A session open, the same failed dialog Save: the Session error dialog lists it and the session's scrollback has no such line
  #   3. A session open, /mux banana: the parse error is the last scrollback line of that session and no dialog opens
  #
  # ========================================
  Background: User Story
    As a fspec TUI user running /mux or the mux config dialog
    I want to see every /mux error and notice even when no agent session is open
    So that a failed save, a refused layout or the help text is never silently dropped

  Scenario: With no session open a failed save from the mux config dialog shows as an app-level error
    Given no agent session is open and the user config file is not valid JSON
    When the mux config dialog commits its draft with Save
    Then the Session error dialog lists an error starting "/mux: could not save config:"

  Scenario: With a session open a /mux error stays in its scrollback
    Given an agent session is open and current
    When the user submits "/mux banana"
    Then the session's last scrollback line is the /mux parse error
    And no dialog opens

  Scenario: With a session open a failed save from the mux config dialog still shows as an app-level error
    Given an agent session is open and current and the user config file is not valid JSON
    When the mux config dialog commits its draft with Save
    Then the Session error dialog lists an error starting "/mux: could not save config:"
    And the session's scrollback has no line starting "/mux: could not save config:"
