# AST Research — RPC-303 show-event-storm

## TypeScript Source: `src/commands/show-event-storm.ts` (~117 LOC)

### Function signatures

```ts
export async function showEventStorm(
  options: ShowEventStormOptions
): Promise<ShowEventStormResult>

export async function showEventStormCommand(workUnitId: string): Promise<void>

export function registerShowEventStormCommand(program: Command): void
```

### Public interfaces

```ts
export interface ShowEventStormOptions {
  workUnitId: string;
  cwd?: string;
}

export interface ShowEventStormResult {
  success: boolean;
  data?: EventStormItem[];
  error?: string;
}
```

### Algorithm

1. Read `spec/work-units.json` via `fileManager.readJSON`
2. `workUnit = data.workUnits[workUnitId]` — return error if not found
3. Return error if `!workUnit.eventStorm || !workUnit.eventStorm.items`
4. Filter `items.filter(item => !item.deleted)`
5. Return `data: activeItems`

### CLI surface

```ts
program
  .command('show-event-storm')
  .description('Display Event Storm artifacts as JSON (no semantic interpretation)')
  .argument('<work-unit-id>', 'Work unit ID to query')
```

CLI output: `JSON.stringify(result.data, null, 2)` to stdout, exit 0/1.

NOTE: The CLI does NOT currently expose `--type` filtering despite mentions elsewhere — surface is pure JSON dump of active items.

### Rust port plan

- **fspec_core/src/commands/show_event_storm.rs** — handler returning JSON list
- **fspec_core/src/help/configs/show_event_storm.rs** — help config
- **fspec/src/show_event_storm.rs** — CLI bridge (thin marshaller)
- **fspec/tests/fixtures/help/show-event-storm.txt** — help byte-fixture

### Shared-file change requests for supervisor

- `canonical.rs`: register `show-event-storm`
- `dispatch.rs`: route to handler
- `help/configs/mod.rs`: re-export
- `main.rs`: add clap subcommand with positional `<work-unit-id>` arg
