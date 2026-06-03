# RPC-310 — `tag-stats` — Gherkin Port Notes

**Category:** (C) Read/Query
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/tag-stats.ts`

## Rust Port Plan
Parses all features, aggregates tag usage counts across feature/scenario/rule/examples levels.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Don't forget `feat.rules[].tags`, `feat.examples[].tags`.
- Output histogram sorted by count desc, then alphabetic.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns
- §7 Tag manipulation

