# RPC-320 — `validate` — Gherkin Port Notes

**Category:** (A) Parse/Validate
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Behavior
- Imports `@cucumber/gherkin` (`AstBuilder`, `GherkinClassicTokenMatcher`, `Parser`) and `@cucumber/messages`.
- Walks a glob of `.feature` files; for each, parses and reports any syntax errors with line/col + a source snippet window.
- Exits non-zero on any failure.

## Rust Port Plan
- Use `gherkin::Feature::parse(src, GherkinEnv::default())` per file.
- On `Err(ParseError)`: format the error string. Because `ParseError.position`/`expected` are private (see master guide §5), keep your own line cache built from the source string to produce the snippet UI the TS version provides.
- Use the shared `gherkin_validate.rs` module so `check` (RPC-201) and `update-work-unit-status` (RPC-319) share the same error format.

## Key Files
- TS source: `src/commands/validate.ts`
- Shared Rust module to create first: `codelet/fspec-core/src/gherkin_validate.rs`

## Gotchas
- Parser auto-appends a trailing newline (`gherkin-rs/src/lib.rs:236-240`) — don't claim "fixed by adding newline" in error messages.
- A successful parse does NOT imply tags are valid. `validate-tags` (RPC-324) is separate.
