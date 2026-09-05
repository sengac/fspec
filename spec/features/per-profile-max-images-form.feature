@done
@PROV-144
@rust
@tui
@provider-settings
@high
Feature: Per-profile Max Images form field
  """
  The /provider OpenAI profile create/edit form (rust/fspec-tui/src/views/
  provider_settings/) gains a new 'Max Images' numeric field appended after
  Preserve Thinking (9th field). Empty means 'use default 4', '0' means no
  vision, any positive integer n is a cap of n images per Read tool result.
  Non-numeric input rejects the save with a hint. The stored value (or the
  default 4 when the key is absent) prefills the field; saving an empty
  field clears the stored key (absent => default 4). The on-disk read
  (profiles_config.rs profile_definition_from_value) reads `maxImages` as
  u32 mirroring the autoContinue pattern.
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want to set a Max Images limit in the /provider profile form
    So that the Read tool enforces how many images a tool call may return, and profiles for no-vision models (0) make image reads fail with a clear message

  # ========================================
  # TUI form: the new Max Images field
  # ========================================

  Scenario: Max Images field prefills to the default 4 when absent
    Given an OpenAI profile "work-vllm" exists with no maxImages key stored
    When I open the profile edit form in the /provider view
    Then the "Max Images" field appears after "Preserve Thinking"
    And the "Max Images" field is prefilled with 4
    When I type 2 into the "Max Images" field and press save
    Then the profile "work-vllm" is stored with maxImages 2
    And re-opening the form shows the "Max Images" field prefilled with 2

  Scenario: Empty Max Images field saves as absent and resolves to the default
    Given an OpenAI profile "work-vllm" stores maxImages 2
    When I open the profile edit form and clear the "Max Images" field
    And I press save
    Then the profile "work-vllm" has no maxImages key on disk
    And re-opening the form shows the "Max Images" field prefilled with 4

  Scenario: Non-numeric Max Images input rejects the save
    Given a profile edit form is open
    When I type "abc" into the "Max Images" field and press save
    Then the save is rejected with a hint that Max Images must be a whole number (0 = no vision, 4 = default)
    And nothing is persisted
