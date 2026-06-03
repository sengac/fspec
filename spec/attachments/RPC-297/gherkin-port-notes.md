# RPC-297 — `search-scenarios` — Gherkin Port Notes

**Category:** (C) Read/Query
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/search-scenarios.ts`

## Rust Port Plan
Uses `gherkin_query::search_scenarios(query, regex_flag)` — searches scenario names and step text. Supports substring and regex modes.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Use the `regex` crate (already in workspace).
- Output structure must include feature path, scenario name, matching step (with line number from `step.position.line`).

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


