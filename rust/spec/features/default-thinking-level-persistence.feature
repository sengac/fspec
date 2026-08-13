@done
@tui
@session-management
@session
@TUI-002
Feature: Persist and re-apply default thinking level so [T:High] badge shows on idle sessions

  """
  New module rust/sessions/src/default_thinking_level_persistence.rs mirrors default_model_persistence.rs: save/load _with_dir path-injectable cores + global convenience wrappers using codelet_common::get_data_dir. File default-thinking-level.json holds { level: u8 }. Load clamps/validates 0..=3, returns ThinkingLevel::Off otherwise.
  set_thinking_level_default (handle_impl.rs) persists via save_default_thinking_level(level) ALWAYS, then applies in-memory when the session exists. Session-creation paths (create_session_with_id + create_isolated_session_with_id) call session.set_base_thinking_level(load_default_thinking_level() as u8) right after BackgroundSession::new. Do NOT modify the badge renderer (header_build.rs thinking_label).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The default thinking level is persisted to a host config file under the data dir as an integer 0..=3 (0=Off,1=Low,2=Medium,3=High)
  #   2. Loading the default returns Off when the file is missing, malformed, or holds an out-of-range value (graceful degradation)
  #   3. set_thinking_level_default persists the level (always) in addition to applying it in-memory to the session when the session exists
  #   4. On session creation the persisted default is loaded and applied to the new session's base thinking level so the first idle render reflects it
  #   5. A persisted default of Off applies no badge (base level Off → thinking_label returns None)
  #   6. Server-side at the session-creation path (create_session_with_id / create_isolated_session_with_id) so both embedded and websocket transports inherit it for free, mirroring TS applying it on every new/resumed session.
  #
  # EXAMPLES:
  #   1. Saving High (3) then loading from the same data dir returns High
  #   2. Loading from a data dir with no config file returns Off
  #   3. A config file holding the value 7 (out of range) loads as Off
  #   4. set_thinking_level_default(High) writes the config so a subsequent load returns High even when no session exists
  #   5. A new session created while the persisted default is High has its base thinking level set to High, so the idle header renders [T:High]
  #   6. A new session created while the persisted default is Off keeps base level Off, so the idle header renders no [T:] badge
  #
  # QUESTIONS (ANSWERED):
  #   Q: Apply the persisted default server-side at session creation (all transports inherit) or client-side in dispatch_session_chrome?
  #   A: Server-side at the session-creation path (create_session_with_id / create_isolated_session_with_id) so both embedded and websocket transports inherit it for free, mirroring TS applying it on every new/resumed session.
  #
  # ========================================

  Background: User Story
    As a developer using the Rust ratatui AgentView
    I want to have my chosen default thinking level persisted and re-applied to every new session
    So that an idle session shows the yellow [T:High] badge just like the TypeScript reference

  Scenario: Saving the default thinking level round-trips from disk
    Given a data directory with no persisted default thinking level
    When the default thinking level High is saved to that data directory
    And the default thinking level is loaded from that data directory
    Then the loaded default thinking level is High

  Scenario: A missing config file loads as Off
    Given a data directory with no persisted default thinking level
    When the default thinking level is loaded from that data directory
    Then the loaded default thinking level is Off

  Scenario: An out-of-range persisted value loads as Off
    Given a data directory whose default thinking level config holds the value 7
    When the default thinking level is loaded from that data directory
    Then the loaded default thinking level is Off

  Scenario: set_thinking_level_default persists the level even with no live session
    Given a session manager rooted at a data directory with no persisted default
    When set_thinking_level_default is called with High for an unknown session
    And the default thinking level is loaded from that data directory
    Then the loaded default thinking level is High

  Scenario: A new session inherits a persisted High default
    Given a session manager rooted at a data directory whose persisted default thinking level is High
    When a new session is created
    Then the new session's thinking level is High

  Scenario: A new session inherits a persisted Off default
    Given a session manager rooted at a data directory whose persisted default thinking level is Off
    When a new session is created
    Then the new session's thinking level is Off
