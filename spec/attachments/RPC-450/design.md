# RPC-450 design: workspace-aware work unit details and reordering

## Problem

- `get_work_unit_details(id)` reads `self.inner.cwd()` — the daemon's start
  workspace (`rust/rpc/src/service_implementation/schedule_loops.rs:76-83`).
- `move_work_unit_up(id)` / `move_work_unit_down(id)` also write the start
  workspace (`rust/rpc/src/service_implementation/workspace_history.rs:5-36`).
- RPC-443/RPC-445 let one daemon serve several workspaces: the TUI board can
  show another workspace via `list_workspace_work_units(workspace)`
  (`schedule_loops.rs:89-98`, `inner.workspace(Some(path))` in
  `rust/rpc/src/workspaces.rs:130-166`), and the Paseo plugin does the same.
  Opening or reordering a card on such a board reads/writes the wrong
  `spec/work-units.json` (or fails with "not found").

## Design

- All three RPCs take a required `workspace: String` (an absolute or
  canonicalisable workspace path) as their first argument:
  `get_work_unit_details(workspace, id)`,
  `move_work_unit_up(workspace, id)`, `move_work_unit_down(workspace, id)`.
- The service resolves it with `inner.workspace(Some(Path::new(&workspace)))`,
  exactly as `list_workspace_work_units` does; the start workspace is passed by
  its path like any other. An unknown or unopenable workspace is an error. No
  implicit default to the start workspace.
- Every layer is updated: RPC declaration, service impl, MCP dispatch schema,
  TUI `FspecBackend`, embedded + websocket transports, test mocks.
- The fspec TUI passes its currently selected board workspace
  (`BoardStore::selected_workspace()`, RPC-445 state: the start workspace's
  path until the user picks another). Until the workspace list has loaded the
  board's workspace is unknown, and the TUI refuses instead of guessing: card
  details show "the board's workspace is not known yet" and a reorder is
  logged and skipped. Reorders now run on a parked pending task
  (`app/dispatch_reorder.rs`, split out of `app/dispatch.rs`) and log failures
  at warn instead of debug.
- Error text: details for an unknown workspace fail with "daemon workspace
  unavailable for work unit <id>: workspace path is not a directory: <path>";
  moves fail with "workspace path is not a directory: <path>".
- Tests that relied on the old start-only behaviour were moved onto the
  explicit argument: RPC-017's "returns Err when no cwd is attached" scenario
  became "returns Err for a workspace that does not exist" (with an explicit
  workspace the service no longer depends on its own cwd), and BOARD-024's
  missing-workspace case passes a path that does not exist.

## Alternatives rejected

- `Option<String>` with `None` = start workspace: an implicit default; callers
  would silently act on the start workspace when they forget to pass the
  board's workspace — the exact bug being fixed.
- New `*_in_workspace` RPCs: duplicates and leaves the old start-only methods
  as a trap; back-compat is not required.

## Wire / API changes

- `get_work_unit_details(workspace: String, id: String) -> Result<String, String>`
- `move_work_unit_up(workspace: String, id: String) -> Result<(), String>`
- `move_work_unit_down(workspace: String, id: String) -> Result<(), String>`
Method indices are unchanged; argument tuples gain a leading `String`.
