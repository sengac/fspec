@done
@PROV-139
@rust
@provider-settings
@tui
@keyboard-navigation
Feature: OpenAI profile streaming toggle in the Provider Settings form
  """
  The Rust ratatui /provider Provider Settings profile create/edit form
  (codelet/fspec-tui/src/views/provider_settings/profile_form.rs) exposes a
  sixth field "Streaming" as a boolean toggle. A new profile seeds Streaming to
  enabled; editing seeds it from the stored definition via streaming_enabled().
  Space (and Left/Right) flips the value while the field is focused; printable
  characters do NOT mutate it. build_definition() emits streaming: Some(<bool>).
  The boolean-field logic is extracted to a sibling module to keep the file
  under 300 LoC. Verified by codelet/fspec-tui/tests/prov139_streaming_form.rs.
  """

  Background: User Story
    As a fspec user configuring an OpenAI-compatible endpoint
    I want to toggle streaming on or off in the /provider settings form
    So that I can disable SSE streaming for endpoints that misbehave with it

  Scenario: New create-profile form seeds Streaming to enabled
    Given the user opens the create-profile form
    When the form is initialized
    Then the Streaming field shows Enabled

  Scenario: Space toggles the Streaming field
    Given the user is on the create-profile form with the Streaming field focused
    When the user presses Space
    Then the Streaming field flips from Enabled to Disabled

  Scenario: Typing a printable character does not mutate the Streaming field
    Given the user is on the create-profile form with the Streaming field focused and Streaming enabled
    When the user types the letter x
    Then the Streaming field stays Enabled with no text appended

  Scenario: Editing a profile seeds Streaming from the stored definition
    Given a stored profile whose streaming flag is set to disabled
    When the user opens that profile in the edit form
    Then the Streaming field shows Disabled

  Scenario: build_definition emits the current toggle value
    Given the user is on the profile form with Streaming toggled to disabled
    When the form builds a profile definition
    Then the built definition carries streaming set to disabled
