# RPC-332 — AST Research: wire `check` Formatting sub-check

**Date:** 2026-06-18
**Tool:** AstGrep (Rust)
**Scope:** `rust/fspec-core/src/commands/check.rs`

## Confirmed: still relevant, correctly blocked by RPC-330

## AST findings

| Symbol / site | Location | Note |
|---------------|----------|------|
| `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` | `check.rs:65` | The ported `check` entry point |
| `let format_status = "SKIP";` | `check.rs:155` | Formatting sub-check is hard-coded SKIP; never contributes to success determination |

Module doc (`check.rs:11-22`) records the divergence: TS `check` runs a third
format sub-check (re-serialise each feature file via `formatGherkinDocument` and
compare byte-for-byte); the Rust port reports SKIP because the formatter
(`io/gherkin_format.rs`) was not yet byte-parity.

## Remaining scope (once RPC-330 lands)

1. For each feature file: parse → `format_feature` → compare against on-disk bytes.
2. Report PASS/FAIL (`"Formatting check failed: <file> needs formatting"`).
3. Wire `format_status` into the success determination (FAIL fails the run; the
   current SKIP semantics are removed).

## Dependency

`blockedBy: [RPC-330]` — wiring this before description blank-line preservation
lands would produce false FAILs on properly-formatted files.
