# RPC-104 — Per-Row Icons, Indents and Color Coding (Visual Spec)

**Parent:** RPC-054
**Scope:** Port the TS `ProviderSettingsPanel` per-row visual styling (selection
marker, expand glyph, indent prefix, emoji icon, foreground/background colour
band) to the Rust `codelet/fspec-tui/src/views/provider_settings/list.rs`
renderer.

---

## TypeScript reference surface

Canonical source: `src/tui/components/ProviderSettingsPanel.tsx` (lines 569 –
770). All five nav-item kinds render through the same `navItems.slice(...)`
map, but each branch carries its **own** colour pairing and indent.

### Per-row visual contract (TS line citations)

| Nav item       | Selection prefix     | Inner indent | Icon            | Colour scheme (sel / unsel) | TS lines |
|----------------|----------------------|--------------|-----------------|------------------------------|----------|
| `provider`     | `"> "` / `"  "` (L591) | none         | `▼ ` / `▶ ` (L592) | `bg:yellow,fg:black` / `fg:white` (L587-588) | 576-636 |
| `profile`      | `"> "` / `"  "` (L653) | `'    '` (4-sp, L654) | `📁 ` (L654)       | `bg:cyan,fg:black` / `fg:cyan` (L649-650)    | 638-679 |
| `oauth-login`  | `"> "` / `"  "` (L693) | `'    '` (4-sp, L694) | `🔑 ` (L694)       | `bg:magenta,fg:black` / `fg:magenta` (L689-690) | 682-698 |
| `oauth-status` | `"> "` / `"  "` (L712) | `'    '` (4-sp, L713) | (label has its own glyph, no prefix icon) | `bg:green,fg:black` / `fg:green` (L708-709) | 701-717 |
| `api-key`      | `"> "` / `"  "` (L733) | `'    '` (4-sp, L734) | `🔑 ` (L734)       | `bg:yellow,fg:black` / `fg:yellow` (L729-730) | 720-752 |
| `add-profile`  | `"> "` / `"  "` (L765) | `'    '` (4-sp, L766) | `+ ` (L766)        | `bg:green,fg:black` / `fg:green` (L761-762)    | 754-769 |

The provider row is the only row that does NOT get the 4-space inner indent —
it is the parent of the tree. All child rows (profile, oauth-login,
oauth-status, api-key, add-profile) prepend `'    '` after the selection
prefix to visually indent under their parent provider.

### Inline status decorations on the `provider` row (L594-633)

After the provider name, additional spans are appended in this order:

1. **Configured marker** — when `status.hasKey === true`:
   - text: `" ✓ "` + `status.maskedKey`
   - colour: `fg:green` when unselected, `fg:black` when selected (L595)
   - then `" [" + status.source + "]"` in dim style (L598-603)
2. **Unconfigured marker** — when `status.hasKey === false`:
   - text: `" (not configured)"`
   - colour: `fg:gray` when unselected, `fg:black` when selected (L606)
3. **Profile-count badge** — only for `provider.id === "openai"` and
   `profileCount > 0`:
   - text: `" (N profile)" / " (N profiles)"`
   - colour: dim (L611-617)
4. **Test-result inline** — when `testResult.providerId === item.providerId`
   AND `testResult.profileName` is absent:
   - text: `" " + testResult.message`
   - colour: green when `success`, red when failure (L618-632)

### Inline status decorations on the `api-key` row (L735-748)

Same pattern: `" ✓ " + maskedKey + " [" + source + "]"` (configured) or
`" (not set)"` (unconfigured). Same fg:green / fg:gray flip on selection.

### Inline status decorations on the `profile` row (L655-674)

After the profile name: `" → " + profile.config.baseUrl` (dim).
Then test-result span identical to provider row, but matched on both
`providerId` AND `profileName` (L661-675).

---

## Ratatui colour mapping

Ink uses chalk-style colour names. Ratatui `Color` enum maps 1:1 onto the
ANSI 16-colour palette:

| Ink colour | Ratatui constant       | ANSI code |
|------------|------------------------|-----------|
| `yellow`   | `Color::Yellow`        | 33        |
| `cyan`     | `Color::Cyan`          | 36        |
| `magenta`  | `Color::Magenta`       | 35        |
| `green`    | `Color::Green`         | 32        |
| `red`      | `Color::Red`           | 31        |
| `gray`     | `Color::Gray`          | 37        |
| `black`    | `Color::Black`         | 30        |
| `white`    | `Color::White`         | 37 bright |
| `dimColor` | `Modifier::DIM` flag   | n/a       |

The selection band on the `provider` row therefore renders as
`Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD)`.
The fully unselected provider row is `Style::default().fg(Color::White)`.

Note: Ratatui has no native emoji width handling — `📁`, `🔑` are 2 cells
wide in most terminals and need to be measured through `unicode-width` for
correct truncation. We will rely on the existing `tui-text` clamp helper.

---

## Files to add / modify in the Rust port

1. `codelet/fspec-tui/src/views/provider_settings/row_render.rs` (NEW, ≤180
   LoC). Exposes:
   - `enum RowKind { Provider, Profile, OauthLogin, OauthStatus, ApiKey, AddProfile }`
   - `fn render_row(kind: RowKind, label: &str, selected: bool, status: Option<RowStatus>, area: Rect, buf: &mut Buffer)`
   - Private helper `fn row_style(kind, selected) -> Style` returning the
     fg/bg pair from the table above.
2. `codelet/fspec-tui/src/views/provider_settings/list.rs` — replace the
   current single-style row paint loop (L200-225) with calls into
   `row_render::render_row(...)` for every `NavItem` produced by RPC-103's
   flat tree iterator.
3. `codelet/fspec-tui/src/views/provider_settings/icons.rs` (NEW, ≤60 LoC).
   Constants for the icon glyphs: `EXPANDED = "▼ "`, `COLLAPSED = "▶ "`,
   `FOLDER = "📁 "`, `KEY = "🔑 "`, `PLUS = "+ "`, `INDENT = "    "`,
   `SEL = "> "`, `NOSEL = "  "`.

`list.rs` must stay ≤300 LoC — extracting row paint into `row_render.rs`
keeps the file inside the ceiling once the colour matrix is added.

---

## Integration test plan

`codelet/fspec-tui/tests/provider_settings_row_render.rs` (NEW):

1. `row_render_emits_yellow_selection_band_on_provider_row` — invoke
   `render_row(RowKind::Provider, "OpenAI", selected=true, …)` into a 1-line
   buffer; assert every cell carries `bg=Yellow,fg=Black`.
2. `row_render_emits_cyan_band_on_profile_row` — selected profile row gets
   `bg=Cyan,fg=Black`.
3. `row_render_emits_magenta_band_on_oauth_login_row` — selected oauth-login
   row gets `bg=Magenta,fg=Black`.
4. `row_render_emits_green_band_on_oauth_status_row` — selected oauth-status
   row gets `bg=Green,fg=Black`.
5. `row_render_emits_green_band_on_add_profile_row` — selected add-profile
   row gets `bg=Green,fg=Black`.
6. `row_render_child_rows_get_four_space_indent` — assert the first four
   cells AFTER the selection prefix are spaces for every non-provider kind.
7. `row_render_provider_row_has_no_inner_indent` — assert cell index 2 (just
   past the `"> "` prefix) is `▼`/`▶`, NOT a space.
8. `row_render_expand_glyph_flips_on_is_expanded` — provider row with
   `expanded=true` paints `▼`; with `expanded=false` paints `▶`.

All eight tests use ratatui's `TestBackend` + `Buffer::diff` to assert
exact cell content and style. No NAPI/no async — pure widget tests.

---

## Acceptance signals

- `cargo test -p codelet-fspec-tui provider_settings_row_render` is green.
- A manual `cargo run -- /provider` shows the four-colour palette matching
  the TS Ink screenshot in `spec/attachments/RPC-054/provider-settings.md`.
- Selection band always inverts fg→bg of the row's tint (yellow→black,
  cyan→black, etc.) per TS lines 587-590, 649-650, 689-690, 708-709,
  729-730, 761-762.
