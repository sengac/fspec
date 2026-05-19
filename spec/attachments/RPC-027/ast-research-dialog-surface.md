# AST Research — RPC-027 dialog surface

## Goal

Identify every Rust dialog/popup file that currently constructs a
`tui_popup::Popup`, every `Block::default()` square-border usage in
dialog code, and the exact `Action` enum and `Component` trait surface
the refactor must extend.

## 1. All `Popup::new(...)` call sites (must be removed)

AstGrep pattern: `Popup::new($$$ARGS)` — language rust — path `codelet/fspec-tui/src`

```
codelet/fspec-tui/src/components/help_dialog.rs:111
codelet/fspec-tui/src/components/model_selector_dialog.rs:257
codelet/fspec-tui/src/components/thinking_level_dialog.rs:201
codelet/fspec-tui/src/components/disconnect_dialog.rs:169
codelet/fspec-tui/src/views/agent/slash_command_popup.rs:158
codelet/fspec-tui/src/views/agent/file_search_popup.rs:160
```

Six call sites in six files. Each MUST be replaced with
`dialog_theme::render_dialog(area, buf, &dialog)`.

## 2. `impl Component for $NAME` (dialog Component impls)

AstGrep pattern: `impl Component for $NAME { $$$BODY }`

```
codelet/fspec-tui/src/components/help_dialog.rs:74          → HelpDialog
codelet/fspec-tui/src/components/model_selector_dialog.rs:188 → ModelSelectorDialog
codelet/fspec-tui/src/components/thinking_level_dialog.rs:142 → ThinkingLevelDialog
codelet/fspec-tui/src/components/disconnect_dialog.rs:95    → DisconnectDialog
codelet/fspec-tui/src/components/hello.rs:42                → HelloComponent (NOT a dialog)
codelet/fspec-tui/src/compositor_tests.rs:73                → TestComp (test-only)
```

Four production `Component` dialogs. `ConfirmDialog`,
`SlashCommandPopup`, `FileSearchPopup` do NOT implement `Component` —
they are owned by their parent views and render directly.

## 3. `Block::default()` usages (square-border legacy)

AstGrep pattern: `Block::default()`

```
codelet/fspec-tui/src/views/agent/confirm_dialog.rs:195
codelet/fspec-tui/src/views/agent.rs:250
codelet/fspec-tui/src/views/agent.rs:258
```

Only `confirm_dialog.rs:195` is dialog-related. The two `agent.rs`
hits are the scrollback / footer chrome — out of scope for RPC-027.

## 4. `Action` enum surface (`codelet/fspec-tui/src/components/mod.rs`)

Existing relevant variants (lines 220–369):

```rust
ThinkingLevelLoaded(SessionId, ThinkingLevel),     // RPC-018 read path
ThinkingLevelSelected(SessionId, ThinkingLevel),   // RPC-022 — Enter
ModelSelected(SessionId, String, String),          // RPC-022 — Enter
SetSessionRole(SessionId, Option<String>),         // RPC-022
SessionRoleLoaded(SessionId, Option<String>),      // RPC-022
ListProvidersLoaded(Vec<ProviderInfo>),            // RPC-022
```

The enum closes at line 370. RPC-027 must add ONE new variant before
the closing brace:

```rust
/// RPC-027: emitted by ThinkingLevelDialog on `D` / `d`. App::dispatch
/// spawns `backend.set_thinking_level_default(...)`. The dialog stays
/// open; no badge refresh is needed because the default does not
/// affect the current session's effective level.
SetThinkingLevelDefault(SessionId, ThinkingLevel),
```

## 5. `Component` trait shape (mod.rs:377–407)

The trait already provides `priority` / `is_active` / `id` /
`handle_event` / `update` / `render`. No new trait methods are needed
for RPC-027 — the new `D`-key path is entirely inside
`ThinkingLevelDialog::handle_event` returning
`EventResult::Consumed(None)` (no callback — dialog stays open).

## 6. Backend traits to extend

Existing trait surface (RPC-022):

- `FspecService::set_thinking_level(sid, level) -> Result<(), String>`
- `FspecBackend::set_thinking_level(...)` — both transports
- `SessionManagerHandle::set_thinking_level(...)` — default Ok(())

RPC-027 adds the parallel pair:

- `FspecService::set_thinking_level_default(sid, level) -> Result<(), String>`
- `FspecBackend::set_thinking_level_default(...)`
- `SessionManagerHandle::set_thinking_level_default(...)` — default Ok(())

## 7. Snapshot fixture surface

```
codelet/fspec-tui/src/components/snapshots/
  codelet_fspec_tui__components__help_dialog__tests__help_dialog__centered_popup_80x24.snap
codelet/fspec-tui/tests/snapshots/
  app_with_mock_backend__help_dialog_dismissed.snap
  app_with_mock_backend__help_dialog_visible.snap
```

All three must be regenerated. Five new snapshots will be added
(one per migrated dialog without an existing snapshot).

## 8. Conclusion / scope sanity check

The refactor surface is bounded and well-understood:

- 6 files lose their `Popup::new(...)` call
- 1 file loses its `Block::default().borders(Borders::ALL)` call
- 1 new shared module (~250 LoC) is added
- 1 new `Action` variant + 1 new backend RPC trio is added
- 3 existing snapshots regenerate, 5 new snapshots are written
- 0 TypeScript files are touched

No surprise call sites. No hidden dialog subclasses. The work is
ready to move to `testing`.
