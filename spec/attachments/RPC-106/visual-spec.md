# RPC-106 — TS-Parity Footer Hint Strings (Visual Spec)

**Parent:** RPC-054
**Scope:** Replace the current Rust footer hint strings on
`codelet/fspec-tui/src/views/provider_settings/mod.rs` (lines 120-128) with
the TS-canonical context-sensitive `getFooterHints(itemType)` strings from
`src/tui/utils/providerSettingsHelpers.ts` so the footer line under the
provider settings screen matches the TS Ink reference 1:1.

---

## TypeScript reference surface

Canonical source: `src/tui/utils/providerSettingsHelpers.ts`. The footer is
rendered in `ProviderSettingsPanel.tsx:798-801`:

```tsx
<Box marginTop={1}>
  <Text dimColor>
    {getFooterHints(navItems[selectedIndex]?.type ?? 'provider')}
  </Text>
</Box>
```

The footer always renders as a single dim-styled text line, separated by
the bullet character `·` (U+00B7, MIDDLE DOT). The shared trailing fragment
`FOOTER_COMMON = '/ filter · Tab: Switch to models · Esc: close'` is
appended to every per-row-type hint.

### Per-row-type hint table (verbatim from `providerSettingsHelpers.ts:16-33`)

| Selected nav-item kind | Footer string                                                              |
|------------------------|----------------------------------------------------------------------------|
| `provider`             | `Enter: expand · / filter · Tab: Switch to models · Esc: close`            |
| `oauth-status`         | `Enter: logout · / filter · Tab: Switch to models · Esc: close`            |
| `oauth-login`          | `Enter: start login · / filter · Tab: Switch to models · Esc: close`       |
| `api-key`              | `Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close`  |
| `profile`              | `Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close`  |
| `add-profile`          | `Enter: create · / filter · Tab: Switch to models · Esc: close`            |
| `default` (fallback)   | `/ filter · Tab: Switch to models · Esc: close`                            |

The `default` branch is taken when `navItems[selectedIndex]` is undefined
(empty list, out-of-range index after filter) — in TS the lookup uses
`?? 'provider'` so the default rarely fires, but it MUST be preserved as
a safety net.

### Existing Rust hints (to be replaced)

Current `mod.rs:120-128`:

```rust
pub fn footer_hint(&self) -> &'static str {
    match &self.mode {
        ProviderSettingsMode::List => "Enter Select | ↑↓ Navigate | D Delete | Esc Cancel",
        ProviderSettingsMode::Detail { sub, .. } => match sub {
            DetailSub::Summary { .. } => "t Test | r Refresh Models | Esc Back",
            DetailSub::EditApiKey { .. } => "Enter Save | Esc Cancel",
            DetailSub::OAuthNotice => "Esc Back",
        },
    }
}
```

Problems with the current Rust impl:
1. Uses `|` separator instead of `·` (bullet) — visually different.
2. Capitalises keys (`D Delete`) instead of lowercase (`d: delete`).
3. Has no per-row-type context — every list row sees the same hint.
4. Missing `/ filter` and `Tab: Switch to models` shared suffix.
5. `Esc Cancel` should be `Esc: close`.
6. The hints are scoped to `mode` (List vs Detail) not to the selected
   nav-item kind, so the user gets generic hints instead of the targeted
   `Enter: edit · d: delete` only when an api-key row is focused.

---

## Files to add / modify

1. `codelet/fspec-tui/src/views/provider_settings/footer_hints.rs` (NEW,
   ≤60 LoC). Public helper:

   ```rust
   pub const FOOTER_COMMON: &str = "/ filter · Tab: Switch to models · Esc: close";

   pub fn footer_hint_for(row_kind: Option<RowKind>) -> String {
       use crate::views::provider_settings::row_render::RowKind;
       match row_kind {
           Some(RowKind::Provider)    => format!("Enter: expand · {FOOTER_COMMON}"),
           Some(RowKind::OauthStatus) => format!("Enter: logout · {FOOTER_COMMON}"),
           Some(RowKind::OauthLogin)  => format!("Enter: start login · {FOOTER_COMMON}"),
           Some(RowKind::ApiKey)      => format!("Enter: edit · d: delete · {FOOTER_COMMON}"),
           Some(RowKind::Profile)     => format!("Enter: edit · d: delete · {FOOTER_COMMON}"),
           Some(RowKind::AddProfile)  => format!("Enter: create · {FOOTER_COMMON}"),
           None                       => FOOTER_COMMON.to_string(),
       }
   }
   ```

2. `codelet/fspec-tui/src/views/provider_settings/mod.rs` —
   `footer_hint(&self)` becomes `footer_hint(&self) -> String` returning
   the dispatched-on-selected-row-kind string. Detail-mode hints keep their
   own dedicated strings (`Enter: save · Esc: cancel` for EditApiKey, etc.)
   but adopt the same `·` bullet separator and lowercase-colon style.

3. `codelet/fspec-tui/src/views/provider_settings/list.rs` — needs to expose
   `current_row_kind()` so `mod.rs::footer_hint` can pass it to
   `footer_hint_for`.

Net LoC: `mod.rs` shrinks by ~5 lines, `footer_hints.rs` adds ~50 lines.

---

## Render-side contract

The footer band is rendered through the existing
`render_footer_hint(footer_area, buf, self.footer_hint())` call (mod.rs
line 245). The function already applies dim styling, so the hint string
is passed verbatim. Only the string content changes — no widget changes.

The footer area is always `Length(1)` (mod.rs:227) — a single-line band
at the bottom of the view. Long hints (the api-key / profile variants are
~75 chars) will be truncated by the right-edge clip in
`render_footer_hint`; this matches TS behaviour where `<Text wrap>` is not
applied to the footer.

---

## Ratatui style

Existing helper at `crate::views::agent::mode_view_render::render_footer_hint`
sets `Style::default().add_modifier(Modifier::DIM)`. No change. The bullet
character `·` (U+00B7) is in the ASCII-compatible Latin-1 range and renders
as one cell.

---

## Integration test plan

`codelet/fspec-tui/tests/provider_settings_footer_hints.rs` (NEW):

1. `footer_hint_for_provider_row_includes_enter_expand_and_common_suffix` —
   assert string == `Enter: expand · / filter · Tab: Switch to models · Esc: close`.
2. `footer_hint_for_oauth_status_row_includes_enter_logout` — assert string
   starts with `Enter: logout`.
3. `footer_hint_for_oauth_login_row_includes_enter_start_login` — assert
   string starts with `Enter: start login`.
4. `footer_hint_for_api_key_row_includes_enter_edit_and_d_delete` — assert
   string contains both `Enter: edit` AND `d: delete`.
5. `footer_hint_for_profile_row_matches_api_key_hint` — assert profile and
   api-key produce identical strings (both have edit/delete).
6. `footer_hint_for_add_profile_row_includes_enter_create` — assert string
   starts with `Enter: create`.
7. `footer_hint_for_none_row_returns_only_common_suffix` — assert empty
   list / out-of-range index produces exactly FOOTER_COMMON.
8. `footer_hint_uses_bullet_separator_not_pipe` — assert no `|` appears in
   any of the seven hint strings.
9. `footer_hint_uses_lowercase_colon_for_keys` — assert `Esc:` not
   `Esc ` and `d:` not `D `.
10. `footer_render_writes_hint_into_bottom_row_with_dim_style` — full
    `ProviderSettingsView::render` into a TestBackend, assert the bottom
    row contains the per-selected-row hint with `Modifier::DIM` set on
    each cell.

All ten tests are pure string / widget tests — no NAPI required.

---

## Acceptance signals

- `cargo test -p codelet-fspec-tui provider_settings_footer_hints` green.
- `cargo run -- /provider` shows `Enter: expand · / filter · Tab: Switch
  to models · Esc: close` at the bottom when a provider row is selected,
  and changes to `Enter: edit · d: delete · / filter · Tab: Switch to
  models · Esc: close` when the user navigates onto an api-key child row.
- No `|` characters appear in any footer hint string.
- Bullet `·` separators visually match the TS Ink screenshot in
  `spec/attachments/RPC-054/provider-settings.md`.
