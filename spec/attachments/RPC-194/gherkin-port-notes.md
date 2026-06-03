# RPC-194 — `add-tag-to-scenario` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/add-tag-to-scenario.ts`

## Rust Port Plan
Parses feature, finds scenario by name, calls `gherkin_tags::add_tag(&mut scenario.tags, tag)`. Re-emit.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Scenario lookup is name-based — exact match.
- Must search both `feat.scenarios` and `feat.rules[].scenarios`.
- Work-unit tag (`@AUTH-001`) detection via `is_work_unit_tag` helper.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns
- §7 Tag manipulation
- §8 Re-serialization
