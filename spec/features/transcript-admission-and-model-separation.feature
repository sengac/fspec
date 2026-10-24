@done
@agent-core
@session-management
@persistence
@BUG-250
Feature: Admit transcript-only records without changing model originals
  """
  Shares BUG-246 bounded admission and retry ownership. A failed transcript write
  blocks visible acknowledgement and tool dispatch. Retry stores authentic data
  but never executes a tool. Terminal messages use typed scrubbed outcome codes.
  """

  Scenario: Transcript outcomes never enter model originals
    Given a session with assistant originals and transcript-only terminal records
    When the next model history is restored
    Then terminal prose is absent from model input and original coordinates are unchanged

  Scenario: Transcript persistence failure blocks publication and dispatch
    Given an unavailable durable transcript directory
    When a visible prefix or tool invocation needs acknowledgement
    Then no prefix is published and no tool is executed
    And the original record remains with the shared bounded recovery owner
