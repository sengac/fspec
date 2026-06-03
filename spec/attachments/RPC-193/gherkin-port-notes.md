# RPC-193 — `add-tag-to-feature` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/add-tag-to-feature.ts`

## Rust Port Plan
Parses feature, calls `gherkin_tags::add_tag(&mut feat.tags, tag)`. Re-emit. Tags are stored without `@` (master guide §7).

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Strip `@` prefix before storing; re-add when emitting.
- Cross-check tag registry only if `validateRegistry` option is set (TS behavior).
- Idempotent: adding existing tag is a no-op.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns
- §7 Tag manipulation
- §8 Re-serialization
