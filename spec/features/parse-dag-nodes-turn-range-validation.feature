@done
@rust
@validation
@parser
@compaction
@CMPCT-035
Feature: Validate turn_start <= turn_end in parse_dag_nodes (FV-003-a)
  """
  On completion, FORMAL_VERIFICATION.md MUST be updated: (1) Remove FV-003-a row from Findings table at line 196. (2) Decrement limitation count in FV-003 row of Proofs status table at line 216. (3) Remove the limitation_parser_does_not_validate_start_le_end test from dag_node_proptest.test.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. parse_dag_nodes MUST skip any <dag-node> block where the parsed turn_start > turn_end (before clamping)
  #   2. Skipping a reversed-range block MUST emit a tracing::warn with turn_start and turn_end fields
  #   3. Well-formed dag-node blocks (turn_start <= turn_end) MUST continue to parse and appear in the output
  #   4. Mixed input containing both well-formed and reversed-range blocks MUST yield only the well-formed nodes in the output
  #   5. Clamping behaviour (FV-003-c) is OUT OF SCOPE: post-clamp inversion via message_count remains a pinned limitation in this story
  #
  # EXAMPLES:
  #   1. Input '<dag-node depth="D0" turns="50-10" label="reversed">content</dag-node>' yields zero nodes and a tracing warning is emitted
  #   2. Input '<dag-node depth="D0" turns="10-50" label="forward">content</dag-node>' yields one node with turn_start=10, turn_end=50 and no warning
  #   3. Input '<dag-node depth="D0" turns="42-42" label="single">content</dag-node>' (start == end) yields one node and no warning
  #   4. Mixed input with one reversed block and one forward block yields exactly one node (the forward one) and one warning
  #   5. Proptest: for arbitrary input parsed without message_count, every output node satisfies turn_start <= turn_end (unconditional)
  #
  # ========================================
  Background: User Story
    As a developer relying on parse_dag_nodes
    I want to have parse_dag_nodes reject dag-node blocks with inverted turn ranges (start > end)
    So that the formal model's invariant (turn_start <= turn_end) is enforced at the parse boundary instead of silently propagating downstream

  Scenario: Reversed turn range is rejected with a warning
    Given a DAG content string containing a single dag-node block with depth "D0", turns "50-10", and label "reversed"
    When I call parse_dag_nodes with no message_count
    Then the result contains zero DagNodeMeta entries
    And a tracing warning is emitted carrying turn_start=50 and turn_end=10

  Scenario: Forward turn range is parsed unchanged
    Given a DAG content string containing a single dag-node block with depth "D0", turns "10-50", and label "forward"
    When I call parse_dag_nodes with no message_count
    Then the result contains one DagNodeMeta entry with turn_start=10 and turn_end=50
    And no inverted-range tracing warning is emitted

  Scenario: Equal start and end is accepted
    Given a DAG content string containing a single dag-node block with depth "D0", turns "42-42", and label "single"
    When I call parse_dag_nodes with no message_count
    Then the result contains one DagNodeMeta entry with turn_start=42 and turn_end=42
    And no inverted-range tracing warning is emitted

  Scenario: Mixed input keeps only well-formed nodes
    Given a DAG content string containing a reversed-range dag-node "60-10" and a forward dag-node "0-50"
    When I call parse_dag_nodes with no message_count
    Then the result contains exactly one DagNodeMeta entry corresponding to the forward block
    And exactly one inverted-range tracing warning is emitted

  Scenario: Property — every parsed node satisfies turn_start <= turn_end (unconditional, no clamping)
    Given an arbitrary DAG content string composed of well-formed and reversed dag-node blocks
    When I call parse_dag_nodes with no message_count
    Then for every DagNodeMeta in the result, turn_start <= turn_end holds
