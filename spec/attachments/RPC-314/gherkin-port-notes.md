# RPC-314 — `update-scenario` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/update-scenario.ts`

## Rust Port Plan
Parses feature, finds scenario by old name, sets `scenario.name = new_name`. Re-emit + sync coverage (`.feature.coverage` keys by scenario name).

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)
- `codelet/fspec-core/src/coverage_file.rs` (coverage sidecars)

## Gotchas
- Auto-sync coverage file — call `coverage_file::rename_scenario(path, old, new)`.
- Lookup must match TS (case-sensitive exact match presumably).

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
