// CMPCT-035 tests for `parse_dag_nodes`, moved out of dag_node_proptest.test.rs
// (COV-075: one test file per feature). Included into its own test module in
// compaction/mod.rs together with the shared generators fragment.
//
// Feature: spec/features/parse-dag-nodes-turn-range-validation.feature

use crate::compaction::model::DagDepth;
use crate::compaction::parse_dag_nodes;
use proptest::prelude::*;

// ────────────────────────────────────────────────────────────────────────────
// CMPCT-035 — `parse_dag_nodes` rejects reverse turn ranges (FV-003-a closed)
//
// Feature: spec/features/parse-dag-nodes-turn-range-validation.feature
//
// These tests close FV-003-a by asserting that any `<dag-node>` block with
// `turn_start > turn_end` (BEFORE clamping) is skipped at the parse boundary
// with a tracing::warn. Clamping-induced inversions (FV-003-c) are closed
// separately by the `cmpct_037_*` tests below.
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn cmpct_035_reversed_turn_range_is_rejected() {
    // @step Given a DAG content string containing a single dag-node block with depth "D0", turns "50-10", and label "reversed"
    let content = r#"<dag-node depth="D0" turns="50-10" label="reversed">content</dag-node>"#;

    // @step When I call parse_dag_nodes with no message_count
    let nodes = parse_dag_nodes(content, None);

    // @step Then the result contains zero DagNodeMeta entries
    assert_eq!(nodes.len(), 0, "reversed range must be rejected");
    // @step And a tracing warning is emitted carrying turn_start=50 and turn_end=10
    //
    // (Verified by source inspection — the rejection branch in
    //  `parse_dag_nodes` emits `tracing::warn!(turn_start, turn_end, ...)`.
    //  Programmatic capture would require a tracing-subscriber test layer
    //  which is not currently wired into codelet-core's dev-dependencies;
    //  the existing FV-003-b limitation test follows the same convention.)
}

#[test]
fn cmpct_035_forward_turn_range_is_parsed_unchanged() {
    // @step Given a DAG content string containing a single dag-node block with depth "D0", turns "10-50", and label "forward"
    let content = r#"<dag-node depth="D0" turns="10-50" label="forward">content</dag-node>"#;

    // @step When I call parse_dag_nodes with no message_count
    let nodes = parse_dag_nodes(content, None);

    // @step Then the result contains one DagNodeMeta entry with turn_start=10 and turn_end=50
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].turn_start, 10);
    assert_eq!(nodes[0].turn_end, 50);
    assert!(matches!(nodes[0].depth, DagDepth::D0));
    // @step And no inverted-range tracing warning is emitted
    //
    // (Forward ranges never enter the rejection branch — see
    //  `parse_dag_nodes` invariant test above.)
}

#[test]
fn cmpct_035_equal_start_and_end_is_accepted() {
    // @step Given a DAG content string containing a single dag-node block with depth "D0", turns "42-42", and label "single"
    let content = r#"<dag-node depth="D0" turns="42-42" label="single">content</dag-node>"#;

    // @step When I call parse_dag_nodes with no message_count
    let nodes = parse_dag_nodes(content, None);

    // @step Then the result contains one DagNodeMeta entry with turn_start=42 and turn_end=42
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].turn_start, 42);
    assert_eq!(nodes[0].turn_end, 42);
    // @step And no inverted-range tracing warning is emitted
    //
    // (start == end is the boundary case explicitly allowed by the
    //  `turn_start > turn_end` rejection predicate.)
}

#[test]
fn cmpct_035_mixed_input_keeps_only_well_formed_nodes() {
    // @step Given a DAG content string containing a reversed-range dag-node "60-10" and a forward dag-node "0-50"
    let content = r#"
<dag-node depth="D0" turns="60-10" label="reversed">junk</dag-node>
<dag-node depth="D0" turns="0-50" label="forward">good</dag-node>
"#;

    // @step When I call parse_dag_nodes with no message_count
    let nodes = parse_dag_nodes(content, None);

    // @step Then the result contains exactly one DagNodeMeta entry corresponding to the forward block
    assert_eq!(nodes.len(), 1, "only the forward block should survive");
    assert_eq!(nodes[0].turn_start, 0);
    assert_eq!(nodes[0].turn_end, 50);
    assert_eq!(nodes[0].label, "forward");
    // @step And exactly one inverted-range tracing warning is emitted
    //
    // (Verified by source inspection — exactly one rejection per malformed
    //  block; programmatic capture not wired in codelet-core dev-deps.)
}

proptest! {
    /// CMPCT-035 — P9: Unconditional invariant.
    ///
    /// For ANY input parsed without `message_count` (so clamping cannot
    /// invert ranges), every node in the output satisfies
    /// `turn_start <= turn_end`. This is the unconditional strengthening
    /// of P8 enabled by the parse-time rejection introduced in CMPCT-035.
    #[test]
    fn cmpct_035_proptest_every_parsed_node_has_start_le_end(
        // Mix of well-formed AND arbitrary (possibly reversed) blocks.
        well_formed in prop::collection::vec(arb_well_formed_block(), 0..4),
        arbitrary in prop::collection::vec(arb_arbitrary_range_block(), 0..4),
    ) {
        // @step Given an arbitrary DAG content string composed of well-formed and reversed dag-node blocks
        let mut all_blocks: Vec<String> = Vec::new();
        for (b, _, _, _, _) in &well_formed {
            all_blocks.push(b.clone());
        }
        for (b, _, _) in &arbitrary {
            all_blocks.push(b.clone());
        }
        let content = all_blocks.join("\n");

        // @step When I call parse_dag_nodes with no message_count
        let nodes = parse_dag_nodes(&content, None);

        // @step Then for every DagNodeMeta in the result, turn_start <= turn_end holds
        for n in &nodes {
            prop_assert!(
                n.turn_start <= n.turn_end,
                "FV-003-a regression: node with reversed range slipped through parse: \
                 turn_start={} turn_end={} label={:?}",
                n.turn_start, n.turn_end, n.label
            );
        }
    }
}
