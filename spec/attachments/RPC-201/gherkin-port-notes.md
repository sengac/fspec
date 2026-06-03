# RPC-201 — `check` — Gherkin Port Notes

**Category:** (F) Cross-cutting
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/check.ts`

## Rust Port Plan
Aggregator: runs `validate` (parse) + `validate-tags` + format-check (parse → emit → diff). All three rely on shared Gherkin modules.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Format-check = parse + re-emit; compare against original. Idempotency of emitter is critical.
- Failure aggregation must report ALL issues, not stop at first.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
