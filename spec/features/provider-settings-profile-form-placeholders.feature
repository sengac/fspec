@done
@profiles
@rust
@tui
@provider-settings
@PROV-135
Feature: Profile form fields lack placeholder hints for empty numeric/threshold fields
  """
  Placeholder rendered in profile_form_render.rs field_line via a small placeholder_for(idx) helper; DIM modifier retained; build_definition() is NOT touched so nothing is persisted. Base URL (idx 0) already prefills a real value in new_create() so its placeholder is rarely seen but kept for parity.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An empty Context Window field renders the dim placeholder 128000
  #   2. An empty Max Output Tokens field renders the dim placeholder 16384
  #   3. An empty Compaction Threshold field renders the dim placeholder 80% or 200000
  #   4. An empty Base URL field renders the dim placeholder http://localhost:8888
  #   5. A field with a real typed value renders that value, not the placeholder
  #   6. Placeholder hints are display-only and are never persisted into the saved profile
  #
  # EXAMPLES:
  #   1. Open create-profile, move to Max Output Tokens (empty), the row shows a dim 16384
  #   2. Type 8192 into Max Output Tokens, the row shows 8192 and the saved profile has max_output_tokens 8192
  #   3. Leave Compaction Threshold blank and save, the saved profile has no compaction threshold type or value
  #   4. Move to Context Window (empty), the row shows a dim 128000
  #   5. Move to Compaction Threshold (empty), the row shows a dim 80% or 200000
  #
  # ASSUMPTIONS:
  #   1. The API Key field, when empty, keeps its existing empty/(empty) treatment and does NOT get a numeric placeholder hint.
  #
  # ========================================
  Background: User Story
    As a provider settings user creating or editing an OpenAI profile
    I want to see example placeholder values for the numeric and threshold fields when they are empty
    So that I know what format and typical values are expected for each field

  Scenario: Empty Context Window field shows a dim placeholder
    Given a new profile form is open with the Context Window field empty
    When the profile form is rendered
    Then the Context Window row shows the placeholder "128000"
    Then the placeholder is rendered with the dim modifier

  Scenario: Empty Max Output Tokens field shows a dim placeholder
    Given a new profile form is open with the Max Output Tokens field empty
    When the profile form is rendered
    Then the Max Output Tokens row shows the placeholder "16384"
    Then the placeholder is rendered with the dim modifier

  Scenario: Empty Compaction Threshold field shows a dim placeholder
    Given a new profile form is open with the Compaction Threshold field empty
    When the profile form is rendered
    Then the Compaction Threshold row shows the placeholder "80% or 200000"
    Then the placeholder is rendered with the dim modifier

  Scenario: A field with a typed value shows the value not the placeholder
    Given a new profile form is open
    Given the Max Output Tokens field contains the typed value "8192"
    When the profile form is rendered
    Then the Max Output Tokens row shows "8192"
    Then the Max Output Tokens row does not show the placeholder "16384"

  Scenario: Placeholder hints are never persisted into the saved profile
    Given a new profile form is open with base URL and API key filled in
    Given the Context Window, Max Output Tokens, and Compaction Threshold fields are left empty
    When the profile definition is built from the form
    Then the saved profile has no context window value
    Then the saved profile has no max output tokens value
    Then the saved profile has no compaction threshold type or value
