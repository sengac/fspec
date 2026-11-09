# COV-060 design: remove the unbuilt rust/tests/codex_provider_test.rs

## Problem

`rust/tests/codex_provider_test.rs` (410 lines) is never compiled:

- `rust/Cargo.toml` is a virtual workspace (`[workspace]` only, no `[package]`), so a top-level
  `rust/tests/` directory belongs to no crate and no `[[test]]` entry points at it.
- It imports `codelet::agent::…` and `codelet::providers::{CodexProvider, LlmProvider}`
  (lines 7-8); no crate named `codelet` exists in the workspace.
- It names `spec/features/codex-provider-chatgpt-backend-api-with-oauth.feature`, which does not
  exist, and asserts the 272000 / 4096 Codex defaults (lines 83-87) that BUG-330 removed.

So it lies about current behaviour and can never be run.

## Decision

Delete it. Wiring it into `codelet-providers` would mean rewriting every import and assertion; what
it set out to check is already pinned by built targets: Codex limits by
`rust/providers/tests/bug330_codex_static_limits.rs` (codex-declares-no-static-model-limits) and
the catalog path by `rust/sessions/tests/codex_catalog_limit_resolution.rs`.

A new scenario in `codex-declares-no-static-model-limits.feature`, "No unbuilt test asserts the
removed Codex default limits", is tested from the built `bug330_codex_static_limits` target: it
reads every `rust/tests/*.rs` and fails while any file there pairs `CodexProvider` with `272_000`.
It is red before the deletion and green after.

## Alternatives rejected

- **Wire `rust/tests` into a crate** — 30 files written against a crate layout that no longer
  exists; that is a rewrite, not wiring.
- **Fix only the 272000 assertions in place** — the file would still be unbuilt, so a "fixed"
  assertion would be just as unverified.

## Follow-up

The other 29 `rust/tests/*.rs` files and `rust/examples/*.rs` are unbuilt for the same reason;
a separate backlog card tracks deleting or porting them.

## Wire / API changes

None.
