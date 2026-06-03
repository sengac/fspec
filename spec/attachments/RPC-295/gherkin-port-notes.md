# RPC-295 — `review` — Gherkin Port Notes

**Category:** (F) Cross-cutting
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/review.ts`

## Rust Port Plan
End-to-end work-unit review. Parses linked feature files to verify scenarios exist, prefills cleared, coverage links present.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Reuse `gherkin_query::find_features_by_tag` for `@WORK-UNIT-ID` lookup.
- Prefill detection: scan `scenario.name`, `step.value`, `feat.description` for `[role]`/`[action]`/`[benefit]` placeholders.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns


