@done
@persistence
@model-selection
@session-management
@session
@PROV-119
Feature: Default model selection is not persisted across restarts

  """
  Persist to <data_dir>/default-model.json (JSON {model}) resolved via codelet_common::get_data_dir, mirroring the credentials writer path convention; isolates under the test tempdir set by set_data_directory
  SessionManager::set_default_model persists best-effort after the in-memory write; SessionManager::new loads the persisted value at construction so a fresh process is pre-populated
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A non-empty default model set via set_default_model is persisted to the user config store on disk
  #   2. On SessionManager construction the previously persisted default model is loaded into the in-memory default_model field
  #   3. Empty or whitespace-only model strings are never persisted (PROV-101 invariant preserved)
  #   4. Persistence failures are non-fatal: a failed write is logged but never panics or blocks session creation
  #   5. A missing or malformed config file loads as None (graceful degradation, unchanged pre-PROV-119 behaviour)
  #
  # EXAMPLES:
  #   1. Set default anthropic/claude-opus-4-8, restart process (fresh SessionManager) -> load_default_model returns anthropic/claude-opus-4-8 -> first create_session succeeds
  #   2. No config file present on first launch -> load_default_model returns None (unchanged current behaviour, create_session declines until selection)
  #   3. Calling set_default_model with an empty string writes nothing to disk and leaves get_default_model returning None
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to have my selected default model persist across process restarts
    So that the first create_session succeeds on every launch without re-selecting a model

  Scenario: A selected default model survives a process restart
    Given a data directory with no persisted default model
    And a session manager whose default model is set to "anthropic/claude-opus-4-5"
    When a fresh session manager is constructed against the same data directory
    Then the fresh session manager reports the default model "anthropic/claude-opus-4-5"
    And the first create_session is no longer declined

  Scenario: First launch with no persisted config has no default model
    Given a data directory with no persisted default model
    When a session manager is constructed against that data directory
    Then the session manager reports no default model
    And the first create_session is declined until a model is selected

  Scenario: An empty model selection is never persisted
    Given a data directory with no persisted default model
    And a session manager constructed against that data directory
    When the default model is set to an empty string
    Then no default model file is written to disk
    And the session manager reports no default model

  Scenario: A missing or malformed config file degrades to no default model
    Given a data directory whose default model file is missing or malformed
    When the persisted default model is loaded from that data directory
    Then the loaded default model is none
