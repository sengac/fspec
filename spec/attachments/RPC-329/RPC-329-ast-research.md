# RPC-329 — AST Research: Gherkin raw parser-error-text divergence

**Date:** 2026-06-18
**Tool:** AstGrep (Rust)
**Scope:** `codelet/fspec-core/src/commands/validate.rs`, `codelet/fspec-core/src/io/gherkin.rs`, `gherkin-0.16.0` crate

## Re-scope summary

The original card described THREE symptom classes. Two are now mitigated in the
ported code; only the **raw parser-error message text + top-of-file line-number**
divergence remains. This card is re-scoped to that residual gap.

## AST findings — what already exists (parity achieved)

| Symbol | Location | Status |
|--------|----------|--------|
| `fn get_suggestion(error_message: &str) -> Option<String>` | `validate.rs:356` | PORTED — heuristics mirror TS `getSuggestion` (`src/commands/validate.ts:229-254`) |
| `fn check_for_common_issues(content: &str) -> Vec<ValidationError>` | `validate.rs:290` | PORTED — triple-quote + >2-blank-line heuristics |
| `fn extract_line(message: &str) -> usize` | `validate.rs:279` | PORTED — parses `Error at <L>:<C>` |
| `pub fn parse_feature_lenient(content: &str) -> Result<Feature, ParseError>` | `gherkin.rs:62` | PORTED — two-stage lenient parse + sanitiser absorbs the `"""`-in-description classification disagreement |

## AST findings — residual divergence (the actual remaining work)

- `validate.rs:279 extract_line` reads Rust `gherkin-0.16` messages of the form
  `Error at <L>:<C>: {…}`. For top-of-file errors the Rust parser reports
  `1:…` → "Line 1", while TS cucumber reports "Line 0" (`location?.line || 0`).
- The raw embedded parser-error string differs: Rust `gherkin-0.16`
  `Error at L:C: {"unknown keyword"}` vs TS `@cucumber/gherkin`
  `expected: #FeatureLine, #Comment ...`.
- Root cause is structural: `codelet/Cargo.toml:135` pins `gherkin = "0.16"`
  whereas TS uses `@cucumber/gherkin` — the two parsers emit different message
  vocabularies and line bases.

## Recommended remaining scope

1. Add a cucumber-compatible error formatter (shared in `io/gherkin.rs`) that maps
   `gherkin-0.16` `ParseError` display text into the cucumber token vocabulary.
2. Align top-of-file line number to `0` to match TS.
3. Drive with a dedicated malformed-`.feature` fixture (no Feature keyword) that
   asserts raw text + "Line 0" parity.
