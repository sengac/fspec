# RPC-192 — `add-step` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/add-step.ts`

## Rust Port Plan
Parses feature, locates target `Scenario` by name, builds a `Step` via `Step::builder()` with `keyword: "Given "` (trailing space!), `ty: StepType::Given|When|Then`, `value`. Append to `scenario.steps`. Re-emit.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- `Step.keyword` includes the trailing space in canonical form — `"Given "`, not `"Given"`.
- `And`/`But` should set `ty` to the previous step's resolved type but keep raw keyword as `"And "` / `"But "`.
- Use shared emitter so step indentation (4 spaces) is consistent.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
