@done
@CTX-008
Feature: TUI Configuration Fields and NAPI Bridge for Compaction Threshold
  """
  NAPI bridge: session_set_model and session_set_model_profile accept optional compaction_threshold_type (String) and compaction_threshold_value (u32) params. When provided, set_compaction_threshold_override is called before resolve_compaction_threshold.
  TypeScript types: CompactionThresholdConfig interface with type: 'tokens' | 'percentage' and value: number, added to ProfileConfig, CustomModelDefinition, ModelSelection.
  Input parsing: parseCompactionThreshold(input) → plain number = tokens (min 1000), number with % = percentage (1-100), empty = undefined.
  """

  Background: User Story
    As a developer using a local LLM server
    I want to configure compaction threshold per-model and per-profile in the TUI
    So that I can control when context compaction triggers based on my model's capabilities

  @input-parsing
  Scenario: Parse percentage compaction threshold input
    Given a compaction threshold input field
    When the user enters "80%"
    Then the parsed value should be type "percentage" with value 80

  @input-parsing
  Scenario: Parse token count compaction threshold input
    Given a compaction threshold input field
    When the user enters "200000"
    Then the parsed value should be type "tokens" with value 200000

  @input-parsing
  Scenario: Empty compaction threshold uses built-in default
    Given a compaction threshold input field
    When the user enters ""
    Then the parsed value should be undefined

  @input-parsing
  Scenario: Reject invalid percentage values
    Given a compaction threshold input field
    When the user enters "0%" or "101%"
    Then the parsed value should be undefined

  @input-parsing
  Scenario: Reject token count below minimum threshold
    Given a compaction threshold input field
    When the user enters "500"
    Then the parsed value should be undefined because it is below 1000

  @form-fields
  Scenario: Provider Settings Panel includes compaction threshold field
    Given the Provider Settings Panel form field list
    Then "compactionThreshold" should appear after "maxOutputTokens"

  @form-fields
  Scenario: Custom Model Form includes compaction threshold field
    Given the Custom Model Form field list
    Then "compactionThreshold" should appear between "maxOutputTokens" and "reasoning"

  @type-system
  Scenario: ModelSelection type includes compactionThreshold
    Given the ModelSelection interface
    Then it should have an optional compactionThreshold field of type CompactionThresholdConfig

  @napi-bridge
  Scenario: Model selection service passes compaction threshold to NAPI for profile models
    Given a ModelSelection with compactionThreshold type "tokens" and value 100000
    And the model is a profile-based model
    When the model selection service applies the selection
    Then sessionSetModelProfile should be called with compactionThresholdType "tokens" and compactionThresholdValue 100000

  @napi-bridge
  Scenario: Model selection service passes compaction threshold to NAPI for cloud models
    Given a ModelSelection with compactionThreshold type "percentage" and value 80
    And the model is a cloud provider model
    When the model selection service applies the selection
    Then sessionSetModel should be called with compactionThresholdType "percentage" and compactionThresholdValue 80

  @napi-bridge
  Scenario: Model selection service omits compaction threshold when not configured
    Given a ModelSelection without compactionThreshold
    When the model selection service applies the selection
    Then the NAPI call should pass null for compaction threshold parameters

  @napi-bridge
  @integration
  Scenario: Profile compaction threshold flows through when model has none
    Given a profile with compactionThreshold type "percentage" and value 75
    And a custom model without a compactionThreshold override
    When the user selects the custom model from that profile
    Then the profile-level compaction threshold should be passed to the NAPI call

  @napi-types
  Scenario: NAPI type declarations include compaction threshold parameters
    Given the codelet-napi index.d.ts type declarations
    Then sessionSetModel should accept optional compactionThresholdType and compactionThresholdValue parameters
    And sessionSetModelProfile should accept optional compactionThresholdType and compactionThresholdValue parameters
