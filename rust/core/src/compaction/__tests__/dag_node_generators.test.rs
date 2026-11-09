// Shared proptest generators for the parse_dag_nodes test fragments (FV-003,
// CMPCT-035/036/037), included into each of their test modules in
// compaction/mod.rs (COV-075 split). The including module provides the
// `DagDepth` and `proptest::prelude` imports.

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
                r#"<dag-node depth="{depth}" turns="{start}-{end}" label="{label}">"#,
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
                r#"<dag-node depth="{depth}" turns="{start}-{end}" label="{label}">"#,
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
