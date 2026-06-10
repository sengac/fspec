# AST research — `add-dependencies` (RPC-176)

## TS source: `src/commands/add-dependencies.ts`

### Public surface
- `addDependencies(options: AddDependenciesOptions) -> Promise<AddDependenciesResult>`
- `registerAddDependenciesCommand(program)` — Commander.js registration
- Args (programmatic): `{ workUnitId: string, dependencies: { blocks?: string[], blockedBy?: string[], dependsOn?: string[], relatesTo?: string[] }, cwd?: string }`
- Args (CLI): `add-dependencies <workUnitId> [--blocks <ids...>] [--blocked-by <ids...>] [--depends-on <ids...>] [--relates-to <ids...>]`
- Result: `{ success: true, added: number }`

### Observed semantics (TS file, lines 25–83)
1. Resolves `cwd` from `options.cwd ?? process.cwd()`.
2. For each of the four optional arrays (`blocks`, `blockedBy`, `dependsOn`, `relatesTo`) in **declaration order**, iterates the array and calls `addDependency()` with the singular flag (e.g. `blocks: targetId`), counting each successful call into `added`.
3. Order of processing: `blocks` → `blockedBy` → `dependsOn` → `relatesTo`. Within each, original array order.
4. If any inner `addDependency` throws, the iteration aborts immediately and the error propagates out (partial state may already have been written by earlier iterations because `addDependency` writes atomically per call).
5. CLI bridge prints `✓ Added <n> dependencies successfully` on success; on error prints `✗ Failed to add dependencies: <message>` and `process.exit(1)`.

### Delegates to `addDependency` (src/commands/add-dependency.ts)
For each singular relationship type:
- **blocks** (bidirectional):
  - Validates target exists, no self-dep, no duplicate.
  - Cycle detection via DFS over `blocks` adjacency — throws `Circular dependency detected: <from> -> <path>`.
  - Pushes `targetId` into `workUnit.blocks` (creates array if absent).
  - Pushes `workUnitId` into `target.blockedBy` (creates array if absent).
  - **Auto-status side-effect**: if `target.status` is not `blocked` and not `done`, switches it to `blocked` and updates `data.states` arrays.
- **blockedBy** (inverse, bidirectional):
  - Same validation. Cycle detection runs from the BLOCKER's perspective (`detectCircularDependency(workUnits, blockedBy, workUnitId)`).
  - Pushes `targetId` into `workUnit.blockedBy`.
  - Pushes `workUnitId` into `target.blocks`.
  - **Auto-status side-effect on SELF**: if `workUnit.status` is not blocked/done, switches it to `blocked` AND sets `blockedReason = "Blocked by <targetId>"`.
- **dependsOn** (unidirectional):
  - Validates, no self-dep, no duplicate.
  - Pushes onto `workUnit.dependsOn`. No reverse, no status change.
- **relatesTo** (bidirectional, symmetric):
  - Validates, no self-dep, no duplicate.
  - Pushes onto `workUnit.relatesTo` AND onto `target.relatesTo` (idempotent guard `!includes`).
- Updates `workUnit.updatedAt = new Date().toISOString()`.
- Writes via `fileManager.transaction(workUnitsFile, ...)`.

### Persistence
- File: `spec/work-units.json` (resolved as `join(cwd, 'spec/work-units.json')`).
- Atomic write: TS uses `LockedFileManager.transaction` (lockfile + temp-write + rename).
- Rust equivalent: `crate::io::ensure::ensure_work_units_file` for load + `crate::io::locked_file::write_json_atomic` for save.

### Error surfaces
- `Work unit '<id>' does not exist`
- `Target work unit '<id>' does not exist`
- `Cannot create self-dependency`
- `Dependency already exists`
- `Circular dependency detected: <from> -> <path>`

### Rust port plan
- File: `codelet/fspec-core/src/commands/add_dependencies.rs`.
- Signature: `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
- Args struct (camelCase via serde):
  ```rust
  struct AddDependenciesArgs {
      work_unit_id: String,
      dependencies: DepFlags,
  }
  struct DepFlags {
      blocks: Option<Vec<String>>,
      blocked_by: Option<Vec<String>>,
      depends_on: Option<Vec<String>>,
      relates_to: Option<Vec<String>>,
  }
  ```
- Result JSON: `{"success": true, "added": <n>}` via `#[derive(Serialize)]` struct.
- Implementation strategy: load once with `ensure_work_units_file`, then walk the 4 arrays in order, performing the in-memory mutation logic locally (mirror of `add-dependency` semantics), writing back ONCE at the end with `write_json_atomic`. This differs slightly from TS (which writes per-call) but preserves observable end-state for the supervisor-validated dispatcher tests; per-call writes would race against the same lock in Rust.
- CLI bridge: `codelet/fspec/src/add_dependencies.rs` — clap struct with 5 fields (id + four optional `Vec<String>` for each relationship type), marshalls to JSON, calls `add_dependencies::run`.
- Help fixture: capture from TS `node dist/index.js add-dependencies --help`.

### Fields touched on WorkUnit
- `blocks`, `blockedBy`, `dependsOn`, `relatesTo` — all stored in `WorkUnit.extra` (Map<String, Value>).
- `status`, `blockedReason` — `status` is a typed enum field, `blockedReason` lives in `extra`.
- `data.states.*` arrays must be kept in sync when auto-transitioning to `blocked`.
- `updated_at` must be bumped on the SOURCE work unit.

### Shared-file change requests for supervisor
- None required. `ensure_work_units_file` + `write_json_atomic` are sufficient.
- Note: when batched-write semantics differ visibly (e.g. partial writes when an inner call throws), supervisor may need to clarify the contract. Recommendation: write-once at the end is the cleanest Rust-side semantic and matches what end-users observe after a successful run.

### Architecture notes for the work-unit
- Two-front-doors invariant maintained — CLI bridge is pure JSON marshalling.
- All mutations go through a single `write_json_atomic` call at the end.
- Status side-effects on TARGET (for `blocks`) and on SELF (for `blockedBy`) are preserved.
- `blockedReason` extra field is updated on SELF when `blockedBy` causes auto-transition.
