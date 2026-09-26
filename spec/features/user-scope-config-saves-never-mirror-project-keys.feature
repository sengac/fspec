@done
@BUG-193
@bug
@bug-fix
@persistence
@config
@config-management
@tui
Feature: TUI user-scope config writers mirror project-scope keys (tools, agent) into ~/.fspec/fspec-config.json
  """
  Architecture notes:
  - Fix location: the two save cores in codelet-sessions (save_mux_config_with_dirs in
  mux_config_persistence.rs, save_default_thinking_level_with_dirs in
  default_thinking_level_persistence.rs) currently read via
  codelet_common::fspec_config::load_config_with_dirs (deep-merged user+project view) and
  write the WHOLE merged Value back to ConfigScope::User. Fix: a new helper
  load_user_config_file(data_dir) in codelet_common::fspec_config (missing/empty ->
  empty object, invalid JSON -> Err, same semantics as load_config_file) replaces the
  merged read in the WRITE half of both save cores; the read/load halves keep the
  deep-merged view unchanged. Load paths (load_mux_config_with_dirs,
  load_default_thinking_level_with_dirs) are untouched.
  - Integration points (WHO CALLS THIS): MuxState::save in codelet-fspec-tui
  (store/mux_state.rs) -> save_mux_config (global wrapper, resolves data dir + cwd) ->
  save_mux_config_with_dirs. ThinkingLevel default save -> save_default_thinking_level
  (global wrapper) -> save_default_thinking_level_with_dirs. Both live in
  codelet-sessions and are shared by the fspec binary, NAPI, and embedded TUI
  transports. The cwd parameter becomes unused in the write path after the fix; keep
  it in the *_with_dirs signature for call-site stability but document that it is
  only used for the merged LOAD, not the save.
  - Root cause detail and full write-path inventory: spec/attachments/BUG-193/config-scope-contamination-research.md
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. User-scope config writes (mux save, default-thinking-level save) must read the RAW user file before writing back to user scope — never the deep-merged project-over-user view
  #   2. A user-scope save must never introduce keys that belong only to project scope (tools, agent) into ~/.fspec/fspec-config.json
  #
  # EXAMPLES:
  #   1. A TUI session opened in a Java project (spec/fspec-config.json holds agent='claude' + tools.test) saves the mux layout; afterwards ~/.fspec/fspec-config.json contains the new tui.mux value and still contains providers/rlcd, but does NOT contain 'tools' or 'agent'
  #   2. Repro: open the fspec TUI in a repo whose spec/fspec-config.json has tools+agent, exit a mux session (auto-save fires). Before the fix the global ~/.fspec/fspec-config.json grows the repo's 'tools' and 'agent' keys. After the fix it does not.
  #
  # ========================================
  Background: User Story
    As a fspec TUI user
    I want to save my TUI preferences (mux layout, thinking level) without my per-project settings leaking into my global config
    So that my ~/.fspec/fspec-config.json stays clean of project-specific state across all my repos

  # ========================================
  # SCENARIOS
  # ========================================
  @integration
  Scenario: Saving the mux config does not mirror project-scope keys into the user config
    Given the user-scope fspec-config.json holds providers and rlcd sections
    And the project spec/fspec-config.json holds agent and tools.test keys
    When the mux config is saved to the user scope
    Then the user-scope fspec-config.json contains the saved tui.mux value
    And the user-scope fspec-config.json still contains the providers and rlcd sections
    And the user-scope fspec-config.json does NOT contain the tools key
    And the user-scope fspec-config.json does NOT contain the agent key

  @integration
  Scenario: Saving the default thinking level does not mirror project-scope keys into the user config
    Given the project spec/fspec-config.json holds agent and tools keys
    When the default thinking level High is saved to the user scope
    Then the user-scope fspec-config.json contains tui.defaultThinkingLevel equal to 3
    And the user-scope fspec-config.json does NOT contain the tools key
    And the user-scope fspec-config.json does NOT contain the agent key

  Scenario: A user-scope mux save preserves user-scope siblings
    Given the user-scope fspec-config.json holds tui.lastUsedModel and tui.defaultThinkingLevel
    When the mux config is saved to the user scope
    Then the user-scope fspec-config.json contains the saved tui.mux value
    And the user-scope fspec-config.json still contains tui.lastUsedModel unchanged
    And the user-scope fspec-config.json still contains tui.defaultThinkingLevel unchanged

  Scenario: Saving to a user scope without any user config file still succeeds
    Given the user-scope directory has no fspec-config.json file
    And the project spec/fspec-config.json holds agent and tools keys
    When the mux config is saved to the user scope
    Then the user-scope fspec-config.json is created with only the tui.mux value
    And the user-scope fspec-config.json does NOT contain the tools or agent keys

  Scenario: A malformed user config file degrades to a fresh save without mirroring project keys
    Given the user-scope fspec-config.json contains invalid JSON
    And the project spec/fspec-config.json holds agent and tools keys
    When the mux config is saved to the user scope
    Then the user-scope fspec-config.json contains the saved tui.mux value
    And the user-scope fspec-config.json does NOT contain the tools or agent keys

  Scenario: Project-scope tui.mux still overrides the user-scope value on load
    Given the user-scope fspec-config.json holds tui.mux with orientation Horizontal
    And the project spec/fspec-config.json holds tui.mux with orientation Vertical
    When the mux config is loaded for the project
    Then the loaded mux orientation is Vertical
