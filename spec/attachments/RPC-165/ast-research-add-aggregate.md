# AST Research — `add-aggregate` (RPC-165)

## Scope
Port TS `src/commands/add-aggregate.ts` → Rust `codelet/fspec-core/src/commands/add_aggregate.rs`
(core impl, INLINE style) + CLI bridge `codelet/fspec/src/add_aggregate.rs` + help config.

## TS source AST analysis (`src/commands/add-aggregate.ts`)
- `export async function addAggregate(options: AddAggregateOptions): Promise<AddAggregateResult>` (line 34)
  - Inlines the entire Event Storm mutation. Does NOT call a shared `addEventStormItem` util.
  - `AddAggregateOptions`: `{ workUnitId, text, responsibilities?: string (CSV), timestamp?: number, boundedContext?: string, cwd? }`
  - `AddAggregateResult`: `{ success, error?, aggregateId? }`
- `export function registerAddAggregateCommand(program)` (line 152)
  - Commander surface: `add-aggregate <workUnitId> <text>` with options
    `--responsibilities <list>`, `--timestamp <ms>` (parseInt), `--bounded-context <context>`.
  - Success path: `logger.success('Added aggregate "<text>" to <id> (ID: <aggregateId>)')`.
  - Failure path: `logger.error(error); process.exit(1)`.

## Control flow (verbatim parity targets)
1. `cwd = options.cwd || process.cwd()`; `workUnitsFile = join(cwd,'spec','work-units.json')`.
2. **Missing file** (`!existsSync`) → `{success:false, error:'spec/work-units.json not found. Run fspec init first.'}`.
3. Missing work unit → `{success:false, error:'Work unit <id> not found'}`.
4. status `done`|`blocked` → `{success:false, error:'Cannot add Event Storm items to work unit in <status> state'}`.
5. Init `eventStorm` when absent → `{ level:'process_modeling', items:[], nextItemId:0 }`.
6. Build item, field order: `id, type('aggregate'), color('yellow'), text, deleted(false), createdAt(now ISO)`,
   then optionals appended in order: `responsibilities` (CSV `split(',')`→`trim()`→`filter(len>0)`),
   `timestamp` (only if `!== undefined`), `boundedContext` (only if truthy).
7. `items.push(item)`; `nextItemId++`; `updatedAt = now`; `meta.lastUpdated = now`.
8. Persist via `fileManager.transaction` (single write).

## Reference Rust ports consulted
- `codelet/fspec-core/src/commands/add_rule.rs` — canonical mutation port shape: serde args struct
  (`#[serde(rename_all="camelCase")]`), `WorkUnit.extra` round-trip via `#[serde(flatten)]`,
  `iso8601_now()`, `write_json_atomic`. NOTE: add_rule reuses `ensure_work_units_file` (auto-creates);
  **add-aggregate MUST NOT auto-create** — supervisor ruling Option B: inline path-exists check + read,
  error `'spec/work-units.json not found. Run fspec init first.'` when missing.
- `codelet/fspec-core/src/commands/show_event_storm.rs` — how `eventStorm.items` is read from
  `wu.extra["eventStorm"]["items"]`.
- `codelet/fspec-core/src/dispatch.rs` — `run_ported` arm wiring (supervisor-owned).

## Item JSON shape on disk (eventStorm.items[])
```json
{ "id":0, "type":"aggregate", "color":"yellow", "text":"Order",
  "deleted":false, "createdAt":"<iso>", "responsibilities":["..."],
  "timestamp":123, "boundedContext":"..." }
```

## Two-front-doors
Core `run(args_json, project_root)` returns `{success, aggregateId}` JSON. CLI bridge parses that JSON
to render `✓ Added aggregate "<text>" to <id> (ID: <n>)` to stdout; errors → stderr, exit 1.
No domain logic in the bridge.
