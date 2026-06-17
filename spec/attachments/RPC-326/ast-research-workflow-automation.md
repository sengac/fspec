# AST research — `workflow-automation` (RPC-326)

## TS source: `src/commands/workflow-automation.ts`

A multi-action dispatcher: `workflowAutomation(action, workUnitId, options)`. Three actions.

### Helpers
- `loadWorkUnits(cwd)` → readFile + JSON.parse `spec/work-units.json`.
- `saveWorkUnits(data, cwd)` → `fileManager.transaction` (= `write_json_atomic`).

### Action 1: `recordWorkUnitIteration(workUnitId, { cwd })`
1. Load work units. If `!workUnits[workUnitId]` → throw `Work unit '${workUnitId}' does not exist`.
2. Ensure `workUnit.metrics = {}`, `workUnit.metrics.iterations = 0` defaults.
3. `workUnit.metrics.iterations += 1`; `workUnit.updatedAt = now`.
4. Save.
   NOTE: writes `metrics.iterations` (NESTED), distinct from RPC-264 `record-iteration` which writes a
   TOP-LEVEL `iterations` field. Different on-disk shape.

### Action 2: `autoAdvanceWorkUnitState(workUnitId, { fromState, event }, { cwd })`
1. Load. If `!workUnits[workUnitId]` → throw `Work unit '${workUnitId}' does not exist`.
2. If `workUnit.status !== fromState` → throw
   `Work unit '${workUnitId}' is in state '${workUnit.status}', expected '${fromState}'`.
3. Determine nextState:
   - `tests-pass` + `testing`        → `implementing`
   - `validation-pass` + `validating` → `done`
   - `specs-complete` + `specifying` → `testing`
   - else → throw `Invalid transition: ${event} from ${fromState}`.
4. `workUnit.status = nextState`.
5. Append to `workUnit.stateHistory` array (create if absent): `{ state: nextState, timestamp: now }`.
6. Update `states` index: remove id from `states[fromState]` (filter), push into `states[nextState]`
   (create if absent, only if not already includes).
7. `workUnit.updatedAt = now`. Save.
   NOTE: NO `completedAt` write (unlike auto-advance RPC-198). Adds `stateHistory` (auto-advance does not).
   Includes a third transition `specs-complete` that auto-advance lacks.

### Action 3: `validateWorkUnitSpecAlignment(workUnitId, { cwd })` — READ ONLY
1. Load. If `!workUnits[workUnitId]` → throw `Work unit '${workUnitId}' does not exist`.
2. glob `**/*.feature` under `spec/features`. For each file, regex `@${workUnitId}\b` global match;
   accumulate match count + filenames.
3. Returns `{ aligned: scenariosFound > 0, scenariosFound, features }`. (Does NOT write.)

### Dispatcher `workflowAutomation(action, workUnitId, options)`
- `record-iteration`   → recordWorkUnitIteration
- `auto-advance` (requires `options.event` && `options.fromState`) → autoAdvanceWorkUnitState
- `validate-alignment` → validateWorkUnitSpecAlignment
- else (including auto-advance missing event/fromState) → throw `Invalid action: ${action}`

### Commander registration `registerWorkflowAutomationCommand` — direct binding (NOT Framing A)
```ts
program.command('workflow-automation')
  .argument('<action>', '...')
  .argument('<work-unit-id>', '...')
  .option('--event <event>', '...')
  .option('--from-state <state>', '...')
  .action(workflowAutomation);    // commander passes (action, workUnitId, options) positionally
```
This binds correctly: Commander passes `(action, workUnitId, options)` to `workflowAutomation`. NOT broken.
On success, the function returns `void` / the result of the sub-action (no console output for the
sub-functions except none). So the CLI prints nothing on success and exits 0; on error throws → exits 1.

The rich `-help.ts` (workflow-automation-help.ts) documents all three actions + examples + notes; the help
fixture is captured verbatim from `node dist/index.js workflow-automation --help`.

## Rust port mapping
- Core `run(args_json, project_root)`: args `{ action, workUnitId, event?, fromState? }` (camelCase via
  `rename_all`). Match on `action`. Reuse `WorkUnitsData` + raw-object round-trip (key-order parity) for the
  two mutating actions; for `validate-alignment` glob `spec/features/**/*.feature` + regex.
- Reuse `write_json_atomic`, `iso8601_now`, feature glob helper (`io::feature_glob`).
- Error strings verbatim (single-quoted ids `'<id>'`). No wrapping prefix (TS sub-functions throw raw;
  dispatcher does not wrap).
- Return JSON envelope per action: record-iteration `{success, iterations}`; auto-advance
  `{success, newState}`; validate-alignment `{aligned, scenariosFound, features}`.
- CLI bridge: marshal `action` + `workUnitId` positionals + `event`/`fromState` flags → core; print nothing
  on success (parity), exit 0; on error print `Error:`/`✗`-style to stderr, exit 1. (Confirm exact stderr
  prefix during PHASE B — TS uses no explicit catch in `.action`, so Commander prints the raw thrown error.)
