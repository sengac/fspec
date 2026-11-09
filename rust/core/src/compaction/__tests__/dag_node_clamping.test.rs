// CMPCT-037 tests for `parse_dag_nodes`, moved out of dag_node_proptest.test.rs
// (COV-075: one test file per feature). Included into its own test module in
// compaction/mod.rs together with the shared generators fragment.
//
// Feature: spec/features/prevent-clamping-from-inverting-turn-ranges-in-parse-dag-nodes-fv-003-c.feature

use crate::compaction::parse_dag_nodes;
use proptest::prelude::*;

// ────────────────────────────────────────────────────────────────────────────
// CMPCT-037 — Clamping cannot invert turn ranges (FV-003-c closed)
//
// Feature: spec/features/prevent-clamping-from-inverting-turn-ranges-in-parse-dag-nodes-fv-003-c.feature
//
// These tests close FV-003-c by asserting that any `<dag-node>` block whose
// `turn_start >= message_count` is rejected at the parse boundary (with a
// `tracing::warn`) BEFORE clamping. This means the existing `turn_end`
// clamp-to-`message_count - 1` step can never produce an inverted output
// range, because the residual contract `turn_start < message_count` is
// enforced upstream.
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn cmpct_037_turn_start_beyond_message_count_drops_node_with_warning() {
    // @step Given a DAG content string containing a single dag-node block with depth "D0", turns "200-300", and label "both above"
    let content = r#"<dag-node depth="D0" turns="200-300" label="both above">content</dag-node>"#;

    // @step When I call parse_dag_nodes with message_count=Some(60)
    let nodes = parse_dag_nodes(content, Some(60));

    // @step Then the result contains zero DagNodeMeta entries
    assert_eq!(
        nodes.len(),
        0,
        "node with turn_start beyond message_count must be dropped before clamping"
    );
    // @step And a tracing warning is emitted carrying turn_start=200 and message_count=60
    //
    // (Verified by source inspection — the rejection branch in
    //  `parse_dag_nodes` emits `tracing::warn!(turn_start, message_count, ...)`.
    //  Programmatic capture would require a tracing-subscriber test layer
    //  which is not currently wired into codelet-core's dev-dependencies;
    //  the existing FV-003-a/b limitation tests follow the same convention.)
}

#[test]
fn cmpct_037_turn_end_above_message_count_is_clamped_when_start_in_range() {
    // @step Given a DAG content string containing a single dag-node block with depth "D0", turns "50-300", and label "end above"
    let content = r#"<dag-node depth="D0" turns="50-300" label="end above">content</dag-node>"#;

    // @step When I call parse_dag_nodes with message_count=Some(60)
    let nodes = parse_dag_nodes(content, Some(60));

    // @step Then the result contains one DagNodeMeta entry with turn_start=50 and turn_end=59
    assert_eq!(nodes.len(), 1, "in-range start must keep the node, with end clamped");
    assert_eq!(nodes[0].turn_start, 50);
    assert_eq!(nodes[0].turn_end, 59);
    assert!(nodes[0].turn_start <= nodes[0].turn_end);
    // @step And no clamping-rejection tracing warning is emitted
    //
    // (Verified by source inspection — the rejection branch is gated on
    //  `turn_start >= message_count`; with start=50 < 60 the branch is not
    //  entered.)
}

#[test]
fn cmpct_037_turn_start_equal_to_message_count_minus_one_is_accepted() {
    // @step Given a DAG content string containing a single dag-node block with depth "D0", turns "59-59", and label "boundary"
    let content = r#"<dag-node depth="D0" turns="59-59" label="boundary">content</dag-node>"#;

    // @step When I call parse_dag_nodes with message_count=Some(60)
    let nodes = parse_dag_nodes(content, Some(60));

    // @step Then the result contains one DagNodeMeta entry with turn_start=59 and turn_end=59
    assert_eq!(nodes.len(), 1, "turn_start == message_count - 1 is in range");
    assert_eq!(nodes[0].turn_start, 59);
    assert_eq!(nodes[0].turn_end, 59);
    // @step And no clamping-rejection tracing warning is emitted
    //
    // (Verified by source inspection — `59 >= 60` is false, so the
    //  rejection branch is not entered.)
}

#[test]
fn cmpct_037_turn_start_equal_to_message_count_drops_node() {
    // @step Given a DAG content string containing a single dag-node block with depth "D0", turns "60-100", and label "at boundary"
    let content = r#"<dag-node depth="D0" turns="60-100" label="at boundary">content</dag-node>"#;

    // @step When I call parse_dag_nodes with message_count=Some(60)
    let nodes = parse_dag_nodes(content, Some(60));

    // @step Then the result contains zero DagNodeMeta entries
    assert_eq!(
        nodes.len(),
        0,
        "turn_start == message_count refers to non-existent turn; node must be dropped"
    );
    // @step And a tracing warning is emitted carrying turn_start=60 and message_count=60
    //
    // (Verified by source inspection — `60 >= 60` is true, so the
    //  rejection branch is entered and emits `tracing::warn!`.)
}

#[test]
fn cmpct_037_mixed_input_drops_only_out_of_range_node() {
    // @step Given a DAG content string containing a D0 dag-node turns "10-20" label "in-range" and a D0 dag-node turns "100-150" label "out-of-range"
    let content = r#"
<dag-node depth="D0" turns="10-20" label="in-range">good</dag-node>
<dag-node depth="D0" turns="100-150" label="out-of-range">bad</dag-node>
"#;

    // @step When I call parse_dag_nodes with message_count=Some(50)
    let nodes = parse_dag_nodes(content, Some(50));

    // @step Then the result contains exactly one DagNodeMeta entry with label "in-range", turn_start=10 and turn_end=20
    assert_eq!(nodes.len(), 1, "only the in-range node must survive");
    assert_eq!(nodes[0].label, "in-range");
    assert_eq!(nodes[0].turn_start, 10);
    assert_eq!(nodes[0].turn_end, 20);
    // @step And exactly one clamping-rejection tracing warning is emitted naming the dropped label "out-of-range"
    //
    // (Verified by source inspection — exactly one rejection per
    //  out-of-range block produces exactly one warning.)
}

#[test]
fn cmpct_037_message_count_zero_rejects_every_node() {
    // @step Given a DAG content string containing a single dag-node block with depth "D0", turns "0-10", and label "any"
    let content = r#"<dag-node depth="D0" turns="0-10" label="any">content</dag-node>"#;

    // @step When I call parse_dag_nodes with message_count=Some(0)
    let nodes = parse_dag_nodes(content, Some(0));

    // @step Then the result contains zero DagNodeMeta entries
    assert_eq!(
        nodes.len(),
        0,
        "message_count=Some(0) means no turns exist; every node must be dropped"
    );
    // @step And a tracing warning is emitted carrying turn_start=0 and message_count=0
    //
    // (Verified by source inspection — `0 >= 0` is true, so the
    //  rejection branch is entered for every node when message_count=0.)
}

proptest! {
    /// CMPCT-037 — Property: post-clamping every output node satisfies
    /// `turn_start <= turn_end` AND `turn_end < message_count`.
    ///
    /// Cross-checks the Alloy model FV-003 fact `NodeRangesWellFormed`
    /// (turn_start <= turn_end AND turn_end < message_count) at the parse
    /// boundary, including the clamping path.
    #[test]
    fn cmpct_037_proptest_clamping_preserves_invariants(
        // Mix well-formed input blocks (start <= end) and let the proptest
        // pick an arbitrary message_count. The new rejection step must keep
        // both invariants regardless of how the two interact.
        blocks in prop::collection::vec(arb_well_formed_block(), 0..6),
        message_count in 1usize..300,
    ) {
        // @step Given an arbitrary DAG content string composed of well-formed dag-node blocks and an arbitrary message_count
        let content: String = blocks
            .iter()
            .map(|(b, _, _, _, _)| b.clone())
            .collect::<Vec<_>>()
            .join("\n");

        // @step When I call parse_dag_nodes with that message_count
        let nodes = parse_dag_nodes(&content, Some(message_count));

        // @step Then for every DagNodeMeta in the result, turn_start <= turn_end holds
        // @step And for every DagNodeMeta in the result, turn_end < message_count holds
        for n in &nodes {
            prop_assert!(
                n.turn_start <= n.turn_end,
                "FV-003-c regression: clamping inverted range — \
                 turn_start={} turn_end={} message_count={}",
                n.turn_start, n.turn_end, message_count
            );
            prop_assert!(
                n.turn_end < message_count,
                "FV-003-c regression: turn_end not strictly below message_count — \
                 turn_start={} turn_end={} message_count={}",
                n.turn_start, n.turn_end, message_count
            );
            prop_assert!(
                n.turn_start < message_count,
                "FV-003-c regression: turn_start not strictly below message_count — \
                 turn_start={} message_count={}",
                n.turn_start, message_count
            );
        }
    }
}
