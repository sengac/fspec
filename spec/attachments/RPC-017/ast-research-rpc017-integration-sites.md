# RPC-017 — AST research (2026-05-15)

AST patterns used to locate the integration sites the new
`move_work_unit_up/_down` plumbing must hook into. Run via the Claude
Code AstGrep tool against the `codelet/` Rust workspace.

## 1. `FspecBackend` trait surface (current methods to mirror)

```
ast-grep --lang rust --pattern 'async fn $NAME($$$ARGS) -> $RET { $$$BODY }' \
  codelet/fspec-tui/src/transport/embedded.rs
```

Matches confirming the trait pattern + return-type shape:
- `embedded.rs:56  async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>>`
- `embedded.rs:64  async fn create_session(&self, role: Option<String>) -> Result<SessionId>`
- `embedded.rs:68  async fn send_input(&self, id: SessionId, text: String) -> Result<()>`
- `embedded.rs:97  async fn checkpoint_counts(&self) -> Result<CheckpointCounts>`

➡️  New methods follow the `send_input` shape: `async fn move_work_unit_up(&self, id: String) -> Result<()>`.

## 2. `FspecService` tarpc trait (current RPC methods)

```
ast-grep --lang rust --pattern 'async fn $NAME($$$ARGS) -> $RET' \
  codelet/rpc/src/lib.rs
```

Matches:
- `rpc/src/lib.rs:53  async fn list_work_units() -> Vec<WorkUnitInfo>`
- `rpc/src/lib.rs:60  async fn create_session(role: Option<String>) -> SessionId`
- `rpc/src/lib.rs:77  async fn health() -> HealthInfo`
- `rpc/src/lib.rs:85  async fn checkpoint_counts() -> CheckpointCounts`

➡️  Add `async fn move_work_unit_up(id: String) -> Result<(), String>` and `_down` to `FspecService`. Errors crossed as `String` for serde/tarpc compatibility.

## 3. Existing mkdir-based lock pattern (lift target)

```
ast-grep --lang rust --pattern 'fn $NAME($$$ARGS) -> $RET { $$$BODY }' \
  codelet/napi/src/schedule_handler.rs
```

Lock-pattern functions to lift into `codelet/common/src/file_lock.rs`:
- `schedule_handler.rs:70   fn acquire_lock(lock_dir: &Path) -> Result<(), String>`
- `schedule_handler.rs:99   fn is_lock_stale(lock_dir: &Path) -> bool`
- `schedule_handler.rs:115  fn release_lock(lock_dir: &Path)`
- `schedule_handler.rs:123  fn with_schedules_lock<F>(project: &str, f: F) -> ScheduleResult`

Constants to lift: `LOCK_STALE_MS = 10_000`, `LOCK_MAX_RETRIES = 10`, `LOCK_MIN_BACKOFF_MS = 50`, `LOCK_MAX_BACKOFF_MS = 500`.

➡️  Public surface of new helper:
```rust
pub fn with_file_lock<F, T>(lock_dir: &Path, f: F) -> Result<T, String>
where F: FnOnce() -> Result<T, String>;
```

`with_schedules_lock` becomes a 3-line wrapper. Generic enough that `move_work_unit` can use it directly with `spec/work-units.json.lock`.

## 4. Existing pure-Rust work-units module (read-only today)

```
ast-grep --lang rust --pattern 'pub fn $NAME($$$ARGS) -> $RET { $$$BODY }' \
  codelet/core/src/work_units.rs
```

Read-side surface (UNCHANGED by RPC-017):
- `work_units.rs:166  pub fn read_snapshot(workspace: &Path) -> Result<Vec<WorkUnitInfo>>`
- `work_units.rs:233  pub fn new(workspace: &Path) -> Result<Self>` (WorkUnitsWatcher)
- `work_units.rs:311  pub fn snapshot(&self) -> Vec<WorkUnitInfo>`
- `work_units.rs:321  pub fn subscribe(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>`

`work_units.rs` is already 413 LoC, exceeding the 300-line ceiling. RPC-017 adds a sibling `codelet/core/src/work_units_write.rs` rather than growing the existing file.

## 5. `Action::ReorderUp / Down` dispatch site (no-op today)

`grep`-confirmed (AST patterns equivalent), per `RPC-017/typescript-reference.md`:

`codelet/fspec-tui/src/app/dispatch.rs:190`:
```rust
Action::ReorderUp | Action::ReorderDown => {
    // RPC-012 architecture note [1]: persistence is out of scope
    // for this slice — placeholder no-op.
}
```

➡️  Split into two arms that read `self.board_store.selected_work_unit()`, clone the id, and `tokio::spawn` `backend.move_work_unit_up/_down(id)`.

## 6. `SharedFspecService::cwd` (added in RPC-015, consumed here)

```
ast-grep --lang rust --pattern 'pub fn $NAME($$$ARGS) -> $RET { $$$BODY }' \
  codelet/rpc/src/lib.rs
```

Confirms:
- `rpc/src/lib.rs:205  pub fn with_cwd(mut self, cwd: PathBuf) -> Self`
- `rpc/src/lib.rs:212  pub fn cwd(&self) -> Option<&PathBuf>`

`FspecServiceImpl::move_work_unit_up/_down` reads `self.inner.cwd()`, errors when `None`, otherwise calls `codelet_core::work_units_write::move_work_unit(cwd, &id, Direction)`.

## 7. NAPI work-units shim (additive surface)

```
ast-grep --lang rust --pattern '#[napi] pub fn $NAME($$$ARGS) -> $RET { $$$BODY }' \
  codelet/napi/src/work_units_watcher.rs
```

Existing exports (UNCHANGED): `start_work_units_watcher`, `stop_work_units_watcher`, `get_work_unit_status`, `get_work_unit`, `get_all_work_units`, `is_work_units_watcher_active`.

➡️  Append: `pub fn move_work_unit_up(cwd: String, id: String) -> napi::Result<()>` and `_down`. Each is a 3-line delegate to `codelet_core::work_units_write::move_work_unit`.

## Integration map

```
TS prioritize-work-unit.ts ───────────────┐ (UNCHANGED, uses TS fileManager.transaction)
                                          │
Rust BoardView '['/']'  → Action::Reorder ↓
                                          │
       App::dispatch ──→ tokio::spawn ──→ FspecBackend::move_work_unit_up/_down
                                                │
                ┌───────────────────────────────┴───────────────────────────────┐
        EmbeddedFspecBackend (tarpc channel)               WebSocketFspecBackend (tarpc + WS)
                ↓                                                      ↓
                └──────────→ FspecService::move_work_unit_up/_down ←──┘
                                          │
                       FspecServiceImpl (single source of truth)
                                          │
                                          ↓
                   codelet_core::work_units_write::move_work_unit(cwd, id, Direction)
                                          │
                                          ↓
                   codelet_common::file_lock::with_file_lock(spec/work-units.json.lock, ...)
                                          │
                                          ↓
                   atomic temp + rename of spec/work-units.json
                                          │
                                          ↓
                   WorkUnitsWatcher fires (existing path)
                                          │
                                          ↓
                   Action::WorkUnitsLoaded → App::dispatch → BoardStore::replace_work_units
```

The TS shim ALSO delegates into this Rust pipeline once `napi.move_work_unit_up/_down` are wired — though for RPC-017 the TS shim is not required to switch over; both paths cooperate through the shared inter-process lock + atomic-rename invariant.
