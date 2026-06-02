# RPC-106 AST Research: Footer Hints (TS Parity)

Research date: 2026-06-01.

## TypeScript reference (source of truth)

AstGrep query:
```
pattern: function getFooterHints($_): $_ { $$$BODY }
language: typescript
path: src/tui/utils/
```

Match:
- `src/tui/utils/providerSettingsHelpers.ts:16` — `function getFooterHints(itemType: string): string {`

Full implementation (lines 11–33):

```ts
const FOOTER_COMMON = '/ filter · Tab: Switch to models · Esc: close';

export function getFooterHints(itemType: string): string {
  switch (itemType) {
    case 'provider':       return `Enter: expand · ${FOOTER_COMMON}`;
    case 'oauth-status':   return `Enter: logout · ${FOOTER_COMMON}`;
    case 'oauth-login':    return `Enter: start login · ${FOOTER_COMMON}`;
    case 'api-key':        return `Enter: edit · d: delete · ${FOOTER_COMMON}`;
    case 'profile':        return `Enter: edit · d: delete · ${FOOTER_COMMON}`;
    case 'add-profile':    return `Enter: create · ${FOOTER_COMMON}`;
    default:               return FOOTER_COMMON;
  }
}
```

Key observations:
1. Separator is U+00B7 MIDDLE DOT, NOT pipe.
2. Keybind labels use lowercase + colon (`Enter:`, `d:`, `Esc:`).
3. `default` branch (no selection) returns `FOOTER_COMMON` only — there is NO per-mode hint logic; everything is per-row-kind.

## Rust impl (current, to be replaced)

AstGrep query:
```
pattern: pub fn footer_hint($$$ARGS) -> $RET { $$$BODY }
language: rust
path: codelet/fspec-tui/src/views/provider_settings/
```

Match:
- `codelet/fspec-tui/src/views/provider_settings/mod.rs:134` — `pub fn footer_hint(&self) -> &'static str`

Body:

```rust
pub fn footer_hint(&self) -> &'static str {
    match &self.mode {
        ProviderSettingsMode::List => "Enter Select | ↑↓ Navigate | D Delete | Esc Cancel",
        ProviderSettingsMode::Detail { sub, .. } => match sub {
            DetailSub::Summary { .. }     => "t Test | r Refresh Models | Esc Back",
            DetailSub::EditApiKey { .. }  => "Enter Save | Esc Cancel",
            DetailSub::OAuthNotice        => "Esc Back",
        },
    }
}
```

Gaps vs TS:
- Uses pipe `|` separators instead of `·` U+00B7.
- Uses uppercase keybind labels (`Enter Select`, `D Delete`, `Esc Cancel`) instead of lowercase-colon style.
- Returns ONE static string per top-level mode — does NOT branch per row kind. The whole List mode collapses into a single combined hint regardless of which row is focused.
- Includes `t Test` / `r Refresh Models` in Detail::Summary which are NOT in the TS reference — these are leftover from the Rust-only Summary screen scheduled for removal in RPC-162.
- Return type `&'static str` cannot host an owned formatted string — must become `String` (or `Cow<'static, str>`) to allow concatenated context-sensitive hints.

## RowKind enum (RPC-104 dependency)

AstGrep query:
```
pattern: pub enum RowKind { $$$VARS }
language: rust
```

Match:
- `codelet/fspec-tui/src/views/provider_settings/row_render.rs:35` — `pub enum RowKind`

Variants (RPC-104): `Provider { expanded: bool }`, `Profile`, `OauthLogin`, `OauthStatus`, `ApiKey`, `AddProfile` — one-to-one with TS `itemType` strings.

## Existing footer-hint test assertion to delete

`codelet/fspec-tui/tests/provider_settings_view_rpc054.rs` and `provider_settings_list_keybind_parity_rpc157.rs` assert the OLD hint strings. They must be updated in this card so the test suite stays green.

```bash
grep -rn 'Enter Select \| ↑↓' codelet/fspec-tui/tests/ \
  codelet/fspec-tui/src/views/provider_settings/
```

## Implementation plan

1. New file `codelet/fspec-tui/src/views/provider_settings/footer_hints.rs`:
   - `pub const FOOTER_COMMON: &str = "/ filter · Tab: Switch to models · Esc: close";`
   - `pub fn footer_hint_for(kind: Option<RowKind>) -> String`
2. `mod.rs::footer_hint()` becomes a wrapper that:
   - Looks up the currently-focused `NavItem` via `nav_items[selected_index]` (RPC-103).
   - Translates `NavItemKind` → `RowKind` (reusing `list_nav_render::row_kind_and_label` logic OR inlining the small match).
   - Passes that through `footer_hint_for(Some(kind))`; if `nav_items` is empty, passes `None`.
   - For Detail mode, falls back to mode-specific strings (will be cleaned up further in RPC-162).
   - Return type changes from `&'static str` to `String`.
3. Update the `render_footer_hint` call site at `mod.rs:265` to use the new owned `String`.
4. Update RPC-054 / RPC-157 tests that assert the old pipe-separated hint.

## Test plan

`codelet/fspec-tui/tests/provider_settings_footer_hints_rpc106.rs` — pure string + widget tests.

Cases (matches 12 scenarios in `rpc106-provider-settings-footer-hints.feature`):
1. Provider kind → "Enter: expand · …"
2. ApiKey kind → "Enter: edit · d: delete · …"
3. Profile kind → "Enter: edit · d: delete · …"
4. AddProfile kind → "Enter: create · …"
5. OauthLogin kind → "Enter: start login · …"
6. OauthStatus kind → "Enter: logout · …"
7. None → FOOTER_COMMON only
8. Output contains U+00B7, never `|`.
9. Output contains "Enter:" and "d: delete", never "D Delete"/"Esc Cancel".
10. Real-time update: build view with two nav items of different kinds, move selection, footer changes.
11. End-to-end render: full `ProviderSettingsView::render` paints hint into bottom buffer row.
12. `footer_hint_for(None)` returns the bare `FOOTER_COMMON` constant.
