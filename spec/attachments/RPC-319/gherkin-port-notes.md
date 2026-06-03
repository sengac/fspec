# RPC-319 — `update-work-unit-status` — Gherkin Port Notes

**Category:** (F) Cross-cutting
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/update-work-unit-status.ts`

## Rust Port Plan
Phase-gate enforcement: when transitioning state, parses linked `.feature` files via `gherkin_query::find_features_by_tag(@WORK-UNIT-ID)` to check for prefill placeholders, missing scenarios, etc.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Heavy Gherkin user — depends on `gherkin_query`, `gherkin_tags`, `gherkin_validate`.
- Prefill regex is the same as `review` — share helper.
- `skipTemporalValidation` and other escape hatches must work without Gherkin parse.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


