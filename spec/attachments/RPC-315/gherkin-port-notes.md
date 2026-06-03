# RPC-315 — `update-step` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/update-step.ts`

## Rust Port Plan
Parses feature, finds scenario, finds step by old text, mutates `step.value` and optionally `step.keyword`/`step.ty`. Re-emit.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Changing keyword (e.g. Given → When) requires updating both `step.keyword` (raw) AND `step.ty` (resolved).
- Touching `And`/`But` keyword may need to re-derive `ty` from the previous concrete step.
- @step comment in test files no longer matches after change — link-coverage may break; verify TS warns.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
