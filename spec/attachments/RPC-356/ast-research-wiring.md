# RPC-356 AST research — ChangedFilesView wiring points

AST queries (rust) run against the fspec-tui crate to locate the exact
extension points for the dual-pane Changed Files view.

## ViewMode enum (Navigator)
- `pub enum ViewMode { ... }` — `codelet/fspec-tui/src/views/navigator.rs:29`
  Variants today: Board, Agent, ProviderSettings, Blocklist, ModelSelector.
  Need to add: `ChangedFiles`.

## BoardView key handler
- `pub fn handle_event(&self, event: &Event, store: &BoardStore) -> EventResult`
  — `codelet/fspec-tui/src/views/board.rs:104`
  Add a `KeyCode::Char('f') | KeyCode::Char('F')` arm that emits
  `Action::OpenChangedFilesView` and returns `consumed()`.

## Action enum
- `pub enum Action` — `codelet/fspec-tui/src/components/mod.rs:107`
  Add: `OpenChangedFilesView`, `CloseChangedFilesView`,
  `ChangedFilesLoaded(Vec<ChangedFile>)`,
  `FileDiffLoaded { path, diff }`,
  `ChangedFilesScroll(i32)` mouse/key scroll routing.

## Data loading (App::dispatch)
- `CheckpointCountsLoaded` flow at `app/dispatch.rs:72` + bootstrap
  `backend.checkpoint_counts()` at `app/bootstrap.rs:33` — mirror this for
  `backend.changed_files()` / `backend.file_diff(path)`.
- Backend methods already exist (RPC-355):
  `FspecBackend::changed_files()` and `FspecBackend::file_diff(path)` —
  `codelet/fspec-tui/src/transport/mod.rs:108,115`,
  `transport/embedded.rs:119,124`, `transport/websocket.rs:286,293`.

## Reuse primitives
- `WheelVelocity` + `ensure_visible` + `wrap_index` —
  `codelet/fspec-tui/src/components/scroll_viewport.rs`.
- Mode-view event routing template: `views/navigator_events.rs` (handle_*),
  `views/blocklist/mod.rs` (BlocklistEvent outcome enum).

## Wire type
- `codelet_rpc_types::ChangedFile { path, change_type, staged }` —
  `codelet/rpc-types/src/lib.rs:94`.
