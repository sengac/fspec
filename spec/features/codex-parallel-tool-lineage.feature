@providers
@provider-abstraction
@compaction
@regression
@phase-1
Feature: Continue Codex history after parallel tool execution
  """
  Architecture notes:
  - Provider checkpoints group parallel calls before their recorded outcomes.
  The streaming consumer records each call together with its completed result.
  - Compare these equivalent projections against the original checkpoint batch,
  without changing call order, result order, arguments or recorded payloads.
  - Replay retains original checkpoint items and appends real unmatched outcomes.
  No lineage failure authorizes checkpoint reset or fabricated tool results.
  """

  Background: User Story
    As a Codex session owner
    I want to continue after tools from one response have completed
    So that recording their real outcomes does not invalidate my conversation

  Scenario: Interleaved parallel tool outcomes preserve committed lineage
    Given a checkpoint groups parallel tool calls before their real outcomes
    When the next turn supplies the same calls interleaved with those outcomes
    Then continuation replays each original call and outcome exactly once
    And the same continuation succeeds after restoring the checkpoint from disk

  Scenario: Pending parallel calls retain new interleaved outcomes
    Given a checkpoint contains parallel tool calls awaiting their real outcomes
    When continuation supplies each call followed by its completed outcome
    Then all real outcomes are appended exactly once without duplicating any call

  Scenario Outline: Changed parallel tool ancestry remains rejected
    Given a checkpoint groups parallel tool calls before their real outcomes
    When the interleaved history changes the <ancestry>
    Then continuation fails before sending a provider request
    And the original checkpoint remains unchanged

    Examples:
      | ancestry          |
      | call arguments    |
      | call identity     |
      | call order        |
      | result payload    |
      | result order      |
      | missing result    |
      | intervening text  |
      | unrelated outcome |
