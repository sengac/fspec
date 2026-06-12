# AST Research — RPC-187 `add-policy` Rust port

## TypeScript source (to port)
- `src/commands/add-policy.ts` — `addPolicy(options)` builds an `itemData`
  literal `{type:'policy', color:'purple', text}` then conditionally adds
  `when`, `then`, `timestamp`, `boundedContext` and delegates to the shared
  `addEventStormItem<EventStormPolicy>` helper. Maps `result.itemId` -> `policyId`.
- `src/commands/event-storm-utils.ts:29` — `addEventStormItem<T>(options)`:
  - `existsSync` guard -> "spec/work-units.json not found..." (TS), but the
    Rust port follows the `add_rule.rs` precedent of `ensure_work_units_file`
    (auto-create) and then reports "Work unit <id> not found".
  - Reads work-units, validates `workUnits[id]` exists -> "Work unit <id> not found".
  - Rejects `status === 'done' || 'blocked'` -> "Cannot add Event Storm items
    to work unit in <status> state".
  - Seeds `wu.eventStorm = {level:'process_modeling', items:[], nextItemId:0}`
    when absent.
  - `itemId = nextItemId`; pushes `{...itemData, id, deleted:false, createdAt}`;
    increments `nextItemId`.

## Rust reference shape (copy this)
- `codelet/fspec-core/src/commands/add_rule.rs:67` —
  `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
  - `ensure_work_units_file(project_root)` (auto-create on missing).
  - `data.work_units.contains_key(id)` guard.
  - status guard captured BEFORE mutation.
  - mutates `wu.extra` map; single `write_json_atomic` at end.
  - `iso8601_now()` for timestamps.
- `codelet/fspec-core/src/commands/show_event_storm.rs` — shows how
  `eventStorm.items` is read from `wu.extra["eventStorm"]["items"]`.

## Key parity decisions
- eventStorm sub-object round-trips via `WorkUnit.extra` (serde flatten).
- items[0] key order MUST be: type, color, text, when, then, boundedContext,
  id, deleted, createdAt (TS object-literal insertion order). Use a
  serde_json::Map (preserve_order feature on serde_json in this workspace).
- Dispatcher success data = `{policyId: <id>}`.
- CLI bridge marshals `{workUnitId, text, when?, then?, timestamp?, boundedContext?}`
  only — no domain logic.
