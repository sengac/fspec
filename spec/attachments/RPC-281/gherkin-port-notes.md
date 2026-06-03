# RPC-281 — `remove-tag-from-feature` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/remove-tag-from-feature.ts`

## Rust Port Plan
Parses feature, `gherkin_tags::remove_tag(&mut feat.tags, tag)`. Re-emit.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Compare bare (no `@`) when matching.
- Idempotent: removing absent tag is a no-op.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns
- §7 Tag manipulation
- §8 Re-serialization
