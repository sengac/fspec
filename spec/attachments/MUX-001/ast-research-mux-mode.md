# AST Research — MUX-001 (Mux mode)

Generated with the AstGrep tool during MUX-001 discovery.

## Navigator surface (rust/fspec-tui/src/views/navigator.rs)

- `pub enum ViewMode { ... }` — navigator.rs:29 (Board, Agent, ProviderSettings, Blocklist, ModelSelector, ChangedFiles, Checkpoints). MUX-001 adds `Mux`.
- `pub struct Navigator { board, agent, provider_settings, blocklist, model_selector, changed_files, checkpoints, active_view, action_tx }` — navigator.rs:55. MUX-001 adds `mux: MultiplexLayout`.
- `pub fn handle_event(&mut self, event: &Event, board_store: &BoardStore) -> EventResult` — navigator.rs:115. Single match on `active_view`; MUX-001 adds a `ViewMode::Mux` arm delegating to `MultiplexLayout::handle_event`.
- `pub fn apply_action(&mut self, action: &Action)` — navigator.rs:132. MUX-001 adds arms for `MuxToggle` / `MuxConfigApplied` / `MuxExit`.
- `pub fn render_with_stores(&mut self, area, buf, board_store, agent_store)` — navigator.rs:180. MUX-001 adds a `ViewMode::Mux` arm.

## Child view handler signatures (event isolation contract)

- `BoardView::handle_event(&self, event: &Event, store: &BoardStore) -> EventResult` — views/board.rs:147 (takes `&self`; emits Actions; mouse branch in views/board/mouse.rs).
- `AgentView::handle_event(&mut self, event: &Event) -> EventResult` — views/agent/dispatch.rs:50 (Shift+Left/Right → SessionPrev/Next at dispatch.rs:24-32; Esc → AgentEscPressed; Tab → turn-select toggle).
- `ChangedFilesView::handle_event(&mut self, event: &Event) -> ChangedFilesEvent` — views/changed_files/mod.rs:212 (Tab/Left/Right toggle panes; Esc → Close; cached `last_files_rect`/`last_diff_rect` for mouse hit-testing).
- `CheckpointsView::handle_event(&mut self, event: &Event) -> CheckpointsEvent` — views/checkpoints/mod.rs (same shape as ChangedFiles).

## Board minimum-width constraint

- `calculate_column_widths(terminal_width: u16) -> ColumnWidths` — views/board/grid.rs:47; `calculate_viewport_height(terminal_height: u16)` — grid.rs:127. Board renders blank below ~51 cols (7×8 + 6 separators + 2 borders) → MUX-001 `MIN_PANE_WIDTH = 52`.

## App integration points

- `App::dispatch` (app/dispatch.rs:11) — single mutation surface; `EnterWorkUnit` flips `navigator.active_view = ViewMode::Agent` (dispatch.rs:84) — MUX-001 must NOT flip when mux is active (R8).
- `App::handle_event` 4-stage cascade (app/events.rs:53): DisconnectDialog → Compositor → Navigator → app-shortcuts. Mux routes at Stage 3 via Navigator; dialogs stay at Stage 2 (R9).
- `parse_slash_command` (app/slash_parser.rs:90) — MUX-001 adds a `/mux …` branch routed to `app/mux_parser.rs`.
- `handle_input_submitted` (app/dispatch_slash_commands.rs:192) — consumes the parsed slash variant BEFORE `backend.send_input`.

## Multi-pane precedent

- `ChangedFilesView` (views/changed_files/) — 40/60 Percentage split + 1-col divider + `focused_pane: Pane` + Tab switch + cached per-pane `Rect`s in `Cell`s for wheel hit-testing. MUX-001 generalizes this to top-level views with N panes.
