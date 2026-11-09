@BUG-708
@done
@tui
@mux
@bug-fix
@rust
@MUX-011
Feature: An unparseable saved mux layout silently falls back to the default
  """
  MuxState::load (store/mux_state.rs) returns the exact error instead of logging it at debug: the layout string is parsed on its own so the message names tui.mux.layout and the parser's 1-based column (MuxLayoutError); any other serde error of MuxConfig names tui.mux. codelet_sessions::mux_config_persistence::load_mux_config_with_dirs now returns a non-object tui.mux value instead of None so the TUI reports it. App::load_mux_config (app/state/state_views.rs) sends Action::SessionOperationFailed with the message and keeps the default config; nothing is written back, so the bad text stays on disk until a save. Tests root the process data directory in a temp dir (the bug167 pattern) and drive App::load_mux_config over MockBackend.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A saved tui.mux.layout that does not parse shows an app-level error (Action::SessionOperationFailed) naming tui.mux.layout, the 1-based column and the problem, e.g. 'tui.mux.layout in fspec-config.json: unbalanced bracket: '[' at column 8 is never closed; using the default layout this session' (product owner, 2026-11-08)
  #   2. After that error the default layout is used for this session; the bad text stays on disk until the user fixes it or saves a new layout (/mux save, the mux config dialog's Save or the mux-exit auto-save) (product owner, 2026-11-08)
  #   3. Any other unreadable tui.mux value (not an object, a missing or unknown field, a wrong type) shows the same kind of exact error naming tui.mux and the problem; never a silent default
  #   4. No saved tui.mux at all uses the default layout with no error (nothing was saved)
  #   5. The error shows whether or not an agent session is open (app-level, not a scrollback line)
  #
  # EXAMPLES:
  #   1. fspec-config.json has tui.mux = {layout: 'board, [agent', focused_pane: 0, enabled: false}: bootstrap shows the Session error dialog with 'tui.mux.layout in fspec-config.json: unbalanced bracket: '[' at column 8 is never closed; using the default layout this session', the live layout is 'board, agent' and the file still holds 'board, [agent'
  #   2. tui.mux.layout 'board, banana': the error names column 8 and unknown pane 'banana'
  #   3. tui.mux = 'board' (a string): the error names tui.mux and says it is not an object
  #   4. tui.mux has an extra field 'orientation2': the error names tui.mux and the unknown field
  #   5. No tui.mux key: no error, the default layout 'board, agent'
  #
  # ========================================
  Background: User Story
    As a fspec TUI user with a hand-edited mux layout
    I want to be told exactly what is wrong with a saved tui.mux that cannot be read
    So that the TUI never silently swaps my layout for the default

  Scenario: An unparseable saved layout shows the key, the column and the problem
    Given the user fspec-config.json holds tui.mux with the layout "board, [agent"
    When the TUI loads the saved mux config at startup
    Then the Session error dialog lists "tui.mux.layout in fspec-config.json: unbalanced bracket: '[' at column 8 is never closed; using the default layout this session"
    And the live mux layout is "board, agent"
    And the user fspec-config.json still holds the layout "board, [agent"

  Scenario: An unknown pane in the saved layout names its column
    Given the user fspec-config.json holds tui.mux with the layout "board, banana"
    When the TUI loads the saved mux config at startup
    Then the Session error dialog lists an error naming tui.mux.layout, column 8 and the unknown pane 'banana'

  Scenario: A saved tui.mux that is not an object shows an error
    Given the user fspec-config.json holds tui.mux as the string "board"
    When the TUI loads the saved mux config at startup
    Then the Session error dialog lists an error naming tui.mux
    And the live mux layout is "board, agent"

  Scenario: A saved tui.mux with an unknown field shows an error
    Given the user fspec-config.json holds tui.mux with an extra field "orientation2"
    When the TUI loads the saved mux config at startup
    Then the Session error dialog lists an error naming tui.mux and the unknown field "orientation2"

  Scenario: No saved tui.mux uses the default layout without an error
    Given the user fspec-config.json has no tui.mux key
    When the TUI loads the saved mux config at startup
    Then no Session error dialog opens
    And the live mux layout is "board, agent"

  Scenario: An unreadable fspec-config.json shows its error and the default layout
    Given the user fspec-config.json holds the text "{ not json"
    When the TUI starts and loads the mux config
    Then the Session error dialog lists one error naming fspec-config.json as invalid JSON and saying the default layout is used this session
    And the mux layout is "board, agent"
    And fspec-config.json still holds "{ not json"
