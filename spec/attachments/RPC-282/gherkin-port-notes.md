# RPC-282 — `remove-tag-from-scenario` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/remove-tag-from-scenario.ts`

## Rust Port Plan
Parses feature, finds scenario, `gherkin_tags::remove_tag(&mut scenario.tags, tag)`. Re-emit.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Search across `feat.scenarios` + `feat.rules[].scenarios`.
- Scenario lookup by exact name.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns
- §7 Tag manipulation
- §8 Re-serialization
