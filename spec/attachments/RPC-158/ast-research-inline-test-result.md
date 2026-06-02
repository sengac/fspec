# RPC-158 — AST Research: Inline test-result rendering on focused provider row

**Date:** 2026-06-01
**Tools:** `AstGrep` (rust)

## 1. Target render seam

```
pattern: 'pub fn render_row($$$ARGS) { $$$BODY }'
language: rust
path: codelet/fspec-tui/src/views/provider_settings
```

Single match:

- `codelet/fspec-tui/src/views/provider_settings/row_render.rs:117` —
  `pub fn render_row(kind: RowKind, label: &str, selected: bool, area: Rect, buf: &mut Buffer)`
  Single per-row renderer. Returns `()` today. Must be extended to return
  `u16` — the column AFTER the last painted label cell — so the caller in
  `list_nav_render.rs` can append the inline test-result decoration without
  re-computing prefix/label widths.

## 2. Call sites of render_row

```
pattern: 'render_row($$$ARGS)'
language: rust
path: codelet/fspec-tui
```

Two call sites:

1. **`codelet/fspec-tui/tests/provider_settings_row_render_rpc104.rs:38`** —
   `render_row(kind, label, selected, area, &mut buf)` inside the 18
   RPC-104 unit tests. Currently `let _ = render_row(...)` — adding a
   return value of `u16` is backwards-compatible (callers ignore it).
2. **`codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs:58`** —
   `render_row(kind, &label, selected, row_area, buf)` inside the
   `render_nav_items` paint loop. This is the seam where the new
   decoration paint will plug in: capture the returned `end_x` and, if
   `view.test_result.provider_id == item.provider_id`, paint the
   decoration starting at `end_x`.

## 3. View struct anchor

```
pattern: 'pub struct ProviderSettingsView { $$$FIELDS }'
language: rust
```

Single match:

- `codelet/fspec-tui/src/views/provider_settings/mod.rs:70` — the struct
  carries `providers`, `display_providers`, `expanded`, `nav_items`,
  `selected_index`, `scroll_offset`, `mode`, `filter`, `filter_mode`,
  `delete_confirm`, `status`, `visible_rows`. The new field
  `pub test_result: Option<ProviderTestResult>` will be appended here.
  `ProviderSettingsView::new()` (mod.rs:92) explicitly initialises every
  field — must initialise `test_result: None`.

## 4. Legacy status enum to mirror text bytes

```
pattern: 'pub enum DetailStatus { $$$VARIANTS }'
language: rust
```

Single match:

- `codelet/fspec-tui/src/views/provider_settings/status_text.rs:14` —
  carries `Testing | TestOk { latency_ms } | TestErr { error } |
  RefreshingModels | ModelsRefreshed | SavingCredentials |
  CredentialsSaved | Error`. The new `ProviderTestStatus` mirrors the
  three Test-* variants ONLY, and emits the same visible text:
  - `Testing` → `"Testing…"` (Cyan)
  - `Ok { latency_ms }` → `format!("✓ ok ({latency_ms}ms)")` (Green)
  - `Err { message }` → `format!("✗ {message}")` (Red)
  The non-test variants remain on DetailStatus for legacy fallback paths.

## 5. Risk surface

- **Backwards compat:** Changing render_row to return `u16` is safe — all
  18 RPC-104 tests pre-existing call render_row at statement position and
  discard the result implicitly. Rust unused-result is a warning at most
  for non-`#[must_use]` returns, and warnings are not errors in this
  workspace.
- **No public-API breakage:** the only public addition is the new
  `test_result` field + `ProviderTestResult` + `ProviderTestStatus` types
  + `set_test_result` / `clear_test_result` methods. Existing callers do
  not depend on these.
- **Wide-glyph alignment:** the decoration paint runs AFTER the
  `render_row` re-style pass (the existing wide-cell continuation fix at
  row_render.rs:146) so the wide-glyph alignment for 📁/🔑 icons is
  untouched — Provider rows do not carry those glyphs anyway, so the
  decoration column derivation is straightforward (`end_x` from
  `buf.set_stringn` return).

## 6. Test scaffolding precedent

Sibling tests under `codelet/fspec-tui/tests/` follow the convention:

- `provider_settings_row_render_rpc104.rs` — 18 tests using a fixed
  `Rect { x:0, y:0, width:60, height:1 }` and direct `render_row` calls
  with constructed `RowKind`/`label` pairs.
- `provider_settings_footer_hints_rpc106.rs` — 18 tests using
  `view_for_provider_kind` helper + buffer round-trips.

RPC-158 will follow the same pattern: a `view_with_two_providers()`
helper, `render_to_buffer(view)` wrapper, and one `#[test]` fn per Gherkin
scenario (12 scenarios → 12 tests, plus 3 direct-unit tests).
