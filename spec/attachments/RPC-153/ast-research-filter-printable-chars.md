# RPC-153 AST Research — filterPrintableChars ASCII 32-126 restriction shape

**Date:** 2026-06-02
**Work unit:** RPC-153 — Provider settings api-key edit: filterPrintableChars ASCII 32-126 restriction
**Pattern:** Regression-shape coverage (mirrors RPC-152 / RPC-156 / RPC-151 / RPC-149 / RPC-077)

## Goal

Pin the source-shape of the TS-parity `filterPrintableChars` guard inside
`codelet/fspec-tui/src/views/provider_settings/detail.rs` so that the
implementation already delivered by RPC-161 cannot silently regress (lose
the helper, lose the `(32..=126)` boundary, or lose the guard around
`draft.push(c)`).

## TypeScript reference

`src/tui/utils/providerSettingsHelpers.ts:39-47`:

```ts
export const filterPrintableChars = (input: string): string => {
  let result = '';
  for (const ch of input) {
    const code = ch.charCodeAt(0);
    if (code >= 32 && code <= 126) result += ch;
  }
  return result;
};
```

Boundary contract: inclusive ASCII range **32..=126**. Control chars
(0..=31), DEL (127), Latin-1 supplement (128..=255), and any
non-BMP/emoji must be rejected.

## Rust call sites (AST search)

### 1. Helper definition

```
ast-grep --lang rust --pattern 'fn is_printable_ascii(c: char) -> bool { $$$BODY }'
```

**Match:** `codelet/fspec-tui/src/views/provider_settings/detail.rs:26-29`

```rust
fn is_printable_ascii(c: char) -> bool {
    let code = c as u32;
    (32..=126).contains(&code)
}
```

Signature: `fn is_printable_ascii(c: char) -> bool` — exact substring.
Body MUST contain `(32..=126).contains(&code)` — the canonical
boundary expression.

### 2. Guard site

```
ast-grep --lang rust --pattern 'fn handle_edit_key($$$ARGS) -> ProviderSettingsEvent { $$$BODY }'
```

**Match:** `codelet/fspec-tui/src/views/provider_settings/detail.rs:102-173`

Relevant body excerpt (lines 147-164):

```rust
KeyCode::Char(c) => {
    // RPC-161 — drop control chars / DEL / non-ASCII so only
    // printable ASCII (32..=126) lands in the draft buffer.
    if is_printable_ascii(c) {
        draft.push(c);
        // Clear validation message on any further typing — but
        // only when an ACCEPTED printable char was appended;
        // dropping a non-printable must NOT clear the message.
        if view.status == "API key cannot be empty" {
            view.status.clear();
        }
    }
    view.mode = ProviderSettingsMode::Detail {
        provider_id,
        sub: DetailSub::EditApiKey { draft },
    };
    ProviderSettingsEvent::Consumed
}
```

Invariants the source-shape test must pin:

1. The function body contains `is_printable_ascii(c)` — the guard call.
2. The function body contains `draft.push(c)` — the append.
3. The **byte offset** of `is_printable_ascii(c)` is **less than** the
   byte offset of `draft.push(c)` — proves the guard precedes the
   append (i.e. the push is inside the `if` arm, not above it).

The existing inline `#[cfg(test)] mod tests { ... }` in the same file
(lines 260-286) already covers the behavioural classification of the
helper via runtime tests. RPC-153 complements that with a **fast
source-string scan** so a structural regression is caught without
running ratatui integration tests.

## Test file

`codelet/fspec-tui/tests/rpc153_filter_printable_chars_shape.rs`

Reads the source of `detail.rs` via a path relative to
`CARGO_MANIFEST_DIR` and asserts the substring + brace-balanced body +
byte-offset invariants. Sub-millisecond execution; no key event
simulation.

## Tags

- `@rpc-153` — work unit identifier
- `@source-shape` — pattern marker
- `@regression`, `@ts-parity`, `@provider-settings`, `@tui`, `@rust`,
  `@validation`, `@keyboard-navigation`

## Related cards

- **RPC-161** — delivered the implementation in detail.rs
- **RPC-152** — sister regression-shape card (Tab → SwitchToModels)
- **RPC-156 / RPC-151 / RPC-149** — sibling shape cards on list.rs / mod.rs
- **RPC-077** — original shape pattern (handle_impl.rs redundant clone)
