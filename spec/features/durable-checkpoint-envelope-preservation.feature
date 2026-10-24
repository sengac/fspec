@done
@CMPCT-055
@BUG-243
@compaction
@context-management
@agent-core
@phase-1
@persistence
Feature: Durable checkpoint envelope preservation
  """
  Checkpoint-only records are immutable provider metadata, not conversational
  assistant text. Core promotion preserves their original coordinates and
  ordered conversational suffix while exposing the committed checkpoint on the synthetic DAG.
  Internal checkpoint positions expose bounded state identities through originals.
  A complete selected provider/model identifier retains the native provider owner
  when comparing checkpoint generations.
  Provider restore precedence and physical requests remain separate runtime gates.
  """

  Scenario: Provider checkpoint records survive captured prefix promotion and cold replay
    Given genuine provider checkpoints in a captured prefix and an appended tool suffix
    When the candidate commits and a fresh process reads the durable replay
    Then replay starts with the committed checkpoint followed by the exact ordered conversational suffix
    And checkpoint positions remain non-conversational and every immutable original remains retrievable
