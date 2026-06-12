# AST research — add-bounded-context (RPC-172)

TS source of truth: `src/commands/add-bounded-context.ts` + shared `src/commands/event-storm-utils.ts` (`addEventStormItem`).

## TS structure
- `addBoundedContext(workUnitId, text, options)` validates via the shared util, then appends an Event Storm item of `type: 'bounded_context'`.
- `addEventStormItem(workUnitId, itemData)`:
  - `existsSync(spec/work-units.json)` first → if missing, error `"spec/work-units.json not found. Run fspec init first."` (NO auto-create).
  - Loads work unit; missing → `"Work unit <id> not found"`.
  - `done`/`blocked` → `"Cannot add Event Storm items to work unit in <status> state"`.
  - Seeds `wu.extra.eventStorm = { level: 'process_modeling', items: [], nextItemId: 0 }` when absent.
  - Item shape (insertion order): `{ type, color, text, [description], [timestamp], [boundedContext], id, deleted:false, createdAt }`.
  - bounded_context `color` is JSON `null` (present, not omitted).
  - `id = nextItemId`; then `nextItemId += 1`.
  - Dispatcher result `{ success: true, boundedContextId: id }`.

## Rust port mapping
- `codelet/fspec-core/src/commands/add_bounded_context.rs` — `run(args_json, project_root)`, inline existsSync (Option B), whole-doc serde_json::Value round-trip (preserve_order) for byte-exact key order.
- `codelet/fspec/src/add_bounded_context.rs` — marshalling-only CLI bridge, `--bounded-context` → field `context`, delegates through `dispatch_command`.

## AST anchors examined
- `function addEventStormItem(` / `function addBoundedContext(` in TS.
- Rust reference: `commands/add_rule.rs` (mutation+atomic write), `commands/show_event_storm.rs` (eventStorm read shape), `commands/create_epic.rs` (write port).
