# PROV-135 — Profile form fields lack placeholder hints for empty numeric/threshold fields

## Summary

In the Rust `/provider` profile create/edit form, an empty field renders a generic `(empty)` hint for every field. The TypeScript reference renders **dimmed, per-field placeholder hints** when a field is empty. This is a **presentational-only** parity gap.

## TS Reference Behavior

`src/tui/components/ProviderSettingsPanel.tsx:348–359` — when a field value is empty, it renders `<Text dimColor>` with a per-field hint:

| Field                | Placeholder hint (dim)     |
|----------------------|----------------------------|
| Base URL             | `http://localhost:8888`    |
| Context Window       | `128000`                   |
| Max Output Tokens    | `16384`                    |
| Compaction Threshold | `80% or 200000`            |

**CRITICAL**: These are **display hints only**. They are NOT written into the saved profile. TS `handleSave` (`src/tui/inputHandlers/profileFormModeHandler.ts:111–122`) only includes `contextWindow` / `maxOutputTokens` / `compactionThreshold` in the saved config when the user actually typed a value (spread guards: `...(values.x && { x })`). A profile saved with blank fields stores nothing for those keys.

## Current Rust Behavior

`codelet/fspec-tui/src/views/provider_settings/profile_form_render.rs` — `field_line` (lines ~52–80):

```rust
let value_span = if value.is_empty() {
    Span::styled("(empty)".to_string(), Style::default().add_modifier(Modifier::DIM))
} else { ... };
```

So ALL empty fields show `(empty)` — the user never sees `128000` / `16384` / `80% or 200000`.

Note: Base URL is already *prefilled with a real value* (`DEFAULT_PROFILE_BASE_URL = "http://localhost:8888"` in `ProfileForm::new_create()`, `profile_form.rs:64`), so it never renders empty in create mode. In edit mode a stored profile always has a base_url. So the Base URL placeholder is effectively already covered; the real gaps are Context Window, Max Output Tokens, and Compaction Threshold.

## Field Index Mapping

`PROFILE_FORM_FIELDS` (`profile_form.rs:25–31`), index order:
- 0 = Base URL
- 1 = API Key
- 2 = Context Window       → placeholder `128000`
- 3 = Max Output Tokens    → placeholder `16384`
- 4 = Compaction Threshold → placeholder `80% or 200000`

API Key (idx 1) is a password field; when empty it should keep its existing empty/`(empty)` treatment (no numeric placeholder). Base URL (idx 0) placeholder `http://localhost:8888` for completeness/parity, though it is rarely empty.

## Fix

In `profile_form_render.rs::field_line`, when `value.is_empty()`, render the per-field placeholder string (dim) instead of `(empty)`:

- idx 0 → `http://localhost:8888`
- idx 2 → `128000`
- idx 3 → `16384`
- idx 4 → `80% or 200000`
- idx 1 (API Key) → keep current empty/`(empty)` behavior (no numeric hint)

Add a small helper (e.g. `fn placeholder_for(idx: usize) -> Option<&'static str>`) to keep `field_line` clean and testable. Keep the DIM modifier so the hint is visually distinct from a real value.

**Do NOT** modify `build_definition()` — placeholders must never be persisted. This keeps the save contract identical to TS.

## Acceptance Criteria (Example-Mapping seeds)

- **Rule**: An empty Context Window field renders the dim placeholder `128000`.
- **Rule**: An empty Max Output Tokens field renders the dim placeholder `16384`.
- **Rule**: An empty Compaction Threshold field renders the dim placeholder `80% or 200000`.
- **Rule**: An empty Base URL field renders the dim placeholder `http://localhost:8888`.
- **Rule**: Placeholder hints are NEVER saved — a profile submitted with blank numeric/threshold fields stores no value for those keys.
- **Rule**: A field with a real typed value renders that value (not the placeholder).
- **Example**: Open create-profile, tab to Max Output Tokens (empty) → the row shows a dim `16384`.
- **Example**: Type `8192` into Max Output Tokens → the row shows `8192` (not the placeholder); saved definition has `max_output_tokens = 8192`.
- **Example**: Leave Compaction Threshold blank and save → saved definition has no compaction threshold type/value.

## Files In Scope

- `codelet/fspec-tui/src/views/provider_settings/profile_form_render.rs` (placeholder rendering + helper)
- Tests: render-layer test asserting the correct dim placeholder per empty field; a `build_definition()` test proving placeholders are not persisted.

## Out of Scope

- Changing what values get persisted (save contract stays identical to TS).
- Changing the Base URL real default in `new_create()`.
