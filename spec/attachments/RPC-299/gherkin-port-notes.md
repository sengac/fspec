# RPC-299 — `show-acceptance-criteria` — Gherkin Port Notes

**Category:** (C) Read/Query
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/show-acceptance-criteria.ts`

## Rust Port Plan
Parses features, filters by tag, formats each scenario as markdown or JSON acceptance criteria block.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Markdown formatter is bespoke — model after TS output character-for-character.
- JSON output benefits from `serde` feature.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


