@done
@tui
@provider-settings
@PROV-143
Feature: Profile preserve-thinking form (PROV-143)
  """
  architecture:
  - The toggle is a boolean field (like Streaming) rendered as Enabled/Disabled,
  the 8th (last) form field after Auto-Continue.
  - New profiles default to preserve-thinking OFF.
  - The value persists on-disk as "preserveThinking": true|false and
  round-trips through ProfileDefinition.preserve_thinking.
  """

  Background: User Story
    As a provider profile user
    I want a "Preserve Thinking" toggle in the OpenAI profile config form
    So that I can stop the agent from sending thinking/reasoning tokens back to the LLM in the conversation history

  Scenario: The Preserve Thinking toggle appears after Auto-Continue
    Given the profile form field list is rendered
    When the form is inspected
    Then "Preserve Thinking" is the 8th (last) field
    And the focused-field routing treats it as a boolean toggle like Streaming

  Scenario: A new profile defaults Preserve Thinking to disabled
    Given a brand-new profile form is created
    When the form is inspected
    Then preserve_thinking is false
    And the display value for the field is "Disabled"

  Scenario: Toggling the field flips the boolean
    Given the Preserve Thinking field is focused
    When Space is pressed
    Then the value becomes true and renders "Enabled"
    When Space is pressed again
    Then the value becomes false and renders "Disabled"
    And printable characters are never appended to the field

  Scenario: Editing a profile prefills the stored value
    Given a stored profile with preserveThinking = true
    When the edit form is opened for that profile
    Then preserve_thinking is true and renders "Enabled"
    And a stored profile with the key absent seeds preserve_thinking false

  Scenario: Saving a profile persists the toggle
    Given a profile form with preserve_thinking = true
    When the form is built into a ProfileDefinition
    Then the definition carries preserve_thinking = Some(true)

  Scenario: The config loader round-trips the preserveThinking flag
    Given a config file with preserveThinking = true on one profile and an absent key on another
    When the full-config loader reads the profiles
    Then the stored profile preserves the stored value
    And an absent key seeds None (=> disabled)
