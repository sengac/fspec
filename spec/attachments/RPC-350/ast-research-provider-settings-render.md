# RPC-350 — AST research: provider-settings render layer

AST-based structural analysis (ast-grep, rust) of the code paths the four
in-scope render regressions (R1–R4) touch. Confirms exact call sites and the
single-`Style` contract that R4 must break.

## Functions in `src/views/provider_settings/` (ast-grep `fn $N($$$A) -> $R { ... }`)

Render-relevant declarations confirmed:

- `row_render.rs:59` `fn row_style(kind: RowKind, selected: bool) -> Style`
  — returns ONE `Style` for the whole row. **R4 root cause.**
- `row_render.rs:84` `fn row_prefix(kind: RowKind, selected: bool) -> String`
  — supplies the `> `/`  ` marker + `▼/▶` glyph + child icon (incl. `icons::PLUS`
  for AddProfile). **R3: only the label string changes; the `+ ` glyph stays here.**
- `list_nav_render.rs:109` `fn row_kind_and_label(item, view) -> (RowKind, String)`
  — builds `(kind, label)`; AddProfile arm hard-codes `"Add Profile"` (**R3**),
  provider arm builds `format!("{name}{}", provider_annotation(display))` (**R4/R2**).
- `list_nav_render.rs:149` `fn provider_annotation(...) -> String`
  `:158` `fn api_key_annotation(...)` `:166` `fn source_suffix(...)`
  — these build the `" ✓ {masked}{ [src]}"` / `" (not configured)"` / `" (not set)"`
  suffixes as flat strings (no per-span styling). **R4 + R2** plug in here.

## Confirmed call sites

- `format!("{name}{}", provider_annotation(display))` — single match at
  `list_nav_render.rs:123` → the provider header label assembly point.
  R2 (openai profile badge) and R4 (per-span colouring) both originate here.
- `render_row(kind, &label, selected, row_area, buf)` — `list_nav_render.rs:55`
  is the sole production caller; `tests/provider_settings_row_render_rpc104.rs`
  is the test caller. The single-`Style` `render_row` must keep working for
  RPC-104, so R4 adds a NEW span-aware paint path rather than mutating render_row's
  contract (preserves the wide-glyph band-repair loop at row_render.rs:148-150).

## Title rendering (R1)

- `mode_view_render.rs::render_title_with_count` is SHARED (pub(crate)) and used by
  `full_screen_shell::render_full_screen_scaffold` → ResumeSessionView,
  SearchHistoryView. Must NOT change (R5 guard).
- `full_screen_shell.rs:64` exposes `render_full_screen_scaffold_with_title<T,B>` —
  a title-closure variant already present. R1 routes the provider view through
  this with a two-span (yellow bold name + dim/DarkGray count) title closure.
- `mod.rs::render` currently calls `render_full_screen_scaffold(... "Provider Settings", count, "items" ...)`.
  R1 swaps it to the `_with_title` variant with a provider-specific closure.

## Conclusion / plan

- R1: add a provider-specific two-span title painter; reroute `mod.rs::render`
  through `render_full_screen_scaffold_with_title`. No change to shared title.
- R2: append dim ` ({n} profile[s])` for openai-with-profiles in the provider
  label assembly (segment-aware, see R4).
- R3: change AddProfile label string `"Add Profile"` → `"Create new profile"`.
- R4: introduce a span-aware row painter module (`row_paint`/segments) that paints
  `Vec<(text, fg)>` over the base band, flipping all fg → Black when selected,
  preserving the full-width band + wide-glyph repair loop.
