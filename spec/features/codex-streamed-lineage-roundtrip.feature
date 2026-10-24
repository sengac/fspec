@providers
@provider-abstraction
@compaction
@regression
@phase-1
Feature: Preserve Codex lineage through streamed tool continuations
  """
  Architecture notes:
  - The streaming agent must retain assistant commentary alongside tool calls,
  including when cancellation flushes a completed tool result.
  - Checkpoint comparison accounts for lossless projection differences such as
  text chunk boundaries, output annotations and JSON argument formatting.
  - Replay retains the original provider items; comparison never repairs missing
  assistant text, changed arguments or missing tool outcomes.
  """

  Background: User Story
    As a Codex session owner
    I want streamed tool results to continue their original conversation
    So that ordinary tool execution is not rejected as changed ancestry

  Scenario: Streamed commentary survives a real multi-turn tool continuation
    Given a provider response streams commentary followed by a tool call
    When the agent executes the tool and requests its next response
    Then the continuation contains the original commentary, call and real result exactly once
    And the agent receives the final answer without a lineage failure

  Scenario: Lossless history projections preserve original checkpoint items
    Given a checkpoint contains assistant output metadata and formatted tool arguments
    When equivalent history is projected with split text chunks and reserialized arguments
    Then the continuation retains the original provider items and the real tool result exactly once
    And restoring the checkpoint from disk permits the same continuation

  Scenario Outline: Changed projected ancestry remains rejected
    Given a checkpoint contains assistant commentary and a tool call
    When the projected history changes the <ancestry>
    Then continuation fails before sending a provider request
    And the checkpoint remains unchanged

    Examples:
      | ancestry       |
      | assistant text |
      | tool arguments |

  Scenario: Cancellation retains commentary with the completed tool outcome
    Given a provider response streams commentary followed by a tool call
    When cancellation occurs after the tool returns its real result
    Then the cancelled conversation contains the commentary, call and real result
    And the saved checkpoint permits continuation with that conversation

  Scenario: Registered tool dialect projections retain their original checkpoint identity
    Given a checkpoint contains a registered raw or namespaced tool call
    When its executor history continues through two tool result round trips
    Then each continuation preserves the original provider call and matching result type
    And restoring the checkpoint from disk permits the same continuation

  Scenario: Changed tool dialect ancestry remains rejected
    Given a checkpoint contains a registered raw or namespaced tool call
    When the executor history changes its arguments, executor name or call identity
    Then continuation fails before sending a provider request
    And the checkpoint remains unchanged
