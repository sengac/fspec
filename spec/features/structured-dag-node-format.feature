@CMPCT-017
Feature: Structured DAG Node Format and Engine Parsing
  """
  Uses regex to parse <dag-node> blocks — no XML parser needed.
  DagNodeMeta and DagDepth structs go in rust/core/src/compaction/model.rs alongside existing StructuralAnnotation.
  The pending_dag state is extended to an InjectSummaryState struct holding both the raw DAG string and parsed Vec<DagNodeMeta>.
  Instruction constant COMPACTION_SYSTEM_INSTRUCTION is updated to specify structured dag-node XML format.
  """

  Background: User Story
    As a AI agent undergoing context compaction
    I want to write structured DAG nodes with XML-annotated turn ranges and depth levels that the engine can parse and validate
    So that future compaction cycles can incrementally update DAGs rather than rebuilding from scratch, and the engine can extract provenance metadata for scoped queries

  @happy-path
  Scenario: Parse structured DAG with multiple depth levels
    Given the agent has written a DAG containing three dag-node blocks
      | depth | turns | label                   |
      | D2    | 0-45  | Architecture Decisions  |
      | D1    | 46-82 | Auth Implementation Arc |
      | D0    | 83-95 | Fixing test failures    |
    And the session has 96 persisted messages
    When the engine parses the DAG content after inject_summary
    Then it should extract 3 DagNodeMeta entries
    And the entries should be sorted by turn_start ascending
    And each entry should have the correct depth, turn range, and label

  @backward-compat
  Scenario: Parse plain markdown DAG with no dag-node blocks
    Given the agent has written a free-form markdown DAG with no dag-node XML blocks
    When the engine parses the DAG content after inject_summary
    Then it should return an empty list of DagNodeMeta entries
    And the DAG content should be stored normally without error

  @edge-case
  Scenario: Clamp turn range when turn_end exceeds message count
    Given the agent has written a dag-node with turns "0-200"
    And the session has only 150 persisted messages
    When the engine parses the DAG content after inject_summary
    Then the DagNodeMeta turn_end should be clamped to 149
    And the turn_start should remain 0

  @edge-case
  Scenario: Skip dag-node with invalid depth value
    Given the agent has written two dag-node blocks
      | depth | turns | label         |
      | D2    | 0-45  | Valid node    |
      | D3    | 46-80 | Invalid depth |
    When the engine parses the DAG content after inject_summary
    Then it should extract 1 DagNodeMeta entry for the valid node
    And the invalid depth node should be skipped

  @edge-case
  Scenario: Skip dag-node with missing required attributes
    Given the agent has written a dag-node block missing the label attribute
    And the agent has written a valid dag-node block with all attributes
    When the engine parses the DAG content after inject_summary
    Then only the valid dag-node should be parsed
    And the malformed dag-node should be skipped

  @edge-case
  Scenario: Parse overlapping turn ranges with warning
    Given the agent has written two dag-node blocks with overlapping ranges
      | depth | turns | label        |
      | D2    | 0-50  | First range  |
      | D1    | 30-80 | Second range |
    When the engine parses the DAG content after inject_summary
    Then it should extract 2 DagNodeMeta entries
    And the entries should be sorted by turn_start ascending
    And a warning should be logged about overlapping turn ranges

  @data-model
  Scenario: DagNodeMeta serialization round-trip
    Given a DagNodeMeta with depth D1, turn_start 10, turn_end 50, and label "Test Arc"
    When the DagNodeMeta is serialized to JSON
    And the JSON is deserialized back to DagNodeMeta
    Then all fields should match the original values

  @instruction
  Scenario: Compaction instruction specifies structured dag-node format
    When the compaction system instruction is loaded
    Then it should contain guidance for writing dag-node XML blocks
    And it should explain the D0, D1, and D2 depth semantics
    And it should specify the turns attribute format as "N-M" inclusive range
    And it should require a label attribute on each dag-node

  @integration
  Scenario: Parsed DagNodeMeta stored in InjectSummaryState for downstream access
    Given the agent has written a DAG with structured dag-node blocks
    When inject_summary is called and apply_pending_dag processes the content
    Then the InjectSummaryState should contain both the raw DAG string and the parsed Vec of DagNodeMeta
    And downstream features should be able to access the parsed metadata
