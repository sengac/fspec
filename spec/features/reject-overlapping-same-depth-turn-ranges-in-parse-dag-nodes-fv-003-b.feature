@done
@compaction
@validation
@parser
@rust
@CMPCT-036
Feature: Reject overlapping same-depth turn ranges in parse_dag_nodes (FV-003-b)

  """
  On completion, FORMAL_VERIFICATION.md MUST be updated: (1) Remove FV-003-b row from Findings table at line 197. (2) Decrement limitation count in FV-003 row of Proofs status table at line 216. (3) Remove the limitation_parser_does_not_reject_overlap test from dag_node_proptest.test.rs.
  Existing limitation_parser_does_not_reject_overlap test must be REMOVED from dag_node_proptest.test.rs (it pinned the now-obsolete behaviour)
  Implementation lives in codelet/core/src/compaction/model.rs::parse_dag_nodes — replace the current 'check overlaps and warn' loop with a 'reject overlaps, drop later, log warn' filter. Sort by (depth, turn_start) before the dedupe pass to make left-to-right preference deterministic.
  FORMAL_VERIFICATION.md updates required on done: (1) remove FV-003-b row from Findings table at line 197, (2) update FV-003 row of Proofs status table at line 216 to reflect '1 limitation pinned' instead of '2 limitations pinned'. The post-validating virtual hook already greps for 'FV-003-b' and blocks if still present.
  Stable test pattern follows CMPCT-035 (FV-003-a): example-mapped Gherkin → 1:1 unit tests with @step comments + a single proptest assertion. No tracing-subscriber capture is wired in; warning emission is documented as 'verified by source inspection' per the existing convention.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. parse_dag_nodes MUST drop any dag-node whose [turn_start, turn_end] interval overlaps with an already-accepted same-depth node (post-sort, processing left-to-right)
  #   2. Overlap detection MUST be scoped to nodes sharing the same depth — D1 vs D2 (cross-depth) coverage of the same span MUST still be accepted because hierarchical compaction depends on it
  #   3. When a same-depth overlap is detected, the parser MUST keep the earlier (lower turn_start, then first-encountered) node and drop the later one
  #   4. Dropping an overlapping node MUST emit a tracing::warn carrying the depth, the kept node's range and label, and the dropped node's range and label
  #   5. Adjacency at the boundary (next node's turn_start == prior node's turn_end) MUST count as overlap because turn_end is inclusive (same turn summarised twice)
  #   6. Output ordering (sorted by turn_start ascending, P1) MUST be preserved after overlap rejection
  #
  # EXAMPLES:
  #   1. Two D1 nodes turns="0-10" label="a" and turns="5-15" label="b" → only node "a" is returned; one warning naming both ranges and the dropped label "b" is emitted
  #   2. Two D1 nodes turns="0-5" label="a" and turns="6-10" label="b" (gap, non-overlapping) → both nodes returned; no overlap warning emitted
  #   3. Two D1 nodes turns="0-5" label="a" and turns="5-10" label="b" (touching at turn 5 — turn 5 is in both because turn_end is inclusive) → only "a" is returned and a warning is emitted (boundary touch counts as overlap)
  #   4. A D1 node turns="0-50" label="d1" and a D2 node turns="0-50" label="d2" (different depths but identical span) → both nodes returned; no overlap warning emitted (cross-depth coverage is intentional)
  #   5. Three D1 nodes turns="0-10" label="a", turns="5-8" label="b" (fully inside a), turns="20-30" label="c" → "a" and "c" are returned, "b" is dropped with one warning
  #   6. Proptest: for arbitrary input parsed (with or without message_count), every pair of returned nodes that share the same depth has disjoint [turn_start, turn_end] intervals
  #   7. Empty input and single same-depth node remain unchanged (no overlap possible) — output equals input parse
  #
  # ========================================

  Background: User Story
    As a developer relying on parse_dag_nodes
    I want to have parse_dag_nodes reject same-depth dag-node blocks whose turn ranges overlap a previously-accepted node
    So that the formal-model invariant (G2: SameDepthNonOverlapping) is enforced at the parse boundary instead of being a soft warning that lets duplicate same-depth coverage slip through to downstream consumers

  Scenario: Same-depth overlap drops later node and warns
    Given a DAG content string containing two D1 dag-node blocks with turns "0-10" label "a" and turns "5-15" label "b"
    When I call parse_dag_nodes with no message_count
    Then the result contains exactly one DagNodeMeta with depth D1, turn_start 0, turn_end 10, and label "a"
    Then a tracing warning is emitted naming depth D1, kept range 0-10 label "a", and dropped range 5-15 label "b"


  Scenario: Disjoint same-depth ranges are both kept
    Given a DAG content string containing two D1 dag-node blocks with turns "0-5" label "a" and turns "6-10" label "b"
    When I call parse_dag_nodes with no message_count
    Then the result contains exactly two DagNodeMeta entries with labels "a" then "b" sorted by turn_start
    Then no overlap tracing warning is emitted


  Scenario: Boundary touch counts as overlap because turn_end is inclusive
    Given a DAG content string containing two D1 dag-node blocks with turns "0-5" label "a" and turns "5-10" label "b"
    When I call parse_dag_nodes with no message_count
    Then the result contains exactly one DagNodeMeta with label "a" and turn range 0-5
    Then exactly one overlap tracing warning is emitted naming the dropped label "b"


  Scenario: Cross-depth coverage of the same span is intentional and accepted
    Given a DAG content string containing a D1 dag-node turns "0-50" label "d1" and a D2 dag-node turns "0-50" label "d2"
    When I call parse_dag_nodes with no message_count
    Then the result contains exactly two DagNodeMeta entries with depths D1 and D2 both spanning turns 0-50
    Then no overlap tracing warning is emitted


  Scenario: Containment overlap drops only the inner node and preserves disjoint neighbours
    Given a DAG content string containing three D1 dag-node blocks turns "0-10" label "a", turns "5-8" label "b", and turns "20-30" label "c"
    When I call parse_dag_nodes with no message_count
    Then the result contains exactly two DagNodeMeta entries with labels "a" and "c" sorted by turn_start
    Then exactly one overlap tracing warning is emitted naming the dropped label "b"


  Scenario: Empty input and singleton same-depth input are unaffected
    Given an empty DAG content string and separately a DAG content string containing a single D1 dag-node turns "3-9" label "solo"
    When I call parse_dag_nodes on each input with no message_count
    Then the empty input yields zero DagNodeMeta entries and the singleton input yields exactly one entry with label "solo"
    Then no overlap tracing warning is emitted for either input


  Scenario: Property — every pair of same-depth output nodes has disjoint turn ranges
    Given an arbitrary DAG content string composed of well-formed dag-node blocks across depths D0, D1, and D2
    When I call parse_dag_nodes with no message_count
    Then for every pair of returned DagNodeMeta entries that share the same depth, their [turn_start, turn_end] intervals are disjoint

