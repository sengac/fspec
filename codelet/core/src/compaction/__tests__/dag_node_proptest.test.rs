// FV-003 Property-Based Tests — `parse_dag_nodes` (codelet/core/src/compaction/model.rs)
//
// Cross-checks the Alloy model `dag_compaction.als` against the real Rust
// implementation. The Alloy proof relies on G1 (removed turns covered) and
// G2 (no same-depth overlap) holding at the algorithm level. These proptests
// exercise `parse_dag_nodes` directly with randomly-generated DAG content
// and assert the properties the parser DOES enforce, while documenting
// (with explicit `prop_assert`) any properties it DOES NOT.
//
// Cross-reference: codelet/core/spec/compaction/dag_compaction.als

use crate::compaction::model::DagDepth;
use crate::compaction::parse_dag_nodes;
use proptest::prelude::*;

// ────────────────────────────────────────────────────────────────────────────
// Generators — produce random but well-formed dag-node blocks
// ────────────────────────────────────────────────────────────────────────────

/// One of the three valid depths.
fn arb_depth() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("D0"), Just("D1"), Just("D2")]
}

/// A label without quotes (regex requires `[^"]+`).
/// We restrict to ASCII printable to keep counterexamples readable.
fn arb_label() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 _\\-]{1,30}"
}

/// A well-formed dag-node block with start <= end.
fn arb_well_formed_block() -> impl Strategy<Value = (String, &'static str, usize, usize, String)> {
    (arb_depth(), 0usize..200, 0usize..200, arb_label())
        .prop_map(|(depth, a, b, label)| {
            let (start, end) = if a <= b { (a, b) } else { (b, a) };
            let block = format!(
                r#"<dag-node depth="{}" turns="{}-{}" label="{}">"#,
                depth, start, end, label
            );
            (block, depth, start, end, label)
        })
}

/// A dag-node block where start MAY exceed end — used to probe the parser's
/// rejection of reversed-range input.
fn arb_arbitrary_range_block() -> impl Strategy<Value = (String, usize, usize)> {
    (arb_depth(), 0usize..200, 0usize..200, arb_label())
        .prop_map(|(depth, start, end, label)| {
            let block = format!(
                r#"<dag-node depth="{}" turns="{}-{}" label="{}">"#,
                depth, start, end, label
            );
            (block, start, end)
        })
}

/// A list of well-formed blocks, joined by newlines.
fn arb_dag_content() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_well_formed_block(), 0..8).prop_map(|blocks| {
        blocks
            .into_iter()
            .map(|(b, _, _, _, _)| b)
            .collect::<Vec<_>>()
            .join("\n")
    })
}

// ────────────────────────────────────────────────────────────────────────────
// Property tests — what `parse_dag_nodes` DOES uphold
// ────────────────────────────────────────────────────────────────────────────

proptest! {
    /// P1: Output is sorted by `turn_start` ascending.
    ///
    /// Source: model.rs:460  `nodes.sort_by_key(|n| n.turn_start);`
    /// Alloy mapping: implicit (sort order is metadata, not modelled).
    #[test]
    fn parser_output_sorted_by_turn_start(content in arb_dag_content()) {
        let nodes = parse_dag_nodes(&content, None);
        for window in nodes.windows(2) {
            prop_assert!(
                window[0].turn_start <= window[1].turn_start,
                "Output not sorted: {} > {}",
                window[0].turn_start, window[1].turn_start
            );
        }
    }

    /// P2: Every output node has depth in {D0, D1, D2}.
    ///
    /// Cross-checks Alloy assertion `DepthBounded`.
    /// Source: model.rs:433-437  match arms exhaustively cover D0/D1/D2.
    #[test]
    fn parser_output_depth_bounded(content in arb_dag_content()) {
        let nodes = parse_dag_nodes(&content, None);
        for n in &nodes {
            prop_assert!(matches!(n.depth, DagDepth::D0 | DagDepth::D1 | DagDepth::D2));
        }
    }

    /// P3: Every label is non-empty (regex requires `[^"]+`).
    #[test]
    fn parser_output_labels_nonempty(content in arb_dag_content()) {
        let nodes = parse_dag_nodes(&content, None);
        for n in &nodes {
            prop_assert!(!n.label.is_empty());
        }
    }

    /// P4: When `message_count` is provided, every `turn_end < message_count`.
    ///
    /// Source: model.rs:444-448  clamping logic.
    /// Cross-checks Alloy fact `NodeRangesWellFormed` (turn_end < message_count).
    #[test]
    fn parser_clamps_turn_end_to_message_count(
        content in arb_dag_content(),
        message_count in 1usize..100,
    ) {
        let nodes = parse_dag_nodes(&content, Some(message_count));
        for n in &nodes {
            prop_assert!(
                n.turn_end < message_count,
                "turn_end={} not clamped to message_count={}",
                n.turn_end, message_count
            );
        }
    }

    /// P5: Empty input → empty output.
    ///
    /// Cross-checks Alloy assertion `EmptySessionNoDag`.
    #[test]
    fn parser_empty_in_empty_out(_unused in any::<u8>()) {
        prop_assert!(parse_dag_nodes("", None).is_empty());
        prop_assert!(parse_dag_nodes("", Some(0)).is_empty());
        prop_assert!(parse_dag_nodes("", Some(100)).is_empty());
    }

    /// P6: Idempotence — re-serialising parsed nodes back to dag-node blocks
    /// and re-parsing yields the same metadata.
    ///
    /// This catches lossy parsing (label escaping, depth round-trip, etc.).
    #[test]
    fn parser_idempotent_roundtrip(blocks in prop::collection::vec(arb_well_formed_block(), 0..6)) {
        let content: String = blocks
            .iter()
            .map(|(b, _, _, _, _)| b.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let first = parse_dag_nodes(&content, None);

        // Re-emit
        let reemitted: String = first
            .iter()
            .map(|n| {
                let depth = match n.depth {
                    DagDepth::D0 => "D0",
                    DagDepth::D1 => "D1",
                    DagDepth::D2 => "D2",
                };
                format!(
                    r#"<dag-node depth="{}" turns="{}-{}" label="{}">"#,
                    depth, n.turn_start, n.turn_end, n.label
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let second = parse_dag_nodes(&reemitted, None);
        prop_assert_eq!(first, second);
    }

    /// P7: Output count never exceeds count of well-formed dag-node blocks
    /// in input.
    ///
    /// Combined with P6 idempotence, gives a coarse check that the parser
    /// doesn't fabricate or duplicate entries.
    #[test]
    fn parser_output_count_bounded_by_input_blocks(
        blocks in prop::collection::vec(arb_well_formed_block(), 0..6),
    ) {
        let content: String = blocks
            .iter()
            .map(|(b, _, _, _, _)| b.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let nodes = parse_dag_nodes(&content, None);
        prop_assert!(nodes.len() <= blocks.len());
    }

    /// P8: When all input blocks are well-formed (start <= end) and within
    /// `message_count`, every output node satisfies turn_start <= turn_end.
    ///
    /// This is a CONDITIONAL property — see P9 for the unconditional case.
    #[test]
    fn parser_preserves_start_le_end_when_input_well_formed(
        blocks in prop::collection::vec(arb_well_formed_block(), 0..6),
    ) {
        let content: String = blocks
            .iter()
            .map(|(b, _, _, _, _)| b.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let nodes = parse_dag_nodes(&content, None);
        for n in &nodes {
            prop_assert!(
                n.turn_start <= n.turn_end,
                "well-formed input produced reversed range: start={} end={}",
                n.turn_start, n.turn_end
            );
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Documented limitations — properties the parser does NOT enforce
//
// These tests exist to PIN the behaviour. If a future change starts
// enforcing one of these, the test will fail and prompt a deliberate
// decision (update assertion + delete this comment + close the gap in the
// Alloy model's facts).
// ────────────────────────────────────────────────────────────────────────────

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
