@done
@compaction
@parser
@validation
@rust
@CMPCT-037
Feature: Prevent clamping from inverting turn ranges in parse_dag_nodes (FV-003-c)
  """
  On completion, docs/FORMAL_VERIFICATION.md MUST be updated: (1) Remove FV-003-c row from Findings table at line 198. (2) Update the FV-003 row of Proofs status table — if all three limitations now resolved, change to 'Cross-checked'. (3) Remove the limitation_clamping_can_invert_range test from dag_node_proptest.test.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When message_count is provided and turn_start >= message_count, parse_dag_nodes MUST drop the node entirely (return None for that capture)
  #   2. Dropping a node because turn_start >= message_count MUST emit a tracing::warn carrying turn_start and message_count
  #   3. When message_count is provided and turn_start < message_count, turn_end MUST still be clamped to message_count - 1 if it exceeds that bound
  #   4. After clamping, every output node MUST satisfy turn_start <= turn_end unconditionally (including when message_count is provided)
  #   5. When message_count is None, no clamping or message_count-based rejection occurs — only the FV-003-a parse-time start>end check applies
  #   6. When message_count is Some(0), turn_start >= 0 always, so every node with message_count=Some(0) MUST be dropped
  #
  # EXAMPLES:
  #   1. Input '<dag-node depth="D0" turns="200-300" label="both above">content</dag-node>' with message_count=Some(60) yields zero nodes and a tracing warning naming turn_start=200 and message_count=60
  #   2. Input '<dag-node depth="D0" turns="50-300" label="end above">content</dag-node>' with message_count=Some(60) yields one node with turn_start=50 and turn_end=59 and no rejection warning
  #   3. Input '<dag-node depth="D0" turns="59-59" label="boundary">content</dag-node>' with message_count=Some(60) yields one node with turn_start=59 and turn_end=59 (start = message_count - 1 is in range)
  #   4. Input '<dag-node depth="D0" turns="60-100" label="at boundary">content</dag-node>' with message_count=Some(60) yields zero nodes (turn_start == message_count is out of range)
  #   5. Mixed input with one in-range node turns="10-20" and one out-of-range node turns="100-150" with message_count=Some(50) yields exactly one node with label of the in-range block and one rejection warning
  #   6. Input '<dag-node depth="D0" turns="0-10" label="any">content</dag-node>' with message_count=Some(0) yields zero nodes (start >= 0 always so nothing fits)
  #   7. Proptest: for arbitrary well-formed dag-node input AND any message_count, every output node satisfies turn_start <= turn_end AND turn_end < message_count
  #
  # ========================================
  Background: User Story
    As a developer relying on parse_dag_nodes
    I want to have parse_dag_nodes drop nodes whose turn_start is beyond message_count and only clamp turn_end when turn_start is in range
    So that the formal model's invariant (turn_start <= turn_end) is preserved across clamping, even when input is well-formed but message_count is small

  Scenario: turn_start beyond message_count drops the node with a warning
    Given a DAG content string containing a single dag-node block with depth "D0", turns "200-300", and label "both above"
    When I call parse_dag_nodes with message_count=Some(60)
    Then the result contains zero DagNodeMeta entries
    And a tracing warning is emitted carrying turn_start=200 and message_count=60

  Scenario: turn_end above message_count is clamped while turn_start is preserved
    Given a DAG content string containing a single dag-node block with depth "D0", turns "50-300", and label "end above"
    When I call parse_dag_nodes with message_count=Some(60)
    Then the result contains one DagNodeMeta entry with turn_start=50 and turn_end=59
    And no clamping-rejection tracing warning is emitted

  Scenario: turn_start equal to message_count - 1 is accepted
    Given a DAG content string containing a single dag-node block with depth "D0", turns "59-59", and label "boundary"
    When I call parse_dag_nodes with message_count=Some(60)
    Then the result contains one DagNodeMeta entry with turn_start=59 and turn_end=59
    And no clamping-rejection tracing warning is emitted

  Scenario: turn_start equal to message_count drops the node
    Given a DAG content string containing a single dag-node block with depth "D0", turns "60-100", and label "at boundary"
    When I call parse_dag_nodes with message_count=Some(60)
    Then the result contains zero DagNodeMeta entries
    And a tracing warning is emitted carrying turn_start=60 and message_count=60

  Scenario: Mixed input drops only the out-of-range node
    Given a DAG content string containing a D0 dag-node turns "10-20" label "in-range" and a D0 dag-node turns "100-150" label "out-of-range"
    When I call parse_dag_nodes with message_count=Some(50)
    Then the result contains exactly one DagNodeMeta entry with label "in-range", turn_start=10 and turn_end=20
    And exactly one clamping-rejection tracing warning is emitted naming the dropped label "out-of-range"

  Scenario: message_count of zero rejects every node
    Given a DAG content string containing a single dag-node block with depth "D0", turns "0-10", and label "any"
    When I call parse_dag_nodes with message_count=Some(0)
    Then the result contains zero DagNodeMeta entries
    And a tracing warning is emitted carrying turn_start=0 and message_count=0

  Scenario: Property — every parsed node satisfies turn_start <= turn_end and turn_end < message_count after clamping
    Given an arbitrary DAG content string composed of well-formed dag-node blocks and an arbitrary message_count
    When I call parse_dag_nodes with that message_count
    Then for every DagNodeMeta in the result, turn_start <= turn_end holds
    And for every DagNodeMeta in the result, turn_end < message_count holds
