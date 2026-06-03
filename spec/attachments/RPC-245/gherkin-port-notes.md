# RPC-245 — `list-features` — Gherkin Port Notes

**Category:** (C) Read/Query
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/list-features.ts`

## Rust Port Plan
Globs `spec/features/*.feature`, parses each, emits feature name + tags + scenario count + file path.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Use `gherkin_query::parse_all_features`.
- Filter by `--tag` (with or without `@`).

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


