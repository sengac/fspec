// CMPCT-036 tests for `parse_dag_nodes`, moved out of dag_node_proptest.test.rs
// (COV-075: one test file per feature). Included into its own test module in
// compaction/mod.rs together with the shared generators fragment.
//
// Feature: spec/features/reject-overlapping-same-depth-turn-ranges-in-parse-dag-nodes-fv-003-b.feature

use crate::compaction::model::DagDepth;
use crate::compaction::parse_dag_nodes;
use proptest::prelude::*;

// ────────────────────────────────────────────────────────────────────────────
// CMPCT-036 — `parse_dag_nodes` rejects same-depth overlap (FV-003-b closed)
//
// Feature: spec/features/reject-overlapping-same-depth-turn-ranges-in-parse-dag-nodes-fv-003-b.feature
//
// These tests close FV-003-b by asserting that any second `<dag-node>` whose
// `[turn_start, turn_end]` interval overlaps an already-accepted same-depth
// node is dropped at the parse boundary with a `tracing::warn`. Cross-depth
// overlap (e.g., a D2 node spanning the same turns as a D1 node) remains
// accepted because hierarchical compaction depends on it.
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn cmpct_036_same_depth_overlap_drops_later_node_and_warns() {
    // @step Given a DAG content string containing two D1 dag-node blocks with turns "0-10" label "a" and turns "5-15" label "b"
    let content = r#"
<dag-node depth="D1" turns="0-10" label="a">content a</dag-node>
<dag-node depth="D1" turns="5-15" label="b">content b</dag-node>
"#;

    // @step When I call parse_dag_nodes with no message_count
    let nodes = parse_dag_nodes(content, None);

    // @step Then the result contains exactly one DagNodeMeta with depth D1, turn_start 0, turn_end 10, and label "a"
    assert_eq!(nodes.len(), 1, "later overlapping same-depth node must be dropped");
    assert!(matches!(nodes[0].depth, DagDepth::D1));
    assert_eq!(nodes[0].turn_start, 0);
    assert_eq!(nodes[0].turn_end, 10);
    assert_eq!(nodes[0].label, "a");
    // @step Then a tracing warning is emitted naming depth D1, kept range 0-10 label "a", and dropped range 5-15 label "b"
    //
    // (Verified by source inspection — the rejection branch in
    //  `parse_dag_nodes` emits `tracing::warn!` carrying the depth, the
    //  kept node's range and label, and the dropped node's range and
    //  label. Programmatic capture would require a tracing-subscriber
    //  test layer which is not currently wired into codelet-core's
    //  dev-dependencies; the existing CMPCT-035 tests follow the same
    //  convention.)
}

#[test]
fn cmpct_036_disjoint_same_depth_ranges_are_both_kept() {
    // @step Given a DAG content string containing two D1 dag-node blocks with turns "0-5" label "a" and turns "6-10" label "b"
    let content = r#"
<dag-node depth="D1" turns="0-5" label="a">content a</dag-node>
<dag-node depth="D1" turns="6-10" label="b">content b</dag-node>
"#;

    // @step When I call parse_dag_nodes with no message_count
    let nodes = parse_dag_nodes(content, None);

    // @step Then the result contains exactly two DagNodeMeta entries with labels "a" then "b" sorted by turn_start
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].label, "a");
    assert_eq!(nodes[0].turn_start, 0);
    assert_eq!(nodes[0].turn_end, 5);
    assert_eq!(nodes[1].label, "b");
    assert_eq!(nodes[1].turn_start, 6);
    assert_eq!(nodes[1].turn_end, 10);
    // @step Then no overlap tracing warning is emitted
    //
    // (Verified by source inspection — when no overlap is detected the
    //  warning emission branch is not entered.)
}

#[test]
fn cmpct_036_boundary_touch_counts_as_overlap() {
    // @step Given a DAG content string containing two D1 dag-node blocks with turns "0-5" label "a" and turns "5-10" label "b"
    let content = r#"
<dag-node depth="D1" turns="0-5" label="a">content a</dag-node>
<dag-node depth="D1" turns="5-10" label="b">content b</dag-node>
"#;

    // @step When I call parse_dag_nodes with no message_count
    let nodes = parse_dag_nodes(content, None);

    // @step Then the result contains exactly one DagNodeMeta with label "a" and turn range 0-5
    assert_eq!(
        nodes.len(),
        1,
        "boundary touch (next.turn_start == prior.turn_end) must count as overlap because turn_end is inclusive"
    );
    assert_eq!(nodes[0].label, "a");
    assert_eq!(nodes[0].turn_start, 0);
    assert_eq!(nodes[0].turn_end, 5);
    // @step Then exactly one overlap tracing warning is emitted naming the dropped label "b"
    //
    // (Verified by source inspection — boundary equality enters the
    //  rejection branch via the inclusive `turn_start <= last.turn_end`
    //  predicate, emitting exactly one warning.)
}

#[test]
fn cmpct_036_cross_depth_coverage_is_accepted() {
    // @step Given a DAG content string containing a D1 dag-node turns "0-50" label "d1" and a D2 dag-node turns "0-50" label "d2"
    let content = r#"
<dag-node depth="D1" turns="0-50" label="d1">content d1</dag-node>
<dag-node depth="D2" turns="0-50" label="d2">content d2</dag-node>
"#;

    // @step When I call parse_dag_nodes with no message_count
    let nodes = parse_dag_nodes(content, None);

    // @step Then the result contains exactly two DagNodeMeta entries with depths D1 and D2 both spanning turns 0-50
    assert_eq!(
        nodes.len(),
        2,
        "cross-depth coverage of the same span must be accepted (intentional per the formal model)"
    );
    let depths: Vec<&DagDepth> = nodes.iter().map(|n| &n.depth).collect();
    assert!(depths.iter().any(|d| matches!(d, DagDepth::D1)));
    assert!(depths.iter().any(|d| matches!(d, DagDepth::D2)));
    for n in &nodes {
        assert_eq!(n.turn_start, 0);
        assert_eq!(n.turn_end, 50);
    }
    // @step Then no overlap tracing warning is emitted
    //
    // (Verified by source inspection — overlap rejection is scoped to
    //  same-depth pairs only.)
}

#[test]
fn cmpct_036_containment_drops_inner_and_preserves_disjoint_neighbour() {
    // @step Given a DAG content string containing three D1 dag-node blocks turns "0-10" label "a", turns "5-8" label "b", and turns "20-30" label "c"
    let content = r#"
<dag-node depth="D1" turns="0-10" label="a">content a</dag-node>
<dag-node depth="D1" turns="5-8" label="b">content b</dag-node>
<dag-node depth="D1" turns="20-30" label="c">content c</dag-node>
"#;

    // @step When I call parse_dag_nodes with no message_count
    let nodes = parse_dag_nodes(content, None);

    // @step Then the result contains exactly two DagNodeMeta entries with labels "a" and "c" sorted by turn_start
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].label, "a");
    assert_eq!(nodes[0].turn_start, 0);
    assert_eq!(nodes[0].turn_end, 10);
    assert_eq!(nodes[1].label, "c");
    assert_eq!(nodes[1].turn_start, 20);
    assert_eq!(nodes[1].turn_end, 30);
    // @step Then exactly one overlap tracing warning is emitted naming the dropped label "b"
    //
    // (Verified by source inspection — exactly one drop per overlapping
    //  same-depth node produces exactly one warning.)
}

#[test]
fn cmpct_036_empty_and_singleton_inputs_are_unaffected() {
    // @step Given an empty DAG content string and separately a DAG content string containing a single D1 dag-node turns "3-9" label "solo"
    let empty = "";
    let singleton = r#"<dag-node depth="D1" turns="3-9" label="solo">content</dag-node>"#;

    // @step When I call parse_dag_nodes on each input with no message_count
    let empty_nodes = parse_dag_nodes(empty, None);
    let singleton_nodes = parse_dag_nodes(singleton, None);

    // @step Then the empty input yields zero DagNodeMeta entries and the singleton input yields exactly one entry with label "solo"
    assert!(empty_nodes.is_empty());
    assert_eq!(singleton_nodes.len(), 1);
    assert_eq!(singleton_nodes[0].label, "solo");
    assert_eq!(singleton_nodes[0].turn_start, 3);
    assert_eq!(singleton_nodes[0].turn_end, 9);
    // @step Then no overlap tracing warning is emitted for either input
    //
    // (Verified by source inspection — overlap detection requires at
    //  least two same-depth nodes, so empty and singleton inputs cannot
    //  enter the rejection branch.)
}

proptest! {
    /// CMPCT-036 — Property: every pair of same-depth output nodes has
    /// disjoint `[turn_start, turn_end]` intervals.
    ///
    /// Cross-checks the Alloy model FV-003 fact `G2`
    /// (SameDepthNonOverlapping) at the parse boundary.
    #[test]
    fn cmpct_036_proptest_same_depth_pairs_are_disjoint(
        blocks in prop::collection::vec(arb_well_formed_block(), 0..8),
    ) {
        // @step Given an arbitrary DAG content string composed of well-formed dag-node blocks across depths D0, D1, and D2
        let content: String = blocks
            .iter()
            .map(|(b, _, _, _, _)| b.clone())
            .collect::<Vec<_>>()
            .join("\n");

        // @step When I call parse_dag_nodes with no message_count
        let nodes = parse_dag_nodes(&content, None);

        // @step Then for every pair of returned DagNodeMeta entries that share the same depth, their [turn_start, turn_end] intervals are disjoint
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                if std::mem::discriminant(&nodes[i].depth) == std::mem::discriminant(&nodes[j].depth) {
                    let (a, b) = (&nodes[i], &nodes[j]);
                    let disjoint = a.turn_end < b.turn_start || b.turn_end < a.turn_start;
                    prop_assert!(
                        disjoint,
                        "FV-003-b regression: same-depth nodes overlap — \
                         a=({:?},{}-{},{:?}) b=({:?},{}-{},{:?})",
                        a.depth, a.turn_start, a.turn_end, a.label,
                        b.depth, b.turn_start, b.turn_end, b.label
                    );
                }
            }
        }
    }
}
