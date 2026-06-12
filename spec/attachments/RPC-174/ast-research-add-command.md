# AST Research — `add-command` (RPC-174)

## Scope
Port TS `src/commands/add-command.ts` → Rust `codelet/fspec-core/src/commands/add_command.rs`
(core impl, INLINE style) + CLI bridge `codelet/fspec/src/add_command.rs` + help config.

## TS source AST analysis (`src/commands/add-command.ts`)
- `export async function addCommand(options: AddCommandOptions): Promise<AddCommandResult>` (line 35)
  - Inlines the entire Event Storm mutation. Does NOT call a shared `addEventStormItem` util.
  - `AddCommandOptions`: `{ workUnitId, text, actor?: string, timestamp?: number, boundedContext?: string, cwd? }`
  - `AddCommandResult`: `{ success, error?, commandId? }`
- `export function registerAddCommandCommand(program)` (line 149)
  - Commander surface: `add-command <workUnitId> <text>` with options
    `--actor <actor>`, `--timestamp <ms>` (parseInt), `--bounded-context <context>`.
  - Success path: `output.log(chalk.green('✓ Added command "<text>" to <id> (ID: <commandId>)'))`.
  - Failure path: `output.error('✗ Failed to add command:', error); process.exit(1)`.

## Control flow (verbatim parity targets)
1. `cwd = options.cwd || process.cwd()`; `workUnitsFile = join(cwd,'spec','work-units.json')`.
2. **Missing file** (`!existsSync`) → `{success:false, error:'spec/work-units.json not found. Run fspec init first.'}`.
3. Missing work unit → `{success:false, error:'Work unit <id> not found'}`.
4. status `done`|`blocked` → `{success:false, error:'Cannot add Event Storm items to work unit in <status> state'}`.
5. Init `eventStorm` when absent → `{ level:'process_modeling', items:[], nextItemId:0 }`.
6. Build item, field order: `id, type('command'), color('blue'), text, deleted(false), createdAt(now ISO)`,
   then optionals appended in order: `actor` (only if truthy), `timestamp` (only if `!== undefined`),
   `boundedContext` (only if truthy).
7. `items.push(item)`; `nextItemId++`; `updatedAt = now`; `meta.lastUpdated = now`.
8. Persist via `fileManager.transaction` (single write).

## Difference vs add-aggregate
Only the optional field set (`actor` vs `responsibilities`), `type`/`color` literals, and the CLI
success/error rendering differ. Validation + eventStorm seeding + id-increment logic are identical.
TS add-command help omits the "done state" common-error entry (validation still rejects done/blocked).

## Reference Rust ports consulted
- `codelet/fspec-core/src/commands/add_rule.rs` — canonical mutation port shape. As with add-aggregate,
  **add-command MUST NOT auto-create** — supervisor ruling Option B: inline path-exists check + read,
  error `'spec/work-units.json not found. Run fspec init first.'` when missing.
- `codelet/fspec-core/src/commands/show_event_storm.rs` — `eventStorm.items` read shape.
- `codelet/fspec/src/add_rule.rs` — CLI bridge shape (clap args → JSON → core::run, render line).

## Item JSON shape on disk (eventStorm.items[])
```json
{ "id":0, "type":"command", "color":"blue", "text":"PlaceOrder",
  "deleted":false, "createdAt":"<iso>", "actor":"Customer",
  "timestamp":123, "boundedContext":"..." }
```

## Two-front-doors
Core `run(args_json, project_root)` returns `{success, commandId}` JSON. CLI bridge parses that JSON
to render `✓ Added command "<text>" to <id> (ID: <n>)` to stdout; errors → `✗ Failed to add command: <msg>`
on stderr, exit 1. No domain logic in the bridge.
