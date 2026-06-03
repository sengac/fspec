# RPC-234 — `generate-scenarios` — Gherkin Port Notes

**Category:** (E) Generation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/generate-scenarios.ts`

## Rust Port Plan
**The main user of the `Feature` BUILDER.** Translates Example Mapping (rules, examples, user story) into a Gherkin AST via `Feature::builder() / Scenario::builder() / Step::builder()`, then `gherkin_emit::emit_feature(&feat)`, writes to disk.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)
- `codelet/fspec-core/src/coverage_file.rs` (coverage sidecars)

## Gotchas
- Step keyword/type inference from example wording is the same business logic as TS — port carefully.
- If feature file already exists, merge or refuse — match TS.
- Generated file must be valid input to `Feature::parse` — write a round-trip test.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
