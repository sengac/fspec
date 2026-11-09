@done
@CMPCT-084
@compaction
@agent-loop
@CMPCT-087
@agent-core
@context-management
Feature: Compactor synthesis continues in DAG-node chunks when the output limit truncates it
  """
  Compactor synthesis (agent-loop/src/compaction_execution.rs) reads the stop reason of every
  synthesis response: from StreamingResponse::stop_reason() on the streaming path and from the
  serialized raw response on the non-streaming path. A length/max_tokens stop is truncation.

  The chunking loop (agent-loop/src/compaction_chunking.rs) keeps text up to the last complete
  </dag-node> or </dag-files>, resends it as the assistant turn and asks for only the remaining
  nodes. A chunk that adds no complete node gets a "write the next node now" instruction. At most
  8 chunks run inside the existing synthesis deadline; two consecutive chunks without progress
  fail with a typed output-limit error. Each chunk request passes request admission. The
  assembled candidate goes through the unchanged DAG validation and commit.

  Scenarios exercise compaction_execution::run directly (exploration ends via max depth, so
  synthesis runs); the shared BUG-229 compaction harness currently fails in its fixture.
  """

  Background: User Story
    As a developer whose long sessions compact on local reasoning models with a small output cap
    I want to have the compactor write its DAG across several responses when one response hits the output limit
    So that compaction succeeds instead of blocking the session until I resubmit input

  Scenario: A synthesis truncated mid-node continues from the next node and commits the complete DAG
    Given a compactor whose synthesis returns one complete node and half of the next before the output limit
    And the continuation response returns the remaining node and stops normally
    When the compactor runs
    Then two synthesis requests are sent
    And the continuation request carries only the complete node and asks for the remaining nodes
    And the compactor candidate contains both nodes and not the partial fragment

  Scenario: A synthesis that spends the output limit on reasoning is asked to write the next node immediately
    Given a compactor whose first synthesis response contains no text and stops at the output limit
    And the next response returns the complete DAG and stops normally
    When the compactor runs
    Then the continuation request has no assistant turn and asks for the next node immediately
    And the compactor candidate is the complete DAG

  Scenario: Repeated output-limit stops without progress fail with an output-limit error
    Given a compactor whose every synthesis response contains no text and stops at the output limit
    When the compactor runs
    Then the compactor fails with an error stating the output limit was reached
    And no further synthesis request is sent after the second chunk

  Scenario: A correction attempt re-synthesizes without repeating exploration
    Given a compactor whose first candidate fails DAG validation after exploring the session
    When the correction attempt runs
    Then the correction sends at most one exploration request before synthesis
    And the corrected DAG is returned

  Scenario: The compaction instruction is sent once
    Given a compactor run for a persisted session
    When the first compactor request is sent
    Then the system preamble does not repeat the compaction instruction
    And the user task carries the compaction instruction
