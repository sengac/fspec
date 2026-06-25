# RPC-350 — Provider Settings list-mode visual parity regressions vs TypeScript

## Context

The Rust `/provider` view (`codelet/fspec-tui/src/views/provider_settings/`) is a port of
the canonical TypeScript `ProviderSettingsPanel.tsx`
(`src/tui/components/ProviderSettingsPanel.tsx`). The two implementations are supposed to be
**visual parity**. A side-by-side screenshot comparison (TS reference vs Rust port, identical
19-item provider tree, OpenAI expanded with one `qwen` profile, the `+ <create>` row selected)
surfaced four deterministic render-layer regressions plus two non-deterministic data-layer
differences.

This document is the single source of truth for the four **in-scope** render-layer fixes.

### Canonical reference (TypeScript)
- `src/tui/components/ProviderSettingsPanel.tsx`
  - Header: lines **550-555**
  - Provider row + inline decorations: lines **576-633**
  - `(N profile/s)` badge: lines **611-617**
  - Add-profile label: line **766**

### Rust port under change
- `codelet/fspec-tui/src/views/provider_settings/row_render.rs` (single-style row painter)
- `codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs` (`row_kind_and_label`, annotations)
- `codelet/fspec-tui/src/views/agent/mode_view_render.rs` (`render_title_with_count`)
- `codelet/fspec-tui/src/views/full_screen_shell.rs` (scaffold; title closure injection point)
- `codelet/fspec-tui/src/views/provider_settings/mod.rs` (`render`, `title_text`)

---

## In-scope regressions (deterministic)

### R1 — Title color & styling
**TS** (`ProviderSettingsPanel.tsx:550-555`):
```tsx
<Text bold color="yellow">Provider Settings</Text>
<Text dimColor> ({navItems.length} items)</Text>
```
Two spans: **bold yellow** name + **dim gray** ` (N items)`.

**Rust** (`mode_view_render.rs:25-29`): one span `"{title} ({count} {suffix})"` styled
`fg(Blue).bold()` — whole title teal/blue, count not dimmed.

**Expected:** name segment `fg(Yellow).bold()`, count segment dim/gray (ratatui has no
`dimColor`; use `Color::DarkGray` or `Modifier::DIM`). The provider view goes through
`render_full_screen_scaffold` -> `render_title_with_count`, which is **shared** with other views
(Resume Session, etc.). DO NOT change the shared blue-title behavior for those views. Use the
existing `render_full_screen_scaffold_with_title` title-closure variant (already present in
`full_screen_shell.rs:64`) to paint a provider-specific two-span title, OR add a dedicated
title renderer. Pick whichever keeps other views' snapshots unchanged.

### R2 — Missing `(N profile/s)` badge
**TS** (`:611-617`): for `openai` only, when `profileCount > 0`, appends dim
` (N profile)` / ` (N profiles)` (pluralized) AFTER the `(not configured)` / configured suffix.

**Rust** (`list_nav_render.rs:provider_annotation`): builds `"{name}{annotation}"` with NO
profile-count suffix.

**Expected:** for `provider_id == "openai"` with `display.profiles.len() > 0`, append
` ({n} profile)` when n==1 else ` ({n} profiles)`, styled dim. Only openai. The count comes from
`ProviderDisplayInfo::profiles` (already populated by `projection.rs`).

### R3 — Add-profile row label
**TS** (`:766`): `+ Create new profile`.

**Rust** (`list_nav_render.rs:132`):
```rust
NavItemKind::AddProfile => (RowKind::AddProfile, "Add Profile".to_string()),
```
renders `+ Add Profile`.

**Expected:** label text `Create new profile` (the `+ ` glyph is added by the row prefix in
`row_render.rs:row_prefix` via `icons::PLUS`, so only the label string changes ->
`"Create new profile"`).

### R4 — Per-span coloring of inline decorations
**TS** paints each fragment of a provider row with its own color
(`ProviderSettingsPanel.tsx:586-633`):
- name -> white (`color={isSelected ? 'black' : 'white'}`)
- ` ✓ {maskedKey}` -> **green** (`:595`)
- ` [{source}]` -> **dim** (`:599`)
- ` (not configured)` -> **gray** (`:606`)
- ` (N profile/s)` -> **dim** (`:612`)

On a **selected** row every fragment fg flips to **black** to stay readable on the inverted
colour band.

The api-key child row mirrors the same scheme (`:728-749`), with empty state ` (not set)`
gray instead of ` (not configured)`.

**Rust** (`row_render.rs:59-78`, `render_row`): `row_style` returns ONE `Style` for the whole
row; `render_row` paints the entire `prefix + label` (name + `✓ key [env]` + `(not configured)`)
as a single styled run -> flat white. No green key, no dim source, no gray empty-state.

**Expected:** split the provider/api-key row label into styled segments and paint each with its
own fg, matching the TS matrix. On a selected row all segment fgs become `Color::Black` over the
yellow band. This is the largest change: `render_row` currently has a single-`Style` contract,
so it needs a span-aware variant (e.g. accept `Vec<(String, Style)>` segments, or a dedicated
provider/api-key paint path that composes the base band + per-segment fg overrides while keeping
the full-width background band intact — see the existing wide-glyph band-repair loop at
`row_render.rs:148-150` which must be preserved).

> NOTE: RPC-104 Rule [4] (see `show-work-unit RPC-104`) ALREADY specified this green/dim/gray
> behavior and the profile badge. The Rust port regressed against its own spec. The new feature
> file for RPC-350 should re-assert these as parity scenarios with cell-level `TestBackend`
> assertions on fg/bg per column range.

---

## OUT OF SCOPE (documented, do NOT implement here)

### D1 — Anthropic shows `OAuth [Claude]` instead of `✓ sk-ant-…VgAA [env]`
TS detected the `ANTHROPIC_API_KEY` env key; Rust backend returned `masked_key = None`, so
`projection.rs` (PROV-099 gating) synthesized the OAuth annotation. This is a
**credential-detection / environment** difference in `list_provider_credentials`, NOT a render
bug.

### D2 — Z.AI shows `(not configured)` instead of `✓ 5fc6d5…NHC7 [env]`
Same class: the Rust backend did not surface the Z.AI env key present in the TS capture.

Both D1 and D2 require first confirming the two screenshots were captured with **identical**
environment variables. If they were, the fix lives in the providers/credentials layer, not the
TUI. Treat as a separate follow-up work unit after verification.

---

## Acceptance / test strategy (for the implementer — 100% ACDD)

1. Feature file `spec/features/rpc350-provider-settings-list-mode-parity.feature` with one
   scenario per regression R1-R4 (capability-named, parity-asserting).
2. Tests FIRST (red): extend/add a `codelet/fspec-tui/tests/` integration suite using ratatui
   `TestBackend` + cell-level fg/bg assertions (follow the pattern in
   `provider_settings_row_render_rpc104.rs` and `provider_settings_header_count_rpc105.rs`).
   - R1: assert title row cell styles — name cells `Yellow`+`BOLD`, count cells dim/`DarkGray`.
   - R2: assert openai expanded provider row contains ` (1 profile)` with dim style.
   - R3: assert the add-profile row label text is `Create new profile`.
   - R4: assert per-column fg — green over the masked-key span, dim over `[source]`, gray over
     `(not configured)` / `(not set)`; on a selected row assert all those spans are `Black`.
3. `@step` comments on EVERY Gherkin step.
4. `link-coverage` test lines (red), then implement, then `link-coverage` impl lines (green).
5. Run the FULL `cargo test -p codelet-fspec-tui` suite — existing RPC-104/105/107/108 snapshot
   tests MUST still pass (do not regress other views' shared title rendering).
6. Respect the 300-LoC file ceiling; factor a span-aware painter into its own module if needed.

## Definition of Done
- R1-R4 implemented, all new scenarios green, full `codelet-fspec-tui` test suite green.
- No change to non-provider views' title styling.
- Coverage linked (test + impl) for every scenario.
- Feature file validates (`fspec validate`) and tags registered (`fspec validate-tags`).
