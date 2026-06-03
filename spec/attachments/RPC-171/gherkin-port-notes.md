# RPC-171 — `add-background` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/add-background.ts`

## Rust Port Plan
Parses feature, builds a `Background` node via `Background::builder()` with `keyword`, `name`, `steps`, sets `feat.background = Some(bg)`, re-emits.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Backgrounds have no tags (spec-correct, parser-correct).
- A feature can have only ONE background — overwriting must mirror TS behavior (error vs replace).
- Each `Rule` may have its own background — clarify which one is targeted by reading TS source.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
