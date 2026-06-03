# RPC-190 — `add-scenario` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/add-scenario.ts`

## Rust Port Plan
Parses feature, builds a `Scenario` via `Scenario::builder()` with `keyword: "Scenario"`, `name`, empty `steps: vec![]`. Push onto `feat.scenarios`. Re-emit.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Duplicate name detection should mirror TS exactly.
- Insertion position (end of feature vs end of last rule) must match TS.
- `Scenario::builder().examples(vec![])` is implicit via TypedBuilder default — only set for outlines.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
