# RPC-230 — `format` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/format.ts`

## Rust Port Plan
THE canonical writer of `gherkin_emit.rs`. Parses every (or specified) `.feature`, re-emits in canonical layout, writes back if changed.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- **Highest priority shared module.** Build `gherkin_emit.rs` carefully here — every other (B) mutator inherits its output style.
- Comment preservation is the central gap (master guide §2). Either drop, or implement a sidecar comment collector keyed by source line numbers.
- Idempotency check: parse → emit → parse → emit must yield byte-identical output.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
