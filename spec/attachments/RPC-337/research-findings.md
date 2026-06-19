# RPC-337 — Research Findings

Full-screen Model Selector mode-view + shared full-screen shell extraction.

## Origin / context cards

| Card | Title | Status | Relevance |
|------|-------|--------|-----------|
| **RPC-022** | Modal dialogs: ModelSelector + ThinkingLevel + RoleBanner | done | Ported TS `ModelSelectorScreen.tsx` (199L) + `ModelSelectorView.tsx` (210L) into Rust — but as a **centered Compositor modal** (`components/model_selector_dialog.rs`, `Priority::Foreground`, `render_dialog` adapter), NOT a full-screen view. This is the regression we are fixing. (RPC-337 `dependsOn` RPC-022.) |
| **RPC-054** | /provider ProviderSettingsView + provider-credentials RPC surface | done | Converted the provider screen into a proper full-screen **mode-view** (`views/provider_settings/`), owned by `Navigator` via `ViewMode::ProviderSettings`. It reused individual helpers instead of extracting a **shared full-screen shell component** — leaving ~3 hand-rolled copies of the scaffold. (RPC-337 `relatesTo` RPC-054.) |

## The original TypeScript full-screen model selector

Three cooperating layers (TUI-072/073/075 split):

| Layer | File | Role |
|-------|------|------|
| State | `useModelSelectorState.ts` | Zustand `modelStore` data + local selection/scroll/filter |
| Orchestrator | `ModelSelectorScreen.tsx` | composes state + `useInput` + chooses which sub-view to render |
| Presentation | `ModelSelectorView.tsx` | pure stateless full-screen layout |

**Mounted full-screen, NOT modal** — early `return` in `AgentView.tsx` replaces the whole screen:

```tsx
if (showModelSelector) {
  return (
    <ModelSelectorScreen
      width={terminalWidth}
      height={terminalHeight}
      currentModelId={currentModel?.apiModelId}
      onSelectModel={handleModelSelect}
      onClose={() => setShowModelSelector(false)}
      onSwitchToSettings={() => { setShowModelSelector(false); setShowSettingsTab(true); }}
    />
  );
}
```

Outer box sizes to full terminal with a black background:
```tsx
<Box flexDirection="column" width={width} height={height} backgroundColor="black">
```

### Layout regions (top → bottom)
**Header → optional Filter → flex-grow List + proportional Scrollbar → Footer → Legend**

- Header: `Select Model` (cyan bold) + `(refreshing...)` + `(N models)` dim count.
- Filter row: shown only in filter mode or when filter active; `Filter: <text>` + fake cursor.
- List: `flatItems.slice(scrollOffset, scrollOffset + visibleHeight)` (manual windowing); scrollbar column draws proportional thumb `■` over track `│` when content exceeds viewport.
- Footer hints: `Enter: select | ←→: collapse/expand | r: refresh | a: add model | e: edit | d: delete | / filter | Esc: close`.
- Legend: `[R] Reasoning | [V] Vision | [C] Custom | 📁 Profile (local server)`.

### Data model + navigation
- `ProviderSection[]` (cloud providers + 📁 profile sections), each with `providerId`, `providerName`, `models`, optional `profileName`/`profileConfig`, `isUnreachable`, `customModelIds`.
- Flattened to `ModelSelectorItem[]` via `buildFlatModelList(sections, expandedProviders)` — model rows only emitted for expanded providers.
- `filterFlatItems(flatItems, filter)` narrows the list.
- Two-axis selection: `selectedSectionIdx` + `selectedModelIdx` (`-1` = section header). `findCurrentFlatIndex(...)` reconciles to a single highlighted flat index.
- Keys: ↑/↓ navigate (adjust scroll), ← collapse, → expand, Enter toggles a section / commits a model (`selectModel` → `onSelectModel` → `onClose`), `/` filter, `r` refresh, Tab → settings, `a`/`e`/`d` custom-model CRUD (out of scope for first Rust pass, like RPC-022).
- `visibleHeight = height - 6` (reserves header/filter/footer/legend rows).

## The Rust canonical full-screen "mode-view" pattern

Reference: `ResumeSessionView` (RPC-026); `ProviderSettingsView` (RPC-054) mirrors it.

```rust
pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
    Clear.render(area, buf);                       // fully overwrite underlying view
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // [0] title
            Constraint::Length(1),  // [1] separator
            Constraint::Min(0),     // [2] body (flexes)
            Constraint::Length(1),  // [3] footer
        ]).split(area);
    render_title_with_count(split[0], buf, "...", count, "...");
    /* body branches on mode */
    render_footer_hint(split[3], buf, &self.footer_hint());
    if let Some(d) = self.delete_confirm.as_ref() { d.render(area, buf); } // ConfirmDialog overlay
}
```

Shared helpers already promoted to `pub(crate)`:
- `views/agent/mode_view_render.rs`: `render_title_with_count(area, buf, title, count, suffix)`, `render_footer_hint(area, buf, text)`.
- `components/scroll_viewport.rs`: `ensure_visible(&mut scroll_offset, selected, visible_rows, total)`, `wrap_index(current, delta, total)`, plus `WheelVelocity`/`WheelDirection`.

Ownership/entry:
- `ProviderSettingsView` is a first-class `Navigator` child gated by `ViewMode::ProviderSettings`, flipped only in `Navigator::apply_action` via `Action::OpenProviderSettingsView` / `Action::CloseProviderSettingsView`.
- `handle_event` routes by `active_view` → `handle_provider_settings_event` → `provider_settings.handle_key(*key)` → translate `ProviderSettingsEvent` (`Consumed/Ignored/Emit/Close/SwitchToModels`) onto the action bus.
- Entered via `/provider`: `dispatch_rpc020.rs` `SlashCommandAction::Provider` arm sends `Action::OpenProviderSettingsView`.
- `ResumeSessionView` differs: it is `Option<…>` on `AgentView`, opened by direct call `handle_open_resume_view()`, not a `ViewMode`.

## Existing reusable Rust assets

- `components/model_selector_dialog_rows.rs` already builds the flattened provider/model rows + badges + header-skipping navigation:
  `build_rows`, `build_dialog_rows`, `move_up_skipping_headers`, `move_down_skipping_headers`, `page_step_selectable`, `first_selectable`, `last_selectable`, struct `ModelSelectorRow`. These are reusable by the new mode-view (currently `pub(super)` to `components::` — will need re-scoping).
- Wire types `ProviderInfo { key, display_name, models }` and `ModelEntry { id, display_name, context_window, supports_reasoning, supports_vision, is_custom }` (`codelet/rpc-types/src/lib.rs`) already drive both TS and Rust selectors — unchanged.
- Backend RPC `list_providers()` + `set_session_model()` + `get_model_info()` already wired (see `dispatch_rpc022.rs`).

## The mismatch / debt

1. TS model selector = full-screen view; Rust port = centered modal. **Fix: make it a full-screen Navigator mode-view.**
2. RPC-054 hand-rolled the `Clear` + 4-constraint scaffold instead of extracting a shared shell — now duplicated in `provider_settings/mod.rs`, `agent/resume_session_view.rs`, `agent/search_history_view.rs`. **Fix: extract a shared full-screen shell and refit all consumers.**

## Implementation plan

1. **Extract shared full-screen shell** — e.g. `render_full_screen_scaffold(area, buf, title+count+suffix, footer_hint, body_fn(body_area, buf), overlay)` doing `Clear` + 4-constraint split + title/footer + optional `ConfirmDialog` overlay. Refit `ProviderSettingsView`, `ResumeSessionView`, `SearchHistoryView` (no behaviour change).
2. **New `views/model_selector/`** mode-view mirroring `views/provider_settings/` (`mod.rs` orchestrator, `list.rs`, `rows.rs`, etc., each <300 LoC). Holds rows (reuse `model_selector_dialog_rows.rs`), `selected_index`, `scroll_offset`, `filter`/`filter_mode`, expand/collapse state. Title `Select Model (N models)`; body = flattened provider/model list with `[R]/[V]/[Nk]/[C]` badges + proportional scrollbar; footer `Enter Select | ←→ Expand/Collapse | / Filter | r Refresh | Esc Close`. `ModelSelectorEvent` outcome enum mirrors `ProviderSettingsEvent`, emits `Action::ModelSelected(...)`.
3. **Navigator wiring** — add `ViewMode::ModelSelector`, own `model_selector: ModelSelectorView`, `handle_model_selector_event`, render arm, `apply_action` arms for new `Action::OpenModelSelectorView` / `Action::CloseModelSelectorView`. Wire `ProviderSettingsEvent::SwitchToModels` (Tab — currently a no-op) to flip to `ViewMode::ModelSelector`.
4. **Slash command** — change `dispatch_rpc020.rs` `SlashCommandAction::Model` from pushing `ModelSelectorDialog` to dispatching `Action::OpenModelSelectorView` (+ spawn `list_providers`). Retire `Action::OpenModelDialog` + `components/model_selector_dialog.rs` (+ `_rows`) once tests migrate. Provider list still folds in via `Action::ListProvidersLoaded`.
