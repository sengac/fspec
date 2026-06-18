# RPC-330 — AST Research: format drops inter-paragraph blank lines in descriptions

**Date:** 2026-06-18
**Tool:** AstGrep (Rust)
**Scope:** `codelet/fspec-core/src/io/gherkin_format.rs`, `gherkin-0.16.0` crate parser

## Confirmed: real, unfixed bug

## AST findings — formatter side

| Symbol | Location | Note |
|--------|----------|------|
| `pub fn format_feature(feature: &Feature) -> String` | `gherkin_format.rs:36` | Formats from the PARSED `feature.description`, not raw source |
| `fn format_description(description: &str, lines: &mut Vec<String>, indent_level: usize)` | `gherkin_format.rs:351` | Collapses runs of blank lines (`consecutive_blank < 2`) but can only emit blanks that survived parsing |
| `fn dedent(s: &str) -> String` | `gherkin_format.rs:260` | Doc-string body re-dedent (already fixed for doc strings) |

## AST findings — root cause in the parser (upstream crate)

`gherkin-0.16.0/src/parser.rs:381`:

```
rule description(excluded: &[&str]) -> Option<String>
    = d:(description_line(excluded) ** _) __ {
        let d = d.join("\n");
        ...
    }
```

The `** _` operator makes blank lines the **separator** between
`description_line`s, so blank lines between paragraphs are CONSUMED and never
returned in `Feature.description`. By the time `format_description` runs the
blank lines are already gone.

## Recommended fix

Mirror the doc-string fix (RPC already done there): re-extract feature/scenario
description text from the **raw source** between the header line and the first
child construct, instead of relying on the lossy parsed `description` field.

## Downstream

Blocks **RPC-332** (`blocks: [RPC-332]`) — the `check` Formatting sub-check stays
SKIP until this lands and the formatter reaches byte-parity.
