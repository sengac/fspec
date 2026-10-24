@PROV-154
@PROV-160
@providers
@provider-abstraction
@streaming
@integration
Feature: Codex network idle deadline
  """
  Only network progress resets the Codex transport deadline. UI ticks cannot
  extend it, and completed responses release it before local tool execution.
  Ordinary providers retain their existing policy.
  """

  Scenario: Network idle deadline ignores UI ticks
    Given a Codex server that sends partial text then stalls
    When the caller polls alongside frequent UI ticks
    Then abandoned partial text is retracted and repeated network idle deadlines exhaust the bounded stream retries

  Scenario: Tool execution and other providers retain their policies
    Given a completed Codex response followed by a long-running tool
    When time passes beyond the network deadline
    Then the tool is not timed out and other providers retain generic recovery
