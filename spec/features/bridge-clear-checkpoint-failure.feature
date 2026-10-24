@agent-core
@phase-1
@context-management
@compaction
@CMPCT-054
Feature: Bridge clear checkpoint failure
  """
  The background bridge control must expose a failed durable provider reset
  instead of silently acknowledging conversation replacement. Existing messages
  and counters remain intact when checkpoint persistence fails. The integration
  uses a real registered bridge control and an unavailable filesystem store.
  """

  Scenario: Failed bridge clear reports the durable reset failure
    Given a registered bridge control with committed messages and provider state
    And the provider checkpoint store becomes unavailable
    When the bridge requests conversation clearing
    Then an explicit clear failure is emitted without a cleared or completion event
    And the committed messages and counters remain unchanged
