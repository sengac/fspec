@done
@provider-settings
@tui
@ts-parity
@rust
@PROV-115
Feature: Profile compaction-threshold range validation parity (reject <1% / >100% and <1000 tokens)
  """
  Mirror TS constants exactly: MIN_PERCENTAGE=1, MAX_PERCENTAGE=100, MIN_TOKEN_THRESHOLD=1000 (compactionThresholdParser.ts:15-21)
  Apply the range guard on the profile save path (profile_form::build_definition, after parse_compaction_trigger returns the split fields) so out-of-range -> (None, None). Do NOT bake the range into the SHARED parse_compaction_trigger (model_selector/form.rs) unless the worker verifies TS range-checks the custom-model form too; default is to leave model_selector untouched and keep its tests green.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A percentage compaction trigger (trailing '%') is valid only when the integer is 1..=100 inclusive; outside that range it is treated as unset (omitted from the saved ProfileDefinition)
  #   2. A bare-integer (tokens) compaction trigger is valid only when >= 1000; below 1000 it is treated as unset
  #   3. Empty or non-numeric compaction-trigger input is unset (no regression from current behavior)
  #   4. An out-of-range compaction trigger does NOT block saving the profile — baseUrl/apiKey/name still save and the threshold is simply omitted
  #   5. Range enforcement is scoped to the profile form save path; the shared model_selector custom-model form behavior is unchanged (TS does not range-check the custom-model form)
  #
  # EXAMPLES:
  #   1. User types '0%' as the compaction trigger and saves a valid profile -> the saved fspec-config.json profile has NO compactionThreshold key
  #   2. User types '100%' -> saved profile has compactionThreshold {type: percentage, value: 100}; '1%' -> {percentage, 1}
  #   3. User types '999' -> saved profile has no compactionThreshold; '1000' -> {type: tokens, value: 1000}
  #   4. User types '101%' but supplies valid baseUrl/apiKey/name -> the profile still saves, with the compactionThreshold key omitted
  #
  # ========================================
  Background: User Story
    As a fspec-tui user editing an OpenAI profile
    I want to enter a compaction trigger value
    So that out-of-range thresholds are rejected exactly like the TypeScript TUI instead of being silently persisted

  @tui
  @provider-settings
  @validation
  Scenario: A below-minimum percentage is treated as unset
    Given a profile create form with a valid base URL, API key and name
    And the compaction trigger field contains "0%"
    When the profile is saved
    Then the saved profile definition has no compaction threshold type
    And the saved profile definition has no compaction threshold value

  @tui
  @provider-settings
  @validation
  Scenario: Percentage boundaries 1 and 100 are accepted
    Given a profile create form with a valid base URL, API key and name
    When the compaction trigger field contains "1%" and the profile is saved
    Then the saved profile definition has compaction threshold type "percentage" and value 1
    When the compaction trigger field contains "100%" and the profile is saved
    Then the saved profile definition has compaction threshold type "percentage" and value 100

  @tui
  @provider-settings
  @validation
  Scenario: An above-maximum percentage is omitted but the profile still saves
    Given a profile create form with a valid base URL, API key and name
    And the compaction trigger field contains "101%"
    When the profile is saved
    Then the profile is saved successfully
    And the saved profile definition has no compaction threshold type
    And the saved profile definition has no compaction threshold value

  @tui
  @provider-settings
  @validation
  Scenario: A below-minimum token count is treated as unset
    Given a profile create form with a valid base URL, API key and name
    When the compaction trigger field contains "999" and the profile is saved
    Then the saved profile definition has no compaction threshold value
    When the compaction trigger field contains "1000" and the profile is saved
    Then the saved profile definition has compaction threshold type "tokens" and value 1000

  @tui
  @provider-settings
  @validation
  Scenario: Empty and non-numeric input remain unset
    Given a profile create form with a valid base URL, API key and name
    When the compaction trigger field contains "" and the profile is saved
    Then the saved profile definition has no compaction threshold value
    When the compaction trigger field contains "abc" and the profile is saved
    Then the saved profile definition has no compaction threshold value
