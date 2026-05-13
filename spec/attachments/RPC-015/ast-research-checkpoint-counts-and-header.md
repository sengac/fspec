# RPC-015 — AST research notes

## Scope

Pre-implementation AST scan to confirm the integration points the master
plan calls out in spec/attachments/RPC-002/rust-tui-parity-master-plan-2026-05-13.md
exist exactly as expected.

## codelet/git/src/ghost_commit.rs — existing helpers (5 public functions)

`AstGrep --pattern='pub fn $NAME(...)' --path codelet/git/src/ghost_commit.rs`

```
codelet/git/src/ghost_commit.rs:63:1 :pub fn create_ghost_commit(...)
codelet/git/src/ghost_commit.rs:384:1:pub fn restore_ghost_commit(...)
codelet/git/src/ghost_commit.rs:489:1:pub fn list_ghost_checkpoints(dir: &Path, work_unit_id: &str) -> Result<Vec<String>>
codelet/git/src/ghost_commit.rs:519:1:pub fn delete_ghost_checkpoint(...)
codelet/git/src/ghost_commit.rs:551:1:pub fn get_checkpoint_diff_files(...)
```

Conclusion:
- `list_ghost_checkpoints` already iterates `repo.references()` filtering
  by a single `refs/fspec-checkpoints/{work_unit_id}/` prefix. We CANNOT
  re-use it directly because we need to aggregate across all work units.
- We extend the module with a new `count_checkpoints(&Path) -> Result<CheckpointCounts>`
  that walks every ref under `refs/fspec-checkpoints/` and classifies the
  last path segment via `name.contains("-auto-")`.
- All other helpers stay untouched.

## codelet/rpc-types/src/lib.rs — existing structs (16 public structs)

`AstGrep --pattern='pub struct $NAME { ... }' --path codelet/rpc-types/src/lib.rs`

Already lifted shapes: `WorkUnitInfo`, `SessionId`, `SessionInfo`, `LogRecord`,
`HealthInfo`, `CompactionProgress`, `ToolCallInfo`, `ToolResultInfo`,
`ToolProgressInfo`, `ContextFillInfo`, `SupervisorPendingInjectionInfo`,
`IncomingMessageImage`, `CompactionResult`, `FspecRequest`, `FspecResult`,
`TokenTracker`.

Conclusion:
- The cfg-gated `#[cfg_attr(feature = "napi", napi_derive::napi(object))]`
  + serde `Serialize/Deserialize` pattern is well-established. New
  `CheckpointCounts` follows the same pattern (Debug + Clone + Copy +
  Default + PartialEq + Eq + Serialize + Deserialize + napi-object cfg).

## codelet/rpc/src/lib.rs — FspecService trait + impl shape

Existing surface (already read):
- `#[tarpc::service]` trait `FspecService` has 7 methods: `list_work_units`,
  `list_sessions`, `create_session`, `send_input`, `interrupt`,
  `get_session_status`, `health`.
- `SharedFspecService` struct holds a `watcher: ArcSwap<WorkUnitsWatcher>`
  and optional `session_manager` etc. — NO concept of a workspace cwd yet.
- `FspecServiceImpl` is the cloneable adapter that implements the trait.

Decision:
- Add `cwd: Option<PathBuf>` to `SharedFspecService`. Hosts that want real
  counts pass `Some(cwd)`; tests that don't care default to `None` and
  receive `{0,0}` (mirrors TS `countCheckpoints` returning `{0,0}` for a
  missing `.git/` dir). New `with_cwd_and_session_manager(...)`
  constructor wires this into existing hosts (rpc-server bin, embedded
  ratatui host, napi factory).

## codelet/fspec-tui/src/views/board.rs — render orchestrator

`AstGrep --pattern='pub fn $NAME(...) { $$$BODY }' --path codelet/fspec-tui/src/views/board.rs`

```
codelet/fspec-tui/src/views/board.rs:132:5:pub fn render_with_store(&self, area: Rect, buf: &mut Buffer, store: &BoardStore)
```

The orchestrator already uses `Layout::default().direction(Vertical).constraints([...])`
with 9 splits (top border 1 / details 5 / ├┬┤ 1 / column header 1 / ├┼┤ 1 /
content Min / ├┴┤ 1 / footer 1 / bottom border 1).

Decision:
- Insert two new constraints BEFORE the details strip (positions 1+2):
  `Length(4)` for the header strip + `Length(1)` for the new `├──┤`
  separator. Total layout grows from 9 splits → 11 splits.
- Header strip painted by a new `paint_header(area, buf, &theme, &store)`
  function that fans out into the three new widget modules.

## codelet/fspec-tui/src/transport/mod.rs — FspecBackend trait

Existing surface: `list_work_units / list_sessions / create_session /
send_input / interrupt / work_units_rx / chunks_rx / logs_rx / health /
request_manual_reconnect`. Add `async fn checkpoint_counts() -> Result<CheckpointCounts>`.
Both `EmbeddedFspecBackend` and `WebSocketFspecBackend` add one-line
tarpc delegates per the established RPC-009 pattern.

## codelet/fspec-tui/src/components/mod.rs — Action enum

The Action enum already has 24 variants (Quit / Redraw / Custom / LoadWorkUnits /
WorkUnitsLoaded / SessionCreated / ChunkReceived / InputSubmitted / Interrupt /
FocusNext / Disconnected / Reconnecting(u32) / Reconnected / ManualReconnect /
EnterWorkUnit / OpenAgentView / BackToBoard / NavigationTargetSet /
AttachSession / FocusPrevColumn / FocusNextColumn / SelectNext / SelectPrev /
ReorderUp / ReorderDown).

Decision:
- Append one new variant `CheckpointCountsLoaded(CheckpointCounts)`.

## TS reference — already read

- `src/utils/checkpoint-index.ts` defines `AUTO_CHECKPOINT_PATTERN = '-auto-'`
  and `isAutomaticCheckpoint(name) = name.includes(AUTO_CHECKPOINT_PATTERN)`.
- The TS `countCheckpoints(cwd)` reads
  `.git/fspec-checkpoints-index/{workUnitId}.json` sidecar files. We don't
  read the sidecar in Rust — we walk git refs directly (the source of
  truth), which is equivalent for counting purposes because the TS code
  writes the sidecar in `git-checkpoint.ts:updateCheckpointIndex` after
  every NAPI ghost-checkpoint create, so both sources should agree.
- `src/tui/components/Logo.tsx` defines the 4-row ASCII art block:
  ```
  ┏┓┏┓┏┓┏┓┏┓ 
  ┣ ┗┓┃┃┣ ┃ 
  ┻ ┗┛┣┛┗┛┗┛ 
  ```
  (4th row is blank/padding). Width = 11 visible chars + 1 trailing
  space = 12 cells.
- `src/tui/components/CheckpointStatus.tsx` paints `Checkpoints: None`
  when both counts are zero, else `Checkpoints: N Manual, M Auto`.
- `src/tui/components/KeybindingShortcuts.tsx` paints
  `C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ / New Agent`
  with a `borderTop` divider above it.

## Test harness conventions

- Cross-transport parity tests follow `tests/cross_transport_app_parity_rpc009.rs`
  and `tests/ws_backend_smoke.rs` — common helpers in `tests/common/` already
  provide `ws_server_for(...)`.
- Render tests follow `tests/view_board_unit_rpc014.rs` against a
  `TestBackend` (ratatui's `Buffer` API for character-level assertions).
- Source-shape tests follow `tests/source_shape_rpc014.rs`.

## Conclusion

All integration points expected by the master plan exist. The implementation
strategy from `spec/attachments/RPC-015/typescript-reference.md` is sound;
no architectural changes required beyond what is sketched in the rules and
architecture notes on the work unit.
