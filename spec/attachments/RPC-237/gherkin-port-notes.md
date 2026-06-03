# RPC-237 — `get-scenarios` — Gherkin Port Notes

**Category:** (C) Read/Query
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/get-scenarios.ts`

## Rust Port Plan
Reads features, filters scenarios by tag, outputs as text/json. Uses `gherkin::Feature` `serde` derive for JSON output (enable `serde` feature).

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Need `serde` feature on `gherkin` crate.
- Output structure must match TS exactly — name, tags (with `@` prefix added back), steps, etc.
- Don't forget scenarios inside `Rule.scenarios`.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


