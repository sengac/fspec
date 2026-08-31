@done
@session
@provider-settings
@PROV-143
Feature: Profile preserve-thinking session seeding (PROV-143)
  """
  architecture:
  - Sessions seeded from a profile (openai:<profile>/<model>) carry the
  profile's stored preserveThinking value into the runtime session's
  preserve_thinking_enabled flag (create_background_session_inner).
  - An absent key seeds false (the default — stripped).
  """

  Background: User Story
    As a provider profile user
    I want my OpenAI profile's Preserve Thinking setting to apply to sessions started from that profile
    So that I do not have to reconfigure thinking handling per session

  Scenario: A profile with preserveThinking = true seeds the session flag on
    Given a local-server profile with preserveThinking = true
    When a session is created against a model of that profile
    Then the session seeds preserve_thinking_enabled = true

  Scenario: A profile with preserveThinking = false seeds the session flag off
    Given a local-server profile with preserveThinking = false
    When a session is created against a model of that profile
    Then the session seeds preserve_thinking_enabled = false

  Scenario: A profile without the key seeds the session flag off
    Given a local-server profile with no preserveThinking key
    When a session is created against a model of that profile
    Then the session seeds preserve_thinking_enabled = false
    And a profile without the key seeds the flag false
