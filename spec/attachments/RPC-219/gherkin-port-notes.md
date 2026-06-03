# RPC-219 — `delete-scenario` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/delete-scenario.ts`

## Rust Port Plan
Parses feature, `feat.scenarios.retain(|s| s.name != target)`. Re-emit. Must also update the corresponding `.feature.coverage` JSON (shared `coverage_file.rs`).

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)
- `codelet/fspec-core/src/coverage_file.rs` (coverage sidecars)

## Gotchas
- Auto-coverage update is part of contract — call `coverage_file::sync_scenarios(&feat, path)` after re-emit.
- Soft-delete? Verify TS — scenarios in feature files are hard-deleted; only work-unit items are soft-deleted.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
