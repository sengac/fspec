# RPC-293 — `retag` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/retag.ts`

## Rust Port Plan
Bulk rename `@old` → `@new` across all features. Use `gherkin_query::parse_all_features`, then for each feature mutate `feat.tags`, `feat.scenarios[].tags`, `feat.rules[].tags`, `feat.examples[].tags` in place. Re-emit each.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Cross-cutting — must touch every tag location.
- Support `--dry-run` like TS.
- Atomic-ish: collect all changes, write only after all parses succeed (no partial state).

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns
- §7 Tag manipulation
- §8 Re-serialization
