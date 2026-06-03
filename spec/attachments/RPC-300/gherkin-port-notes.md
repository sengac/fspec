# RPC-300 — `show-coverage` — Gherkin Port Notes

**Category:** (D) Coverage Linking
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/show-coverage.ts`

## Rust Port Plan
Pure read of `.feature.coverage` JSON. No Gherkin parsing required for display, but parsing IS required if showing scenario-level details that aren't in the coverage file.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)
- `codelet/fspec-core/src/coverage_file.rs` (coverage sidecars)

## Gotchas
- Most of this command is JSON read, not Gherkin.
- Only parse feature when displaying step-level coverage.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


