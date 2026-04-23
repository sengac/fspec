@PROV-080
Feature: Opus 4.7 thinking mode appears inactive due to omitted display default
  """
  Fix is a one-line change in ClaudeThinkingFacade::request_config_for_model() — add display:summarized to the adaptive thinking JSON object
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Adaptive thinking config for ALL adaptive models must include display:'summarized' so the API returns visible thinking text
  #   2. Opus 4.6 behaviour must remain unchanged (already defaults to summarized, but explicit is safer)
  #   3. ThinkingLevel::Off must still return None (no thinking config at all)
  #   4. The NAPI getThinkingConfig must propagate the display field through to the JSON output
  #
  # EXAMPLES:
  #   1. getThinkingConfig('claude-opus-4-7', High) returns {"thinking":{"type":"adaptive","display":"summarized"}} — NOT {"thinking":{"type":"adaptive"}} without display
  #   2. getThinkingConfig('claude-opus-4-6', High) returns {"thinking":{"type":"adaptive","display":"summarized"}} — same format as 4.7, no regression
  #   3. getThinkingConfig('claude-opus-4-7', Off) returns {} — Off still disables thinking entirely
  #   4. getThinkingConfig('claude-opus-4-5', High) returns {"thinking":{"type":"enabled","budget_tokens":32000}} — budgeted models unchanged, no display field
  #
  # ========================================
  Background: User Story
    As a developer
    I want to see thinking content when using Claude Opus 4.7
    So that I can verify the model is reasoning and debug issues

  Scenario: Opus 4.7 adaptive config includes display summarized
    Given the model identifier "claude-opus-4-7"
    And the thinking level is High
    When the thinking config is generated
    Then the config should contain thinking type "adaptive"
    And the config should contain thinking display "summarized"

  Scenario: Opus 4.6 adaptive config also includes display summarized
    Given the model identifier "claude-opus-4-6"
    And the thinking level is High
    When the thinking config is generated
    Then the config should contain thinking type "adaptive"
    And the config should contain thinking display "summarized"

  Scenario: Off level returns empty config for adaptive models
    Given the model identifier "claude-opus-4-7"
    And the thinking level is Off
    When the thinking config is generated
    Then the config should be empty

  Scenario: Budgeted models remain unchanged with no display field
    Given the model identifier "claude-opus-4-5"
    And the thinking level is High
    When the thinking config is generated
    Then the config should contain thinking type "enabled"
    And the config should contain thinking budget_tokens 32000
    And the config should not contain a display field
