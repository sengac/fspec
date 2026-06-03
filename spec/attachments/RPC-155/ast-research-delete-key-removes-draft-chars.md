# RPC-155 AST Research — Delete key removes draft chars shape

**Date:** 2026-06-01
**Work unit:** RPC-155 — Provider settings api-key edit: Delete key removes draft chars (in addition to Backspace)
**Pattern:** Regression-shape coverage (mirrors RPC-152 / RPC-153 / RPC-156 / RPC-151 / RPC-149 / RPC-077)

## Goal

Pin the source-shape of the TS-parity deletion-key handling inside
`codelet/fspec-tui/src/views/provider_settings/detail.rs` so that the
implementation already delivered by RPC-163 cannot silently regress
(lose the `Delete` key binding, split the two key paths into separate
arms that could drift apart, or move the deletion arm below the
`KeyCode::Char(c)` arm where it would never fire).

## TypeScript reference

`src/tui/inputHandlers/apiKeyEditModeHandler.ts:46`:

```ts
if (key.backspace || key.delete) {
  setDraft((d) => d.slice(0, -1));
  return;
}
```

Contract: Ink's `useInput` exposes `key.backspace` and `key.delete` as
sibling boolean flags wired through a single OR'd branch to
`draft.slice(0, -1)`. Rust must mirror this so a user pressing either
Backspace OR Delete erases the trailing draft character.

## Rust call site (AST search)

```
ast-grep --lang rust --pattern 'fn handle_edit_key($$$ARGS) -> ProviderSettingsEvent { $$$BODY }'
```

**Match:** `codelet/fspec-tui/src/views/provider_settings/detail.rs:102-173`

Relevant body excerpt (lines 134-146):

```rust
KeyCode::Backspace | KeyCode::Delete => {
    // RPC-163 — TS parity: Ink's useInput exposes key.backspace and
    // key.delete as sibling boolean flags both wired to
    // draft.slice(0, -1) (see src/tui/inputHandlers/apiKeyEditModeHandler.ts:46).
    // Mirror that here with a merged match arm so the two key paths
    // can never diverge.
    draft.pop();
    view.mode = ProviderSettingsMode::Detail {
        provider_id,
        sub: DetailSub::EditApiKey { draft },
    };
    ProviderSettingsEvent::Consumed
}
```

## Invariants the source-shape test must pin

1. The `handle_edit_key` body contains the merged pattern substring
   `KeyCode::Backspace | KeyCode::Delete =>` — proving Delete is wired
   in the SAME arm as Backspace (single source of truth for deletion).
2. The brace-balanced body of that merged arm contains `draft.pop()` —
   proving the deletion call is actually present inside it.
3. The `handle_edit_key` body contains zero occurrences of a standalone
   `KeyCode::Delete =>` arm — Delete may only appear in the merged
   form. A regression that splits the arms would let the two key paths
   diverge.
4. Byte-offset ORDER: the merged `KeyCode::Backspace | KeyCode::Delete`
   arm appears BEFORE the `KeyCode::Char(c)` arm. (Char(c) is a
   catch-all printable; if Backspace/Delete moved below it the
   deletion path would never fire because Backspace / Delete are not
   `Char` keycodes — but ordering still serves as a forward-compatibility
   contract documenting the intended sequence.)

## Test file

`codelet/fspec-tui/tests/rpc155_delete_key_removes_draft_chars_shape.rs`

Reads the source of `detail.rs` via a path relative to
`CARGO_MANIFEST_DIR` and asserts the substring + brace-balanced body +
byte-offset + zero-occurrence invariants. Sub-millisecond execution;
no key event simulation.

## Tags

- `@rpc-155` — work unit identifier
- `@source-shape` — pattern marker
- `@regression`, `@ts-parity`, `@provider-settings`, `@tui`, `@rust`,
  `@keyboard-navigation`

## Related cards

- **RPC-163** — delivered the implementation in detail.rs
- **RPC-153** — sister regression-shape card (filterPrintableChars guard)
- **RPC-152 / RPC-156 / RPC-151 / RPC-149** — sibling shape cards on
  list.rs / mod.rs
- **RPC-077** — original shape pattern (handle_impl.rs redundant clone)
