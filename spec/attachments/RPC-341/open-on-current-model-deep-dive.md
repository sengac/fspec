# RPC-341 — Model selector opens on first model instead of the current model

**Severity: HIGH** — selector always opens with the cursor on the first model
rather than the user's active model; combined with RPC-340 (dead scroll) the
current model may be neither highlighted nor easily reachable.

## Summary

TS auto-expands the section containing `currentModelId` and lands the cursor on
that model when the selector opens (`ModelSelectorScreen.tsx:93-119`). Rust's
`set_current_model` only stores the id for the green `(current)` render marker;
`set_providers` always forces the cursor to `first_selectable_or_zero`. The Rust
port dropped the auto-expand-to-current cursor behavior entirely.

Dispatch ordering is favorable: `set_current_model` is called BEFORE
`set_providers` (`dispatch_model_selector.rs:28-29` then `:42-43` via async
`ListProvidersLoaded`), so the seeding can happen synchronously inside
`set_providers` — no TS-style `hasAutoExpanded` latch is needed.

---

## PART 1 — TypeScript original (reference behavior)

- One-shot latch: `ModelSelectorScreen.tsx:80` `const hasAutoExpanded = useRef(false)`
- Auto-expand effect: `ModelSelectorScreen.tsx:93-119`
  - guard `:95-97`: bail if already done / models not loaded / no currentModelId
  - scan sections `:98-100`: `section.models.findIndex(m => m.id === currentModelId)`
  - auto-expand JUST that section `:102-104`
  - seed selection `:105-106`: `setSelectedSectionIdx(si)` + `setSelectedModelIdx(mi)`
  - latch + break `:107-108`
- Defaults absent auto-expand: `useModelSelectorState.ts:146-150`
  (`selectedSectionIdx=0`, `selectedModelIdx=-1`, empty `expandedProviders`)
- Not-found path: latch stays `false`, no setters fire, cursor stays on section 0
  header (`selectedModelIdx=-1`); effect re-runs on next deps change (e.g. refresh)

---

## PART 2 — Rust side (current state)

- `set_current_model` (pure setter): `mod.rs:84-86` (field `mod.rs:50`)
- `set_providers`: `mod.rs:92-101` — expands ALL providers (`:93`), cursor
  fallback to `first_selectable_or_zero` (`:98-100`), never consults
  `current_model_id`
- `first_selectable_or_zero`: `rows.rs:127-129`
- `ModelSelectorRow` shape: `components/model_selector_dialog_rows.rs:24-44`
  - header rows (`rows.rs:72-84`): `selectable=false`, `provider_key=key`, `model_id=""`
  - model rows (`rows.rs:88-98`): `selectable=true`, `provider_key=key`, `model_id=model.id`
- `current_model_id` used ONLY for the green `(current)` marker: read at
  `mod.rs:311` → passed to `render_body` (`mod.rs:321-328`) → marker in
  `rows.rs:148-154` (test `rows.rs:575`)
- Dispatch order: `dispatch_model_selector.rs:28-29` (set_current_model before
  spawn list_providers), `:42-43` (set_providers via ListProvidersLoaded)

---

## PART 3 — Proposed precise Rust changes

### Step 1 — Add row-lookup helper in `rows.rs` (after `first_selectable_or_zero`, ~`:129`)

```rust
/// Index of the first selectable row whose `model_id` matches
/// `current_model_id`. None when no current model or no matching row.
pub(crate) fn index_of_model(
    rows: &[ModelSelectorRow],
    current_model_id: Option<&str>,
) -> Option<usize> {
    let target = current_model_id?;
    rows.iter().position(|r| r.selectable && r.model_id == target)
}
```
(`r.selectable` guard ensures a header — empty `model_id` — can never match.)

### Step 2 — Seed `selected_index` in `set_providers` (replace tail `mod.rs:97-100`)

```rust
self.rebuild_rows();
// RPC-341: seed cursor on the active-session model when present (TS
// auto-expand-to-current, ModelSelectorScreen.tsx:93-119). All providers
// are expanded here, so the current model's row already exists.
if let Some(idx) =
    rows::index_of_model(&self.rows, self.current_model_id.as_deref())
{
    self.selected_index = idx;
} else if self.selected_index >= self.rows.len()
    || !self.row_is_selectable(self.selected_index)
{
    self.selected_index = rows::first_selectable_or_zero(&self.rows);
}
```
- found → cursor lands on the current model (TS `:105-106` parity)
- not found / None → existing validate-or-first-selectable fallback

> Strict TS expansion parity (expand ONLY the current section instead of all)
> is the concern of **RPC-342** — keep expand-all here so this card stays a
> minimal cursor fix and existing tests (`mod.rs:474` asserts `is_expanded("openai")`)
> stay green.

### Suggested tests (near `mod.rs:387`)

```rust
#[test]
fn set_providers_seeds_cursor_on_current_model() {
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(Some("claude-sonnet".to_string()));
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);
    let row = &v.rows[v.selected_index()];
    assert!(row.selectable);
    assert_eq!(row.model_id, "claude-sonnet");
}

#[test]
fn set_providers_falls_back_when_current_model_absent() {
    let mut v = ModelSelectorView::new();
    v.set_current_model(Some("does-not-exist".to_string()));
    v.set_providers(vec![provider("openai", &["gpt-4o"])]);
    assert_eq!(v.selected_index(), rows::first_selectable_or_zero(&v.rows));
}
```

---

## Interaction with sibling cards
- **RPC-340 (scroll):** the seeded cursor may start below the fold; RPC-340 must
  keep it visible. Seed `scroll_offset` in `set_providers` right after computing
  `selected_index`.
- **RPC-342 (collapse-by-default):** if RPC-342 lands first, the expand-only-current
  logic and this cursor-seed should be implemented together (both are the direct
  port of `ModelSelectorScreen.tsx:93-119`). Coordinate to avoid double-implementation.
