@done
@config-management
@tui
@persistence
@TUI-092
Feature: Repoint default thinking-level persistence to shared fspec-config.json

  """
  Storage is delegated to the CONFIG-008 shared fspec_config module (load_config_with_dirs / write_config_with_dirs / ConfigScope) under nested key tui.defaultThinkingLevel; no dedicated file or DefaultThinkingLevelFile struct remains.
  Path-injectable *_with_dirs cores (data_dir + cwd) carry the logic; thin global wrappers resolve get_data_dir()/current_dir() and keep their existing signatures so session_manager.rs and handle_impl.rs callers stay edit-free.
  Save writes USER scope via read-modify-write preserving siblings; load relies on CONFIG-008 deep-merge so project scope overrides user. Save returns Result<_,String>; load is infallible (Off on any error).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Saving the default thinking level writes the integer (0-3) under the nested key tui.defaultThinkingLevel in the shared fspec-config.json
  #   2. Saving is a read-modify-write to USER scope that preserves all sibling keys already present in the config
  #   3. Loading reads tui.defaultThinkingLevel and maps 1=Low, 2=Medium, 3=High, 0=Off
  #   4. A missing, malformed, or out-of-range (>3) value loads as Off (load is infallible)
  #   5. A project-scope <cwd>/spec/fspec-config.json value overrides the user value on load (deep-merge)
  #   6. The global wrapper signatures save_default_thinking_level(level) and load_default_thinking_level() are unchanged so session_manager.rs and handle_impl.rs callers need no edits
  #
  # EXAMPLES:
  #   1. Saving High writes tui.defaultThinkingLevel=3 into <data_dir>/fspec-config.json
  #   2. After saving Medium, loading from the same data dir returns Medium
  #   3. Config already has tui.lastUsedModel; saving High keeps tui.lastUsedModel intact and adds tui.defaultThinkingLevel=3
  #   4. User config has tui.defaultThinkingLevel=1 but project spec/fspec-config.json has 3; load returns High
  #   5. Config holds tui.defaultThinkingLevel=7; load returns Off
  #   6. No config file present; load returns Off
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the old ~/.fspec/default-thinking-level.json be migrated into the shared config on first load, or simply abandoned?
  #   A: Option (a): No migration. The TS reference never had a default-thinking-level.json file, so there is no interop legacy to preserve. Users re-select once via the /thinking dialog. Simplest and avoids dead migration code.
  #
  # ========================================

  Background: User Story
    As a developer using the AI agent TUI
    I want to have my default thinking level persisted in the shared fspec-config.json under tui.defaultThinkingLevel
    So that it interoperates with the TypeScript fspec build and respects project-scope overrides

  Scenario: Saving High writes the nested tui.defaultThinkingLevel key
    Given a data directory with no persisted shared config
    When the default thinking level High is saved to the user scope
    Then the file fspec-config.json under the data directory contains tui.defaultThinkingLevel equal to 3

  Scenario: Saved level round-trips through the shared config
    Given a data directory with no persisted shared config
    When the default thinking level Medium is saved to the user scope
    And the default thinking level is loaded from the shared config
    Then the loaded default thinking level is Medium

  Scenario: Saving preserves pre-existing sibling keys
    Given a user shared config that already contains tui.lastUsedModel set to "anthropic/claude-opus-4-5"
    When the default thinking level High is saved to the user scope
    Then the shared config still contains tui.lastUsedModel equal to "anthropic/claude-opus-4-5"
    And the shared config contains tui.defaultThinkingLevel equal to 3

  Scenario: Project scope overrides the user value on load
    Given a user shared config with tui.defaultThinkingLevel set to 1
    And a project shared config with tui.defaultThinkingLevel set to 3
    When the default thinking level is loaded from the shared config
    Then the loaded default thinking level is High

  Scenario: An out-of-range value loads as Off
    Given a user shared config with tui.defaultThinkingLevel set to 7
    When the default thinking level is loaded from the shared config
    Then the loaded default thinking level is Off

  Scenario: A missing config file loads as Off
    Given a data directory with no persisted shared config
    When the default thinking level is loaded from the shared config
    Then the loaded default thinking level is Off

  Scenario: The global wrappers round-trip via the shared config
    Given the global data directory is rooted at a throwaway directory
    When the default thinking level High is saved via the global wrapper
    And the default thinking level is loaded via the global wrapper
    Then the loaded default thinking level is High
