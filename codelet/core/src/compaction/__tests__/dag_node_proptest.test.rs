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
/// (lack of) range validation.
#[allow(dead_code)] // reserved for future tests probing reverse-range edge cases
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

#[test]
fn limitation_parser_does_not_validate_start_le_end() {
    // Input has turn_start (50) > turn_end (10). The parser accepts it.
    let content = r#"<dag-node depth="D0" turns="50-10" label="reversed">"#;
    let nodes = parse_dag_nodes(content, None);
    assert_eq!(nodes.len(), 1, "parser accepts reversed ranges");
    assert_eq!(nodes[0].turn_start, 50);
    assert_eq!(nodes[0].turn_end, 10);
    // Conclusion (FV-003 finding): `parse_dag_nodes` does not enforce
    // turn_start <= turn_end. This is an unenforced invariant — downstream
    // code MUST either validate or be tolerant. See FORMAL_VERIFICATION.md.
}

#[test]
fn limitation_parser_does_not_reject_overlap() {
    // Two D0 nodes with overlapping ranges — accepted with a tracing::warn.
    let content = r#"
<dag-node depth="D0" turns="0-50" label="first">
<dag-node depth="D0" turns="30-80" label="second">
"#;
    let nodes = parse_dag_nodes(content, None);
    assert_eq!(nodes.len(), 2, "parser accepts same-depth overlapping ranges");
    // Conclusion (FV-003 finding): G2 (SameDepthNonOverlapping) is NOT
    // enforced by the parser. The Alloy model FV-003 takes G2 as a fact
    // about the *upstream* compaction algorithm. Downstream consumers of
    // `parse_dag_nodes` output cannot assume non-overlap.
}

#[test]
fn limitation_clamping_can_invert_range() {
    // turn_start (50) is below message_count (60), turn_end (200) is clamped
    // to 59. Result: turn_start=50, turn_end=59 (still valid).
    //
    // But if turn_start (200) > message_count, it stays 200; turn_end gets
    // clamped to message_count - 1. Result: turn_start > turn_end.
    let content = r#"<dag-node depth="D0" turns="200-300" label="both above">"#;
    let nodes = parse_dag_nodes(content, Some(60));
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].turn_start, 200);
    assert_eq!(nodes[0].turn_end, 59); // clamped — now start > end
    // Conclusion (FV-003 finding): clamping can produce an inverted range
    // (turn_start > turn_end). This is the same root cause as
    // `limitation_parser_does_not_validate_start_le_end`.
}
