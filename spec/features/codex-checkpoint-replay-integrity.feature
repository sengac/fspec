@done
@CMPCT-080
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Remove silent checkpoint repair and fabricated replay tool results
  """
  Implementation:
  - P1; CB-28/replay. Read attached cmpct-080-remove-replay-fallbacks.md and 00-shared-contract.md. Evidence classification and exact source anchors are in the specific brief. No runtime reproduction claimed. No native/flat/hybrid substitution, forced salvage, silent replay repair or transport downgrade is permitted.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Retain the agent-owned hierarchical DAG and immutable originals. Failure preserves the last committed generation, returns typed failure, blocks unsafe inference and emits no completion; no memory, replay, transport, request-mode or admission fallback. Follow the attached shared contract.
  #   2. Distinguish genuinely absent checkpoint from inaccessible, unsupported, mismatched or failed-to-persist state; propagate typed outcomes through public restore/reset.
  #   3. Only a validated recorded generation transition may change checkpoint ancestry; failed persistence leaves previous committed memory/checkpoint authoritative.
  #   4. Replay uses real result/cancellation records. Missing pairs return explicit replay failure, never fabricated text, silent archive reconstruction or weakened continuity.
  #
  # EXAMPLES:
  #   1. Given poisoned checkpoint/writer state, unsupported version or reset-save failure, when restore/prepare/commit runs, then it cannot succeed as absent/fresh state.
  #   2. Given a tool call without an actual result/cancellation record, when replay builds a request, then it fails without sending invented tool output or changing the checkpoint.
  #
  # ========================================
  Background: User Story
    As a Codex session owner
    I want to receive explicit checkpoint and replay failures
    So that failed persistence and missing tool outcomes never masquerade as safe continuation

  Scenario: Checkpoint commit persistence failure preserves prior memory and disk
    Given a Codex session with a checkpoint persisted in a real temporary store
    And the next checkpoint save fails before durable promotion
    When I commit a completed provider response
    Then I receive an explicit checkpoint persistence failure
    And both the live checkpoint and a cold reload equal the previous durable checkpoint
    And ordinary inference remains blocked until explicit resolution

  Scenario: Failed reset cannot destroy the authoritative checkpoint
    Given a Codex session with a durable checkpoint and committed generation
    And its recorded generation reset cannot be persisted
    When I request the generation reset through the public session API
    Then the public API returns an explicit persistence failure
    And live and cold-resumed checkpoint state remain unchanged
    And no automatic fresh-session request is sent

  Scenario Outline: Replay refuses a tool call with no recorded outcome
    Given a durable Codex checkpoint containing an unresolved <call> call
    And no real tool result or cancellation outcome was recorded
    When I continue the session through the provider adapter
    Then replay returns an explicit missing-outcome failure before sending a request
    And no invented cancellation text is added to the request or checkpoint
    And the durable checkpoint remains unchanged

    Examples:
      | call          |
      | function tool |
      | custom tool   |

  Scenario: A genuinely new session and recorded tool outcomes remain supported
    Given a genuinely new Codex session without a checkpoint
    When I send its first request and continue with real recorded tool outcomes
    Then the captured continuation contains each real result or recorded cancellation exactly once
    And no replacement outcome is fabricated
