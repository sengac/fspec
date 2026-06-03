# RPC-218 — `delete-features` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/delete-features-by-tag.ts`

## Rust Port Plan
Parses every `.feature` (use shared `gherkin_query::parse_all_features`), filters those whose `feat.tags` contains the target tag, deletes the file from disk. Also deletes the matching `.feature.coverage` sidecar.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Bulk operation — support `--dry-run` flag like TS.
- Only checks FEATURE-level tags, not scenario-level (verify in TS).
- Also delete attachments? Check TS behavior.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
