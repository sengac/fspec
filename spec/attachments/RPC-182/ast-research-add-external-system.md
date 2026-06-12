# AST research — add-external-system (RPC-182)

TS source of truth: `src/commands/add-external-system.ts` + shared `src/commands/event-storm-utils.ts` (`addEventStormItem`).

## TS structure
- `addExternalSystem(workUnitId, text, options)` maps CLI `--type` → item field `integrationType` (TS does NOT strictly validate the enum: REST_API, MESSAGE_QUEUE, DATABASE, THIRD_PARTY_SERVICE, FILE_SYSTEM), then appends an Event Storm item of `type: 'external_system'`.
- Shared `addEventStormItem` semantics (identical to add-bounded-context):
  - `existsSync(spec/work-units.json)` first → missing → `"spec/work-units.json not found. Run fspec init first."` (NO auto-create).
  - missing wu → `"Work unit <id> not found"`; done/blocked → `"Cannot add Event Storm items to work unit in <status> state"`.
  - Seeds `eventStorm = { level:'process_modeling', items:[], nextItemId:0 }` when absent.
  - Item shape (insertion order): `{ type, color, text, [integrationType], [timestamp], [boundedContext], id, deleted:false, createdAt }`.
  - external_system `color` is `"pink"`.
  - `id = nextItemId`; `nextItemId += 1`. Dispatcher result `{ success:true, externalSystemId: id }`.

## Rust port mapping
- `codelet/fspec-core/src/commands/add_external_system.rs` — `run(args_json, project_root)`, inline existsSync (Option B), whole-doc serde_json::Value round-trip (preserve_order) for byte-exact key order; `--type` → `system_type` clap arg → `integrationType` field.
- `codelet/fspec/src/add_external_system.rs` — marshalling-only CLI bridge delegating through `dispatch_command`.

## AST anchors examined
- `function addExternalSystem(` / `function addEventStormItem(` in TS.
- Rust reference: `commands/add_rule.rs`, `commands/show_event_storm.rs`, `commands/create_epic.rs`.
