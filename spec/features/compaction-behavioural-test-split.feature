@done
@REFAC-033
@codelet
@refactor
@code-quality
Feature: The compaction behavioural test target is split into small files
  """
  rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs (1689 lines) keeps its
  attributes, modules and imports and include!s, in order, fragments under
  rust/agent-loop/tests/cmpct045/ cut between top-level items, so every test, @step comment and
  assertion is unchanged. BUG-345 edits one of its tests next. Coverage mappings are relocated.
  """

  Background: User Story
    As an fspec developer
    I want the compaction behavioural test target split into small files
    So that fixing one of its tests touches a file under 300 lines

  Scenario: The compaction behavioural test files stay under 300 lines
    Given the cmpct045 test target and its fragment directory
    When the line count of each file is measured
    Then no file has 300 or more lines
