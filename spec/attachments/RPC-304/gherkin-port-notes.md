# RPC-304 — `show-feature` — Gherkin Port Notes

**Category:** (C) Read/Query
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/show-feature.ts`

## Rust Port Plan
Parses one feature, pretty-prints. Uses `gherkin_tags::extract_work_unit_tags` to surface work-unit linkage.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- This is a READ — do NOT re-emit & write back.
- Output format must match TS (which uses chalk colors).

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


