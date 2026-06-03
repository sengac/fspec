# RPC-308 — `show-work-unit` — Gherkin Port Notes

**Category:** (C) Read/Query
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/show-work-unit.ts`

## Rust Port Plan
Parses feature files to locate `@WORK-UNIT-ID` tags via `gherkin_tags::extract_work_unit_tags`. Displays linked scenarios alongside Example Mapping data.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Work-unit tag regex: `^[A-Z]+-\d+$` (after stripping `@`).
- Must search both feature- and scenario-level tags.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns
- §7 Tag manipulation

