# RPC-311 — `unlink-coverage` — Gherkin Port Notes

**Category:** (D) Coverage Linking
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/unlink-coverage.ts`

## Rust Port Plan
Pure read/write of `.feature.coverage` JSON. No Gherkin parsing needed.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)
- `codelet/fspec-core/src/coverage_file.rs` (coverage sidecars)

## Gotchas
- No `gherkin` dependency required if implementation is restricted to JSON.
- Verify against TS — if it parses the feature for any reason, mirror.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


