@phase-1
@cli
@providers
@provider-abstraction
@regression
Feature: Record real tool cancellations when a Codex turn is interrupted
  """
  Architecture notes:
  The streaming consumer owns interruption and records its cancellation outcomes
  before publishing interruption or completion. A committed provider response can
  contain calls which have not yet reached the sequential tool executor.
  Completed results remain authoritative. Replay never manufactures outcomes.
  """

  Background: User Story
    As a Codex session owner
    I want to interrupt tool execution and continue the conversation
    So that stopping a tool does not permanently invalidate session replay

  Scenario Outline: Interrupted batches retain completed results and cancel every remaining call
    Given a Codex response commits three <dialect> tool calls
    And the first <completed> tools finish before interruption
    When I interrupt the next tool <timing>
    Then every unfinished committed call has exactly one recorded cancellation
    And completed tool results remain unchanged
    And cancellation results are published before interruption and completion
    And the next request succeeds with the original invocation identities

    Examples:
      | dialect  | completed | timing              |
      | function | 0         | before it starts    |
      | function | 1         | before it starts    |
      | custom   | 0         | before it starts    |
      | custom   | 1         | before it starts    |
      | function | 0         | while it is running |
      | function | 1         | while it is running |
      | custom   | 0         | while it is running |
      | custom   | 1         | while it is running |

  Scenario: Interrupting before any committed tool call creates no cancellation
    Given a Codex session has no committed tool call
    When I interrupt its streaming consumer
    Then no tool cancellation is recorded or published

  Scenario: Interrupted batches publish missing committed calls before their outcomes
    Given a committed batch contains distinct function calls with a namespaced alias and a namespaced custom patch call
    And zero or one tools have completed before the next tool starts running
    When I interrupt the streaming consumer
    Then each committed call is published exactly once before its matching outcome
    And unpublished calls retain their item and invocation identities and normalized names and inputs
    And completed outcomes remain unchanged
    And conversational history contains every committed call and outcome
    And the next request succeeds with the original invocation identities

  Scenario: Cancellation hooks receive the matching committed tool metadata
    Given a background session has post-tool hooks recording tool names and inputs
    And its committed batch contains distinct function calls with a namespaced alias and a namespaced custom patch call
    When I interrupt its streaming consumer during the first tool
    Then each cancellation hook receives its own tool name and normalized input exactly once
    And persisted conversation blocks contain every committed call before its outcome
