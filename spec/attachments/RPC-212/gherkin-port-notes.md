# RPC-212 — `create-feature` — Gherkin Port Notes

**Category:** (E) Generation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/create-feature.ts`

## Rust Port Plan
Writes a NEW `.feature` file with a Gherkin template, then calls `coverage_file::create` to seed the matching `.feature.coverage`. Initial template can be a static string OR built via `Feature::builder()` + emitter.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)
- `codelet/fspec-core/src/coverage_file.rs` (coverage sidecars)

## Gotchas
- Prefer builder + emitter to ensure the new file matches canonical format byte-for-byte.
- File-naming convention: kebab-case capability name (NOT work-unit-id).
- Tag with `@wip` by default? Verify TS.

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
