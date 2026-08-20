@PROV-142
@wip
@rust
@tui
@provider-settings
Feature: Per-profile autoContinue form field

  """
  The Rust ratatui /provider Provider Settings profile create/edit form
  (rust/fspec-tui/src/views/provider_settings/profile_form.rs) exposes a 7th
  field "Auto-Continue" as a numeric text field, appended after Streaming.
  A new profile seeds it to empty (off); editing seeds it from the stored
  definition via opt_num(auto_continue). Typed text edits the field through
  the existing text-editing branch (no routing change — only index 5 is the
  boolean toggle). build_definition() emits auto_continue: None for empty,
  Some(0) for the explicit-off sentinel, Some(n) for a positive budget, and
  REJECTS the save with a hint when the value is non-numeric (mirroring
  /continue's invalid-argument rejection). Verified by
  rust/fspec-tui/tests/prov142_auto_continue_form.rs.
  """

  Background: User Story
    As a developer using a local OpenAI-compatible server profile
    I want to set the auto-continue default in the profile form
    So that I can persist 0 (off) or a positive budget without re-typing /continue

  Scenario: New create-profile form seeds Auto-Continue to empty
    Given the user opens the create-profile form
    When the form is initialized
    Then the Auto-Continue field appears after the Streaming field
    And the Auto-Continue field is empty with the placeholder hint "0 (off) or n (budget)"

  Scenario: Typing a budget in the Auto-Continue field and saving persists it
    Given the user is on the create-profile form with the Auto-Continue field focused
    When the user types 300 and saves the profile
    Then the profile is saved with autoContinue set to 300

  Scenario: Typing 0 in the Auto-Continue field and saving persists explicit off
    Given the user is on the create-profile form with the Auto-Continue field focused
    When the user types 0 and saves the profile
    Then the profile is saved with autoContinue set to 0

  Scenario: Editing a profile seeds Auto-Continue from the stored value
    Given a stored profile whose autoContinue value is 500
    When the user opens that profile in the edit form
    Then the Auto-Continue field shows 500

  Scenario: Editing a profile without an autoContinue key seeds Auto-Continue to empty
    Given a stored profile with no autoContinue key
    When the user opens that profile in the edit form
    Then the Auto-Continue field is empty with the placeholder hint "0 (off) or n (budget)"

  Scenario: Non-numeric input in the Auto-Continue field rejects the save
    Given the user is on the profile form with the Auto-Continue field focused
    When the user types abc and saves the profile
    Then the save is rejected with a hint that the value must be 0 or a positive integer
    And the profile is not modified on disk
