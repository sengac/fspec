# RPC-104 AST Research — Row Render Module Surface

**Work unit:** RPC-104 (Provider settings: per-row icons, indents and color coding)
**Date:** 2026-06-01

## Goal of this research

Before writing tests for the new `row_render.rs` module, identify:

1. The **existing render entry-point** that the new module will replace
2. The **NavItem data shape** that the renderer must accept
3. The **call site** in `list.rs` where the dispatch needs to happen
4. **LoC headroom** in `list.rs` to ensure the 300-LoC ceiling holds

## AST findings

### 1. Existing render entry-point

```
pattern: pub(super) fn render_list($$$ARGS) { $$$BODY }
path:    codelet/fspec-tui/src/views/provider_settings
```

**Result:**

```
codelet/fspec-tui/src/views/provider_settings/list.rs:166:1
  pub(super) fn render_list(view: &ProviderSettingsView, area: Rect, buf: &mut Buffer)
```

Body summary (manually inspected):

- Lines 167–191: filter-row painting + body_area carve-out
- Lines 193–205: `(no providers configured)` placeholder
- Lines 207–237: row loop — iterates `visible[scroll_offset..end]` and paints
  each row with a **single `Style::REVERSED|BOLD` for selection**, no kind
  awareness, no icons.

This is the loop body that the new `row_render::render_row(...)` call must
replace.

### 2. NavItem data model (RPC-103, already shipped)

```
pattern: pub enum NavItemKind { $$$VARIANTS }
path:    codelet/fspec-tui/src/views/provider_settings
```

**Result:**

```
codelet/fspec-tui/src/views/provider_settings/nav_item.rs:29:1
  pub enum NavItemKind {
      Provider { expanded: bool },
      Profile  { profile_name: String },
      AddProfile,
      ApiKey,
      OAuthLogin  { method: OAuthMethod, label: String },
      OAuthStatus { label: String },
  }
```

`row_render` will map each `NavItemKind` variant 1:1 to a `RowKind` variant
in the new module so the visual matrix (yellow/cyan/magenta/green) stays
canonical.

### 3. Files / modules to add

| Path                                                                  | Status | LoC budget |
|-----------------------------------------------------------------------|--------|------------|
| `codelet/fspec-tui/src/views/provider_settings/row_render.rs`         | NEW    | ≤180       |
| `codelet/fspec-tui/src/views/provider_settings/icons.rs`              | NEW    | ≤60        |
| `codelet/fspec-tui/src/views/provider_settings/list.rs`               | EDIT   | hold ≤300  |
| `codelet/fspec-tui/src/views/provider_settings/mod.rs`                | EDIT   | hold ≤300  |

### 4. LoC headroom

```
$ wc -l codelet/fspec-tui/src/views/provider_settings/*.rs
   238 list.rs   (current — pre-row_render extraction)
   271 mod.rs    (RPC-103 ceiling)
   161 nav_item.rs
   161 detail.rs (post-RPC-161 — 288 LoC reported above is stale)
    53 nav_tree_ops.rs
```

`list.rs` is 238 lines today. The current single-style row loop (lines
207–237) is ~30 lines; replacing it with `row_render::render_row` calls
trims to ~10 lines, leaving ~58 lines of headroom — well inside the 300
ceiling even after icon-aware row painting is wired in.

### 5. Test target

```
codelet/fspec-tui/tests/provider_settings_row_render_rpc104.rs   (NEW)
```

Will use `ratatui::backend::TestBackend` + `Buffer::diff` for cell-level
assertions. No NAPI, no async, no real terminal. Mirrors the testing style
already used by `tests/provider_settings_view_rpc054.rs`.
