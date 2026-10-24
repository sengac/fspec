@done
@CMPCT-075
@compaction
@agent-core
@context-management
@phase-1
@bug
Feature: Every Codex physical continuation is admitted before sending
  """
  Use CMPCT-071 complete selected-request serializer and explicit capacity/output
  reservation. Admission covers instructions, schemas, reminders and replayed
  assistant output, binds model and committed generation, and precedes transport.
  Rejection retains prior measured output and committed checkpoint, returns a
  typed blocked error, emits no final completion, and never retries another
  representation or transport. Same-transport transient retries keep admitted
  bytes and lineage unchanged. No synthetic Continue or native compaction.
  """

  Background: User Story
    As an agent using full-current-DAG Codex requests
    I want each growing physical continuation checked before network transmission
    So that initial admission cannot authorize a later oversized request

  Scenario: Completed continuation growth is blocked before the oversized send
    Given an admitted initial request with explicit limits
    And a completed non-final response appends enough assistant output to exceed capacity
    When Codex attempts its internal continuation
    Then no oversized request reaches the transport
    And measured usage and output remain observable without a final completion
    And the committed provider checkpoint remains unchanged

  Scenario: Interrupted continuation growth is blocked before the oversized send
    Given an admitted initial request with explicit limits
    And an interrupted incomplete response appends enough assistant output to exceed capacity
    When Codex attempts its internal continuation
    Then no oversized request reaches the transport
    And measured usage and output remain observable without a final completion
    And the committed provider checkpoint remains unchanged

  Scenario: Fitting continuation preserves the full selected request
    Given an admitted initial request containing instructions schemas and required reminders
    And replayable assistant output that fits the explicit remaining capacity
    When Codex sends an internal continuation
    Then both captured requests preserve the selected model generation and fixed context
    And the continuation contains the prior output exactly once

  Scenario: Same transport transient retry preserves admitted content
    Given an admitted continuation request and one declared transient transport failure
    When the selected transport retries the continuation
    Then each captured physical send preserves identical admitted bytes and lineage
