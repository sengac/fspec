# RPC-351 — `/model` view selection/arrow parity vs TS reference & `/provider`

**Author:** Supervisor (self-investigated, no worker)
**Scope:** Visual rendering of the full-screen ModelSelector mode-view ONLY.
**Goal:** Make the `/model` selection highlight + arrow glyph match (a) its own
TypeScript reference and (b) the already-parity-correct Rust `/provider` view.

---

## 1. Authoritative source references

### TS reference — `src/tui/components/ModelSelectorView.tsx`
- **Section header rows** (lines ~162–166):
  - `backgroundColor={isSelected ? 'cyan' : undefined}`
  - `color={isSelected ? 'black' : 'white'}`
  - Arrow marker: `{isSelected ? '> ' : '  '}` rendered **before** the ▼/▶ expand icon.
- **Model rows** (lines ~199–203):
  - `backgroundColor={isSelected ? 'cyan' : undefined}`
  - `color={isSelected ? 'black' : 'white'}`
  - Arrow marker: `{isSelected ? '  > ' : '    '}` (note the deeper indent for model rows).
- On a selected row **every** inline colored token (📁, `[C]`, `[R]`, `[V]`, `[ctx]`,
  `(current)`, `(unreachable)`) flips to **black** so it reads against the cyan band.
- The band is a `<Box>` background → it spans the full row width edge-to-edge.

**Summary of TS truth:** solid **cyan** background band + **black** foreground,
**full-row-width** fill, `> ` arrow, and all inline tokens flip to black when selected.
This is the **same selection model the `/provider` view uses** (solid colored bg band +
black text + `> ` arrow), except `/provider` uses per-kind tint colors while `/model`
uses a **uniform cyan** band for all rows.

### Parity-correct Rust reference — `/provider`
- `codelet/fspec-tui/src/views/provider_settings/icons.rs:31` → `pub const SEL: &str = "> ";`
  and `:35` → `pub const NOSEL: &str = "  ";`
- `codelet/fspec-tui/src/views/provider_settings/row_render.rs:132–136` pre-fills the
  **entire row width** with the row style so the colour band covers full width even on
  short rows:
  ```rust
  for x in area.x..area.x + area.width {
      let cell = &mut buf[(x, area.y)];
      cell.set_symbol(" ");
      cell.set_style(style);
  }
  ```
- Selected rows use a solid `bg`/`fg=Black` band (NOT terminal reverse-video), and all
  inline span colours flip to BLACK over the band (see RPC-350 `row_segments.rs`).

---

## 2. The defects in the Rust `/model` port

| # | Defect | Location | Current (WRONG) | Required (parity) |
|---|--------|----------|-----------------|-------------------|
| **D1** | Wrong selection mechanism | `rows_render.rs:146–148`, `header.rs:28–29` | `Style::default().add_modifier(Modifier::REVERSED \| Modifier::BOLD)` | Solid band: `Style::default().bg(Color::Cyan).fg(Color::Black)` |
| **D2** | Band doesn't fill row width | `rows_render.rs` (`Paragraph::new(Line::from(spans))`), `header.rs` (same) | Only the text spans get the style; highlight stops at end of text | Pre-fill `area.x..area.x+area.width` with the selected band style (mirror `provider_settings/row_render.rs:132–136`) |
| **D3** | Wrong arrow glyph (model rows) | `rows_render.rs:150` | `let marker = if is_selected { "▸ " } else { "  " };` | `"> "` when selected, `"  "` when not (TS `ModelSelectorView.tsx:203` → `'  > '` / `'    '`) |
| **D4** | Missing header selection arrow | `header.rs:35` (cloud) + `header.rs:46` (profile) | Renders `format!(" {}", row.label)` / `format!(" {arrow} ")` where `arrow` is the ▼/▶ expand icon — never prepends the `>`/space selection marker | Prepend the TS marker `{isSelected ? '> ' : '  '}` **before** the ▼/▶ icon (TS `:166`) |
| **D5** | Inline tokens don't flip to black | `rows_render.rs` badge loop (`style = base` when selected) + `(current)` marker + `header.rs` 📁 / `(unreachable)` markers | `base` is the REVERSED style — inverts each token's own fg inconsistently | When selected, every inline token must use `fg=Black` on the cyan band (badges, `(current)`, 📁, `(unreachable)`) |

### Why REVERSED is visibly wrong
Terminal reverse-video inverts whatever foreground colour each token already had, so a
green `(current)` becomes green-on-default-bg-inverted, a magenta 📁 inverts differently,
etc. The result is a multi-coloured, ragged highlight that (a) stops at the end of the
text and (b) never matches the clean solid cyan band the TS source and `/provider` show.

---

## 3. Required behaviour (acceptance-criteria seeds)

1. **Selected model/header row paints a solid `Color::Cyan` background band with
   `Color::Black` foreground**, NOT `Modifier::REVERSED`.
2. **The band fills the full row width** (every cell from `area.x` to
   `area.x + area.width`), matching `provider_settings/row_render.rs`.
3. **Selected model rows render the `> ` arrow** (with the deeper model-row indent per TS:
   `  > ` selected / `    ` unselected); unselected rows render padding of equal width to
   keep columns aligned.
4. **Selected header rows prepend the `> ` selection marker before the ▼/▶ expand icon**;
   unselected header rows prepend `  ` (two spaces) of equal width.
5. **Every inline coloured token flips to `Color::Black` when its row is selected**:
   badges (`[C]/[R]/[V]/[ctx]`), `(current)`, 📁 folder icon, `(unreachable)` marker.
6. **Band colour is uniform cyan for both header and model rows** (do NOT adopt the
   per-kind tint scheme `/provider` uses — TS `ModelSelectorView.tsx` uses one cyan band
   for all rows).
7. **Unselected rows are unchanged** (white/accent colours, DIM badges, no band).

---

## 4. Constraints / notes for implementer

- **Files in scope:** `codelet/fspec-tui/src/views/model_selector/rows_render.rs`,
  `codelet/fspec-tui/src/views/model_selector/header.rs`. A small shared helper may be
  extracted if either file approaches the **300-LoC ceiling** (rows_render.rs is ~210
  lines today; header.rs ~67).
- **DO NOT** touch the `/provider` view — it is already correct (RPC-350) and is the
  reference, not a target.
- **DO NOT** change scroll/legend/loading/empty behaviour — only selection styling +
  arrows + per-token black-flip.
- Reuse the constant pattern from `provider_settings/icons.rs` (`SEL = "> "`,
  `NOSEL = "  "`) — consider a model-local constant rather than cross-importing provider
  internals, to keep module boundaries clean.
- Existing snapshot/rows tests in `rows_tests.rs`, `rows_tests_profile.rs`,
  `tests_current_model.rs` will need their expectations updated to the new band/arrow —
  update them as part of the red→green ACDD cycle (they currently assert REVERSED/▸).
- Verify with `cargo test` in `codelet/` and confirm no `/provider` regressions.

---

## 5. Verified line references (captured at investigation time)

```
rows_render.rs:146  let base = if is_selected {
rows_render.rs:147      Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
rows_render.rs:150  let marker = if is_selected { "▸ " } else { "  " };
header.rs:28        let style = if is_selected {
header.rs:29            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
header.rs:35        Paragraph::new(Line::from(Span::styled(format!(" {}", row.label), style)))
header.rs:46        let mut spans = vec![Span::styled(format!(" {arrow} "), style)];  // arrow = ▼/▶, NOT selection marker
provider icons.rs:31   pub const SEL: &str = "> ";
provider row_render.rs:132-136  full-width band pre-fill loop
```
