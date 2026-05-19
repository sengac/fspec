# RPC-027 — Rust Dialog Theme Refactor

## Purpose

The Rust ratatui dialogs in `codelet/fspec-tui` look NOTHING like the
canonical TypeScript Ink dialogs. This document is the **complete style
spec + per-dialog migration plan** for bringing them into pixel-level
parity with the TS reference.

The TypeScript Ink dialogs (`src/tui/components/ThinkingLevelDialog.tsx`,
`AttachmentDialog.tsx`, `ThreeButtonDialog`, `TurnContentModal.tsx`,
`FileSearchPopup.tsx`, `SlashCommandPalette.tsx`, base
`src/components/Dialog.tsx`) are **frozen — they are the spec**. The
Rust side conforms to them.

---

## 1. The canonical TypeScript look

All TS Ink dialogs share the same visual contract. The reference
implementation is `src/tui/components/ThinkingLevelDialog.tsx`:

```
╭───────────────────────────────────────────────╮
│                                               │
│  Thinking Level                               │  ← bold accent (yellow)
│                                               │
│  ▸ Off - No extended thinking                 │  ← selected: bg=yellow fg=black
│    Low - ~4K tokens, quick analysis           │  ← unselected: white + dim desc
│    Medium - ~10K tokens, balanced             │
│    High - ~32K tokens, deep reasoning         │
│                                               │
│   ↑↓ Navigate │ Enter Select │ D Set Default  │  ← dim centered footer
│                │ Esc Close                    │
│                                               │
╰───────────────────────────────────────────────╯
```

Anatomy:

| Element | Style |
|---|---|
| Outer border | `borderStyle="round"` → `╭─╮ │ ╰─╯` |
| Border color | One accent (`yellow`, `cyan`, `red`) |
| Background | `backgroundColor="black"` (opaque) |
| Padding | `padding={1}` on all sides |
| Inner title | `<Text bold color={accent}>` — **NOT** a border title |
| Title→body gap | `<Box marginBottom={1}>` (1 blank row) |
| Row marker (sel) | `▸ ` (U+25B8 + space) |
| Row marker (unsel) | `  ` (two spaces) |
| Sel highlight | `backgroundColor=accent` + `color="black"` |
| Unsel text | `color="white"` |
| Description text | `dimColor={!isSelected}` |
| Body→footer gap | `<Box marginTop={1}>` (1 blank row) |
| Footer | `<Text dimColor>` centered |
| Footer separator | `│` (U+2502) — NOT ASCII pipe |

---

## 2. Current Rust state — what is wrong

Every existing Rust dialog renders via `tui_popup::Popup::new(...).title(...)`:

- The title is painted **into the top border**, not as an inner bold row.
- Body text is **raw uncolored `Text::raw(body)`** — no inverse highlight, no dim.
- Background is **transparent** — the underlying view bleeds through.
- Selection marker varies: `slash_command_popup.rs` and `file_search_popup.rs`
  use a **single** `▸` (no trailing space), breaking column alignment.
- Footers vary: some use `│`, some use ASCII pipe, some omit them entirely.
- `popup_body.rs` highlights selection with `bg=Blue fg=White` (wrong colors).
- `ConfirmDialog` uses `Block::default().borders(Borders::ALL)` (square border,
  no rounded corners, no theme integration).
- `ThinkingLevelDialog` is **missing the `D Set Default` keybinding** the TS
  reference has.

Result: side-by-side the two TUIs look like two unrelated apps.

---

## 3. Target architecture

### 3.1 New module: `codelet/fspec-tui/src/components/dialog_theme.rs`

```rust
//! Canonical fspec dialog theme — single source of truth for the
//! rounded/black/accent look shared with the TypeScript Ink dialogs.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget};

/// One canonical accent color per dialog kind.
#[derive(Debug, Clone, Copy)]
pub enum Accent { Cyan, Yellow, Red }

impl Accent {
    pub fn color(self) -> Color {
        match self {
            Accent::Cyan => Color::Cyan,
            Accent::Yellow => Color::Yellow,
            Accent::Red => Color::Red,
        }
    }
}

/// A single content row inside a dialog body. `selected = true` paints
/// the full row with the inverse highlight (bg=accent, fg=Black, BOLD).
#[derive(Debug, Clone)]
pub struct DialogRow {
    pub spans: Vec<Span<'static>>,
    pub selectable: bool,
    pub selected: bool,
}

/// Per-dialog input to `render_dialog`. `title` and `footer` are the
/// canonical inner-title + dim-centered-footer slots; `rows` is the
/// scrollable body.
pub struct FspecDialog<'a> {
    pub accent: Accent,
    pub title: &'a str,
    pub rows: Vec<DialogRow>,
    pub footer: &'a str,
    pub min_width: u16,
}

pub const MARKER_SELECTED: &str = "▸ ";
pub const MARKER_UNSELECTED: &str = "  ";
pub const FOOTER_SEPARATOR: &str = " │ ";
```

### 3.2 Public API

| Symbol | Purpose |
|---|---|
| `Accent` enum | `Cyan` / `Yellow` / `Red` — only three accents |
| `DialogRow` | One content row with optional inverse highlight |
| `FspecDialog<'a>` | Theme + title + body rows + footer |
| `render_dialog(area, buf, &d)` | Paints everything |
| `dialog_rect(area, &d)` | Centering math (testable) |
| `label_description_row(label, desc, sel)` | Convenience builder |
| `MARKER_SELECTED` / `MARKER_UNSELECTED` | `"▸ "` / `"  "` |
| `FOOTER_SEPARATOR` | `" │ "` |

### 3.3 Render contract

1. Compute `rect = dialog_rect(area, &dialog)`.
2. `Clear.render(rect, buf)` — wipes anything underneath.
3. Paint a `Block` with `BorderType::Rounded`, `border_style = fg(accent)`,
   `style = bg(Color::Black)`.
4. Inner area shrinks by 1 cell of padding on all sides.
5. Row 0: bold accent title.
6. Row 1: blank gap.
7. Body rows. Selected rows are painted with full-width inverse highlight.
8. Last `footer_h + 1` rows: blank gap + footer. Footer text uses
   `Modifier::DIM` and `Alignment::Center`.

### 3.4 Sizing

- `width = max(longest_row, title_len, footer_max_line, min_width) + 4`
  (2 border + 2 padding). Clamped to `area.width`.
- `height = 2 (border) + 2 (padding) + 1 (title) + 1 (gap) + rows.len() + 1 (gap) + footer_lines`.
- Centered: `x = area.x + (area.width - width) / 2`, similarly for `y`.

---

## 4. Per-dialog migration plan

### 4.1 HelpDialog

File: `codelet/fspec-tui/src/components/help_dialog.rs`
**Accent:** Cyan. **Title:** `"Help"`.

Before:
```rust
let popup = Popup::new(sized).title("Help");
popup.render(area, buf);
```

After:
```rust
let rows: Vec<DialogRow> = HELP_BODY.lines().map(|l| DialogRow {
    spans: vec![Span::raw(l.to_string())],
    selectable: false,
    selected: false,
}).collect();
let dialog = FspecDialog {
    accent: Accent::Cyan,
    title: "Help",
    rows,
    footer: "ESC to close",
    min_width: 40,
};
render_dialog(area, buf, &dialog);
```

Remove the `tui_popup` import. Update `help_dialog__centered_popup_80x24` insta snapshot.

### 4.2 DisconnectDialog

File: `codelet/fspec-tui/src/components/disconnect_dialog.rs`
**Accent:** Red. **Title:** `"Disconnected"`. Body lines:
- `"daemon disconnected"` (or with `— auto-reconnecting (attempt N)…`)
- blank
- `"q to quit"`
- `"r to reconnect"`

Footer: empty (CR-1 baseline shows no footer hint). Inline title body
line still re-renders on `Action::Reconnecting(n)` via `update()` — no
architectural change, just swap the render call.

### 4.3 ThinkingLevelDialog

File: `codelet/fspec-tui/src/components/thinking_level_dialog.rs`
**Accent:** Yellow. **Title:** `"Thinking Level"`.
Rows: one per `LEVELS[i]` via `label_description_row(label, description, i == selected_index)`.
Footer: `"↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"`.

Add the **missing D key** handler in `handle_event`:

```rust
KeyCode::Char('d') | KeyCode::Char('D') => {
    let level = self.selected_level();
    self.emit_action(Action::SetThinkingLevelDefault(
        self.session_id.clone(), level
    ));
    return EventResult::consumed(); // dialog stays open
}
```

Wiring:
- New `Action::SetThinkingLevelDefault(SessionId, ThinkingLevel)` variant.
- New `SessionManagerHandle::set_thinking_level_default` with no-op default impl.
- New `FspecBackend::set_thinking_level_default`, both transports.
- New arm in `dispatch_rpc022.rs` spawning the backend call.

### 4.4 ModelSelectorDialog

File: `codelet/fspec-tui/src/components/model_selector_dialog.rs`
**Accent:** Cyan. **Title:** `"Select Model"`.
Only selectable rows can be `selected = true`; provider header rows
render plain.

Capability badge `[R] [V] [Nk]` is appended as a dim span after the
model name span (matches TS dim badges).

Footer: `"↑↓ Navigate │ Enter Select │ Esc Close"`. The out-of-scope
literal `"Custom models: not yet supported"` moves into the footer slot
as a second footer line (still DIM).

`model_selector_dialog_rows.rs` — drop the old `DialogBody` adapter,
return `Vec<DialogRow>` directly from `build_rows`.

### 4.5 ConfirmDialog

File: `codelet/fspec-tui/src/views/agent/confirm_dialog.rs`
**Accent:** Yellow. **Title:** caller-supplied.

Body becomes one non-selectable row holding the message body. Append a
blank spacer row, then a button row. Button row spans:

```rust
let mut spans: Vec<Span<'static>> = Vec::new();
for (i, label) in self.buttons.iter().enumerate() {
    if i > 0 { spans.push(Span::raw(FOOTER_SEPARATOR.to_string())); }
    let style = if i == self.focused {
        Style::default()
            .bg(accent)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else { Style::default() };
    spans.push(Span::styled(format!(" {label} "), style));
}
```

Replace the raw `Block::default().borders(Borders::ALL)` + `Clear` +
`Paragraph` plumbing with a single `render_dialog` call.

### 4.6 SlashCommandPopup

File: `codelet/fspec-tui/src/views/agent/slash_command_popup.rs`
**Accent:** Cyan. **Title:** `"Slash Commands"`.
Each match row: `label_description_row("/<name padded>", description, i == selected_index)`.
**Fix the single-char marker to two-char `"▸ "` / `"  "`**.
Footer: `"↑↓ Navigate │ Tab/Enter Select │ Esc Close"`.

### 4.7 FileSearchPopup

File: `codelet/fspec-tui/src/views/agent/file_search_popup.rs`
**Accent:** Cyan. **Title:** `"File Search"`.
Each match row: marker + path string.
Empty state: single non-selectable row containing
`"(type to search files)"` or `"(no files match \"<filter>\")"`.
Footer: `"↑↓ Navigate │ Tab/Enter Select │ Esc Close"`.

### 4.8 Removed: popup_body.rs

Delete `codelet/fspec-tui/src/views/agent/popup_body.rs` after both
popups migrate. The wrong selection colors (`bg=Blue fg=White`)
disappear with it. Remove its `mod popup_body` declaration.

---

## 5. Cross-language style table

| Dialog | Accent | Title | Color |
|---|---|---|---|
| HelpDialog | Cyan | `Help` | `Color::Cyan` |
| DisconnectDialog | Red | `Disconnected` | `Color::Red` |
| ThinkingLevelDialog | Yellow | `Thinking Level` | `Color::Yellow` |
| ModelSelectorDialog | Cyan | `Select Model` | `Color::Cyan` |
| ConfirmDialog | Yellow | (caller-supplied) | `Color::Yellow` |
| SlashCommandPopup | Cyan | `Slash Commands` | `Color::Cyan` |
| FileSearchPopup | Cyan | `File Search` | `Color::Cyan` |

Constants applying to all dialogs:

| Property | Value |
|---|---|
| Border type | `BorderType::Rounded` |
| Background | `Color::Black` |
| Padding | 1 cell on all sides |
| Title style | `Style::default().fg(accent).add_modifier(Modifier::BOLD)` |
| Inverse highlight | `bg(accent).fg(Color::Black).add_modifier(Modifier::BOLD)` |
| Description style (unsel) | `Style::default().add_modifier(Modifier::DIM)` |
| Footer style | `Style::default().add_modifier(Modifier::DIM)` |
| Footer alignment | `Alignment::Center` |
| Footer separator | `" │ "` (space + U+2502 + space) |
| Selected marker | `"▸ "` (U+25B8 + space) |
| Unselected marker | `"  "` (two spaces) |
| Selected text fg | `Color::Black` |
| Unselected text fg | `Color::White` |

---

## 6. Testing strategy

### 6.1 Unit tests for `dialog_theme.rs`

- `accent_color_matches_variant` — each `Accent` returns the right `Color`.
- `dialog_rect_centers_within_area` — verify x/y/width/height for a known body.
- `dialog_rect_clamps_to_area_when_too_small` — area smaller than ideal.
- `render_paints_border_with_accent_color` — render to 80x24 `TestBackend`,
  assert top-left cell is `╭` with `Style::default().fg(accent)`.
- `render_paints_background_black` — pick any inner cell, assert
  `bg == Some(Color::Black)`.
- `render_title_is_bold_and_accent` — assert title row cell modifiers.
- `render_selected_row_uses_inverse_highlight` — pick the row index of
  a selected `DialogRow`, assert `bg == accent`, `fg == Black`, `BOLD`.
- `render_footer_is_dim_and_centered` — assert dim modifier + position.

### 6.2 Insta snapshot regeneration

Every existing snapshot must be reviewed and regenerated:

- `codelet/fspec-tui/src/components/snapshots/codelet_fspec_tui__components__help_dialog__tests__help_dialog__centered_popup_80x24.snap`
- Any disconnect_dialog/thinking_level_dialog/model_selector_dialog snapshots.

Add new snapshots:
- `thinking_level_dialog__centered_popup_80x24.snap`
- `model_selector_dialog__centered_popup_80x24.snap`
- `slash_command_popup__centered_popup_80x24.snap`
- `file_search_popup__centered_popup_80x24.snap`
- `confirm_dialog__centered_popup_80x24.snap`

### 6.3 Cross-language parity test (optional, future card)

A separate Node + Rust test harness can render each dialog at the same
dimensions and diff the cell grids. Out of scope for RPC-027 but kept
as a possible follow-up.

---

## 7. Migration order (smallest blast radius first)

1. **Land `dialog_theme.rs`** with full unit tests. No callers yet.
2. **Migrate `HelpDialog`** (simplest — no selection, fixed body).
3. **Migrate `DisconnectDialog`** (dynamic title body, still no selection).
4. **Migrate `ThinkingLevelDialog`** (adds selection + `D` key + new Action variant).
5. **Migrate `ModelSelectorDialog`** (selection + non-selectable provider header rows).
6. **Migrate `ConfirmDialog`** (button row variant).
7. **Migrate `SlashCommandPopup`** + **`FileSearchPopup`**, delete `popup_body.rs`.
8. **Regenerate all insta snapshots** and have a maintainer review them visually.

Each step is a single commit. Each commit keeps the build green and all
tests passing (other than the snapshot diff, which is reviewed in step 8).

---

## 8. Acceptance checklist

- [ ] `codelet/fspec-tui/src/components/dialog_theme.rs` exists and exports
      `Accent`, `DialogRow`, `FspecDialog`, `render_dialog`, `dialog_rect`,
      `label_description_row`, `MARKER_SELECTED`, `MARKER_UNSELECTED`, `FOOTER_SEPARATOR`.
- [ ] No dialog module imports `tui_popup` directly anymore (search shows
      only `dialog_theme.rs` uses ratatui primitives).
- [ ] `popup_body.rs` is deleted; no `mod popup_body` declarations remain.
- [ ] All seven dialogs render with the canonical theme.
- [ ] `ThinkingLevelDialog` honors the `D` / `d` key with
      `Action::SetThinkingLevelDefault` plumbed through to the backend.
- [ ] Every dialog file stays under 300 LoC; `dialog_theme.rs` itself
      stays under 300 LoC (split into `dialog_theme_rows.rs` if needed).
- [ ] All insta snapshots regenerated and reviewed.
- [ ] No TypeScript files in `src/tui/components/` or `src/components/`
      were modified.
- [ ] `cargo test -p codelet-fspec-tui` passes.
- [ ] `cargo clippy -p codelet-fspec-tui --all-targets -- -D warnings` passes.

---

## 9. Reference: TS source line citations

| Pattern | TS file | Lines |
|---|---|---|
| Base Dialog overlay | `src/components/Dialog.tsx` | 56–75 |
| Rounded border | `src/components/Dialog.tsx` | 67 |
| Black background | `src/components/Dialog.tsx` | 70 |
| ESC handler | `src/components/Dialog.tsx` | 43–54 |
| Inner bold accent title | `src/tui/components/ThinkingLevelDialog.tsx` | 120–124 |
| Title→body gap | `src/tui/components/ThinkingLevelDialog.tsx` | 120 (marginBottom=1) |
| Selected/unselected marker | `src/tui/components/ThinkingLevelDialog.tsx` | 137 |
| Inverse selection highlight | `src/tui/components/ThinkingLevelDialog.tsx` | 133–139 |
| Description dimColor | `src/tui/components/ThinkingLevelDialog.tsx` | 140–144 |
| Footer (dim + center) | `src/tui/components/ThinkingLevelDialog.tsx` | 150–152 |
| Footer separator U+2502 | `src/tui/components/ThinkingLevelDialog.tsx` | 151 |
| D Set Default key | `src/tui/components/ThinkingLevelDialog.tsx` | 93–96 |
