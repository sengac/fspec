# RPC-221 — `delete-step` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/delete-step.ts`

## Rust Port Plan
Parses feature, finds scenario by name, removes step matching text/keyword. Re-emit.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Step lookup by text/keyword should match TS matcher (substring? exact? case sensitivity?).
- Step deletion may change `And`/`But` resolution semantics — Rust `StepType` is set at parse time so re-emit is unaffected.
- If deleted step was the only Given/When/Then, downstream `And`/`But` may dangle — verify TS warns or refuses.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
