# RPC-307 — `show-test-patterns` — Gherkin Port Notes

**Category:** (C) Read/Query
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/show-test-patterns.ts`

## Rust Port Plan
Uses `gherkin_query::parse_all_features` to find shared step patterns across scenarios filtered by tag. Read-only.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Step text deduplication should match TS algorithm.
- Include coverage hints when `includeCoverage: true`.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


