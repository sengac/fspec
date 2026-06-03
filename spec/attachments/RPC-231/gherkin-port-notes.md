# RPC-231 — `generate-coverage` — Gherkin Port Notes

**Category:** (E) Generation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/generate-coverage.ts`

## Rust Port Plan
For each `.feature` file: parse, extract scenario names via `gherkin_query::get_scenarios`, call `coverage_file::create_or_update` to seed sidecar entries.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)
- `codelet/fspec-core/src/coverage_file.rs` (coverage sidecars)

## Gotchas
- Use shared `coverage_file::sync_scenarios(&feat, path)` — same code path as `audit-coverage --fix`.
- Support `--dry-run`.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
