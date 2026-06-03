# RPC-240 — `link-coverage` — Gherkin Port Notes

**Category:** (D) Coverage Linking
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/link-coverage.ts`

## Rust Port Plan
Uses `coverage_file` types + `gherkin_query::get_scenario_steps` to validate that test file `@step` comments match Gherkin steps line-by-line. Updates `.feature.coverage` stats.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)
- `codelet/fspec-core/src/coverage_file.rs` (coverage sidecars)

## Gotchas
- Step matching is exact-text on `step.value` (no keyword prefix? verify TS).
- `@step` comment extraction is language-agnostic — reuse TS regex.
- Skip-validation mode for reverse ACDD must be supported.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


