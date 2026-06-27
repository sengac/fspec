# RPC-354 — AST research for the end-to-end integration test

Goal: locate the public API surface the umbrella acceptance test drives manually
(mirroring the App loop headlessly). No new production code — the feature is fully
implemented by children RPC-355 (backend) and RPC-356 (UI).

## Integration entry points (driven manually in the test)

### Navigator (codelet/fspec-tui/src/views/navigator.rs)
- `pub fn handle_event(&mut self, event: &Event, board_store: &BoardStore) -> EventResult` (line 96)
  — routes the board `F` key while `active_view == Board`.
- `pub fn apply_action(&mut self, action: &Action)` (line 112)
  — `OpenChangedFilesView` flips `active_view` to `ViewMode::ChangedFiles` (line 140);
    `CloseChangedFilesView` flips back to `Board` (line 143).
- `nav.active_view: ViewMode` field; `nav.changed_files: ChangedFilesView` owned child (line 65).
- `pub fn render_with_stores(area, buf, &board_store, &mut agent_store)` (line 155) renders the
  active child — `ViewMode::ChangedFiles => self.changed_files.render(...)` (line 183).

### ChangedFilesView (codelet/fspec-tui/src/views/changed_files/mod.rs)
- `pub fn set_files(&mut self, files: Vec<ChangedFile>)` (line 97)
- `pub fn set_diff(&mut self, path: &str, diff: Option<String>)` (line 108)
- `pub fn selected_path(&self) -> Option<String>` (line 132)
- `pub fn is_empty(&self) -> bool` (line 152)
- empty-state message "No changed files" rendered in render.rs (EMPTY_MESSAGE).

### Board F key (codelet/fspec-tui/src/views/board.rs)
- `KeyCode::Char('f') | KeyCode::Char('F') => emit(Action::OpenChangedFilesView)` (line 175).

### Load flow to mirror (codelet/fspec-tui/src/app/dispatch_changed_files.rs)
- `backend.changed_files().await` -> `set_files`; then for the selected path
  `backend.file_diff(path).await` -> `set_diff`.

### Backend (codelet/rpc-types ChangedFile @ lib.rs:94; transport EmbeddedFspecBackend)
- `EmbeddedFspecBackend::new(handle, service)`; `backend.changed_files()`, `backend.file_diff(path)`.

## Reuse
- temp-git-repo seeding + `service_for` helpers from `tests/changed_files_rpc355.rs`.
- `key(KeyCode)` event helper + TestBackend render pattern from `views/changed_files/tests.rs`.
