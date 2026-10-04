# AST Research — MENU-008: Board menu bar Zone A reorganization (Kanban / Tools / Settings / Help)

Research for work unit **MENU-008** (epic `menu-bar`). Captures the code shape
that must change when the MenuCategories registry goes from 2 categories
(`Actions` 7 entries + `Help` 2) to 4 (`Kanban` 4 + `Tools` 2 + `Settings` 2
+ `Help` 2), plus the new `P` bare-key arm.

## Source of truth

`rust/fspec-tui/src/components/menu_bar/items.rs`

- `MenuAction` enum (line 35): `NewAgent, Search, Attachments, Checkpoints,
  ChangedFiles, Foundation, Mux, Help, Exit`. Gains `Providers`.
- `MenuAction::to_action` (line 61): maps each variant onto an existing
  `Action`. New arm: `Providers => Action::OpenProviderSettingsView`.
- `ACTIONS: &[MenuEntry]` (line 106): the 7 legacy entries
  `., /, C, F, D, M, A` — replaced by `KANBAN` (4: `., /, D, A`).
- `HELP: &[MenuEntry]` (line 152): 2 entries — UNCHANGED.
- `CATEGORIES: &[MenuCategory]` (line 168): `[actions, help]` — becomes
  `[kanban, tools, settings, help]`.
- New consts: `TOOLS: &[MenuEntry]` = `F Changed Files`, `C Checkpoints`;
  `SETTINGS: &[MenuEntry]` = `M Mux`, `P Providers`.

## Consumers of `CATEGORIES` (AstGrep: `CATEGORIES`, rust/fspec-tui/src)

| File | Line | Role | Impact |
|------|------|------|--------|
| `app/dispatch_menu.rs` | 23, 173, 183 | Execute-row resolution | Index-based; automatic |
| `views/multiplex/menu_snapshot.rs` | 58 | `zone_a: CATEGORIES` | Automatic |
| `views/multiplex/menu_state.rs` | 19, 123, 152 | Ring/dropdown state | Index-based; automatic |
| `views/board/menu_snapshot.rs` | 85 | `zone_a: CATEGORIES` | Automatic |
| `store/board_menu.rs` | 27, 67, 143 | Board ring math (R8) | Index-based (`CATEGORIES.len()`); automatic |
| `components/menu_bar_help.rs` | 58, 80, 247 | `u` help dialog rows | ROW_COUNT const = `ACTIONS.len() + HELP.len()` → recompute (11); import `ACTIONS` |
| `components/menu_bar/mod.rs` | 45, 97 | Re-export + `MenuSnapshot::default` | Re-export list: `ACTIONS` → `KANBAN` |
| `components/menu_bar/items.rs` | 168, 192–194, 221 | Registry + unit tests | Unit tests pin labels/counts |
| `components/menu_bar/dropdown.rs` | 21, 34 | `dropdown_rect` geometry | `CATEGORIES.get(category)`; automatic |
| `components/menu_bar/dropdown_paint.rs` | 23, 45 | Panel painter | Reads category via index; automatic |

## Consumers of the `ACTIONS` const (AstGrep: `ACTIONS`, rust/fspec-tui/src)

- `components/menu_bar_help.rs:58` (import), `:76` (`ROW_COUNT`)
- `components/menu_bar/items.rs:106` (def), `:172` (CATEGORIES), `:195`, `:209` (tests)

All `ACTIONS` references are inside `components/` — rename to `KANBAN` is local.

## Bare-key arms

`rust/fspec-tui/src/views/board/keys.rs::handle_mode_view_key` (line 35):
existing arms `f/c/m/d/a/./u` are all modifier-free-only with
`Ctrl` fall-through guards. The new `P`/`p` arm follows the same pattern
and emits `Action::OpenProviderSettingsView`. This function is shared by the
single Board view and the focused Board pane in mux mode (keyboard isolation
routes Board-pane keys through `BoardView::handle_event`), so the mux surface
gets the arm for free.

## Existing `Action` variants (no new bus tokens needed)

- `OpenProviderSettingsView` — defined in `components/mod.rs:758` (RPC-054);
  dispatched by `app/dispatch_provider_settings.rs:228`; Navigator flips
  `ViewMode::Board`-ish active view to `ProviderSettings` at
  `views/navigator.rs:183`.

## Test files pinning the "Actions" label or 2-category shape
(must be migrated as part of MENU-008)

- `rust/fspec-tui/tests/menu001_menu_bar.rs` (labels, row counts)
- `rust/fspec-tui/tests/menu002_board_surface.rs` (geometry x-coords: "Actions" x15–21, "Help" x23–26)
- `rust/fspec-tui/tests/menu003_agent_surface.rs` (agent bar absent of Actions/Help)
- `rust/fspec-tui/tests/menu004_mux_surface.rs` (bar row text, "Actions" occurrence count)
- `rust/fspec-tui/tests/menu005_menu_bar_help.rs` (help dialog rows)
- `rust/fspec-tui/tests/menu006_active_session_chip_highlight.rs` (bar text)
- `rust/fspec-tui/tests/menu007_agent_board_view_item.rs` (agent bar)
- `rust/fspec-tui/tests/view_board_version_under_logo_board021.rs`
- `rust/fspec-tui/tests/view_board_unit_rpc015.rs`
- `rust/fspec-tui/tests/board_actions_dialog_board023.rs`
- `rust/fspec-tui/tests/board_period_new_agent_rpc395.rs`
- `rust/fspec-tui/tests/board_search_dialog_board022.rs`
- `rust/fspec-tui/tests/mux009_board_m_key.rs`
- `rust/fspec-tui/src/components/menu_bar/paint.rs` (unit test at line 310–316)
- `rust/fspec-tui/src/components/menu_bar/layout.rs` (unit test at line 312)
- `rust/fspec-tui/src/components/menu_bar/mod.rs` (unit tests use `MenuFocus::Item(1)` for Help)

## File budget (300-LoC ceiling)

- `items.rs` currently 276 lines. Adding `MenuAction::Providers` +
  2 new consts (`TOOLS`, `SETTINGS`) + re-shaping `KANBAN`/`CATEGORIES`
  keeps it under 300 if the test module stays focused. If it overflows,
  move the registry's unit tests to `items_tests.rs` or split the consts.
- `views/board/keys.rs` currently 113 lines. The `P` arm adds ~12 lines;
  stays well under 300.
