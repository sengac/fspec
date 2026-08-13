# RPC-336 — AST Research: retire obsolete NotYetPorted dispatcher scenarios

**Date:** 2026-06-18
**Tool:** AstGrep (Rust) + Grep
**Scope:** `rust/fspec-core/tests/dispatcher_test.rs`, `rust/fspec-core/tests/list_work_units.rs`, `rust/fspec-core/src/canonical.rs`, `rust/fspec-core/src/dispatch.rs`

## Confirmed: cleanup still pending

## AST findings — the two obsolete tests (still `#[ignore]`d)

| Test fn | Location | Note |
|---------|----------|------|
| `fn dispatcher_returns_not_yet_ported_for_known_unported_command()` | `dispatcher_test.rs:30` | Hard-codes `"review"` as a "known unported command"; `#[ignore]`d as OBSOLETE (Batch 20) |
| `async fn dispatch_command_returns_not_yet_ported_when_called_from_inside_a_tokio_runtime()` | `list_work_units.rs:553` | Same `"review"` assumption; `#[ignore]`d, tracked by RPC-336 |

## AST findings — why the premise is dead

- `canonical.rs:1028 fn is_ported(name: &str) -> bool` → `PORTED_COMMANDS.contains(&name)`.
- `PORTED_COMMANDS` (`canonical.rs:847`) now has **162** entries; `"review"` is one
  of them (`canonical.rs:1024`, RPC-295). So `is_ported("review") == true`.
- `dispatch.rs:147` gates on `is_ported`; `dispatch.rs:560` marks the
  NotYetPorted fallthrough `unreachable!()` for canonical commands.

⇒ No canonical command can return `NotYetPorted`, so both tests assert an
unreachable path.

## Recommended scope (ACDD)

Remove or repurpose both scenarios. If repurposing, assert instead:
1. `NotYetPorted` error formatting is correct when constructed directly.
2. The tokio-runtime dispatch path does not hang and returns a real
   arg-validation error.
3. Invariant: all 162 canonical commands are ported (`is_ported` over
   `CANONICAL_COMMANDS`).

Then remove the two `#[ignore]` attributes.
