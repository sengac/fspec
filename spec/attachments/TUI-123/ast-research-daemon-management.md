# TUI-123 AST research (2026-11-05)

```
rust/common/src/saved_daemons.rs:150:1:pub fn update(

rust/fspec-tui/src/compositor.rs:166:5:pub fn update(&mut self, action: Action) -> Option<Action> {

rust/common/src/saved_daemons.rs:190:1:fn write(path: &Path, daemons: Vec<SavedDaemon>) -> Result<(), SavedDaemonError> {

No matches found
No matches found
rust/fspec-tui/src/views/changed_files/mouse.rs:27:5:pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> ChangedFilesEvent {
rust/fspec-tui/src/views/board/workspace_dropdown.rs:152:1:pub(super) fn handle_mouse(
rust/fspec-tui/src/views/board/mouse.rs:47:1:pub(super) fn handle_mouse(view: &BoardView, event: &Event, store: &BoardStore) -> EventResult {
rust/fspec-tui/src/views/checkpoints/keys.rs:81:5:pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> CheckpointsEvent {
rust/fspec-tui/src/components/work_unit_search_dialog_mouse.rs:71:5:pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> EventResult {

No matches found
rust/fspec-tui/src/app/dispatch_daemon_admin.rs:90:5:fn remove_saved_daemon(&mut self, name: &str) {

No matches found
rust/fspec-tui/src/components/menu_bar/items.rs:206:1:pub const SETTINGS: &[MenuEntry] = &[

No matches found
No matches found
No matches found
```

Conclusions:
- Saved-daemon writes all funnel through `write` in rust/common/src/saved_daemons.rs (save/update/remove). It uses a fixed
  `daemons.json.tmp` staging name with create_new and takes no lock, so two concurrent writers (`fspec daemons add` or a
  second TUI) either fail with AlreadyExists or lose an entry (read-modify-write race). BUG-477's SpecLock
  (rust/common/src/spec_lock.rs) is the cross-process flock to reuse on ~/.fspec; its replace_file_atomically does not
  create the temp owner-only, so saved_daemons keeps its own 0600 atomic writer with a unique temp name.
- `validate` only checks the scheme prefix, loopback and token; credentials, query/fragment, empty host and port are not
  checked and the URL is stored untrimmed of a trailing slash. Name rules are only "not empty".
- The board header's workspace label is clickable via workspace_dropdown::handle_mouse (views/board/workspace_dropdown.rs),
  chained first from views/board/mouse.rs; the daemon label (daemon_switcher.rs) has no mouse handling.
- App::remove_saved_daemon (app/dispatch_daemon_admin.rs) removes without confirmation.
- The Settings menu (components/menu_bar/items.rs SETTINGS) holds Mux and Providers only; the board help
  (components/help_content.rs) has no row for `s` or `w`.
- ClientDaemonDirectory::connect (rust/fspec/src/daemon_directory.rs) calls WebSocketFspecBackend::connect_with_bearer with
  no timeout and returns the raw tungstenite error chain.
