@done
@BUG-282
@agent-core
@context-management
@compaction
@regression
@phase-1
Feature: Canonical streamed tool outcomes
  """
  Architecture notes:
  The streaming handler feeds native BackgroundOutput persistence as well as
  presentation. Canonical outcome text must cross that boundary without a
  second JSON unescape. Selected request validation must continue to compare
  exact assistant, outcome and DAG ancestry before atomic durable promotion.
  Use real loopback Codex responses, filesystem effects, native persistence,
  committed checkpoints and cold restore. Do not repair protected sessions.
  Negative fixtures must remove an actual outcome original reference, mutate
  actual function-call arguments and name, and alter existing DAG ancestry
  established by a successful durable boundary and real continuation. Assert
  the actual typed safe refusal, including source-prefix rejection when missing
  originals invalidate checkpoint authority. Never substitute added text for
  missing evidence. Canonical JSON-string output must retain its outer quotes.
  """

  Background: User Story
    As a native Codex session owner
    I want completed tool outcomes preserved exactly
    So that safe compaction and continuation do not reject authentic work

  Scenario: Streamed textual outcomes retain exact canonical bytes
    Given real tools return canonical JSON strings, object escapes and Windows paths
    When their streamed outcomes are persisted by the native output consumer
    Then the event, runtime, durable originals and checkpoint retain identical outcome text

  Scenario: Authentic streamed outcomes survive successive compaction and cold continuation
    Given genuine streamed outcomes and a committed Codex checkpoint
    When first and successive admitted compactions publish and restore cold
    Then real continuation retains original outcomes exactly once without repeating effects
    And original history bytes and the advancing checkpoint and DAG bindings are preserved

  Scenario: Altered streamed ancestry still blocks atomic compaction
    Given genuine streamed outcomes and a committed Codex checkpoint
    When an outcome, invocation, assistant text or DAG ancestor is changed or missing
    Then selected compaction refuses publication and preserves the manifest and checkpoint
    And inference remains blocked without fallback or fabricated outcomes
