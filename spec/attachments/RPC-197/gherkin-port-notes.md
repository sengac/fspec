# RPC-197 — `audit-coverage` — Gherkin Port Notes

**Category:** (D) Coverage Linking
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/audit-coverage.ts`

## Rust Port Plan
Reads `.feature.coverage` JSON sidecars (`coverage_file::load`). Cross-references scenario names against the parsed `.feature` AST to detect orphans / missing entries. Optional `--fix` flag rewrites the coverage file.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)
- `codelet/fspec-core/src/coverage_file.rs` (coverage sidecars)

## Gotchas
- Both feature parse and coverage JSON load required.
- Fix mode = call `coverage_file::sync_scenarios(&feat, path)`.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


