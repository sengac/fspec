# RPC-167 — `add-architecture` — Gherkin Port Notes

**Category:** (B) AST Mutation
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Source
`src/commands/add-architecture.ts`

## Rust Port Plan
Parses the target `.feature`, appends an architecture-notes block (typically as `description` text attached to the feature, or as a Gherkin comment in TS). Because `gherkin` v0.16 discards comments (master guide §2), architecture notes MUST be stored on `Feature.description` or in a sidecar file — choose the same channel the TS version uses by reading `src/commands/add-architecture.ts` first.

## Shared Rust Modules Required
- `codelet/fspec-core/src/gherkin_io.rs` (load/save)
- `codelet/fspec-core/src/gherkin_emit.rs` (canonical text writer)
- `codelet/fspec-core/src/gherkin_query.rs` (read helpers)
- `codelet/fspec-core/src/gherkin_tags.rs` (tag manipulation)


## Gotchas
- Architecture notes survive re-emit only if stored as `description`. If TS stores them as `#` comments, port logic to sidecar JSON instead.
- Use shared `gherkin_io.rs` (load) + `gherkin_emit.rs` (save).

## Reference Snippets
See master guide sections:
- §3 AST schema
- §4 Parsing
- §6 Mutation patterns

- §8 Re-serialization
