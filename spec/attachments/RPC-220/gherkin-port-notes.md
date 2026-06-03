# RPC-220 — `delete-scenarios` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/delete-scenarios-by-tag.ts`

## Rust Port Plan
Parses feature(s), `feat.scenarios.retain(|s| !s.tags.iter().any(|t| t == target))`. Re-emit + sync coverage.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Tag stored without `@` in AST — compare bare.
- Support `--dry-run` like TS.
- Also scan `Rule.scenarios` if rules are present.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
