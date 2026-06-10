# RPC-306 — AST Research: show-foundation-event-storm

**TS Source:** `src/commands/show-foundation-event-storm.ts` (~145 LOC)

## Exported Symbols (AstGrep)

```
src/commands/show-foundation-event-storm.ts:22  export async function showFoundationEventStorm(options: ShowFoundationEventStormOptions = {})
src/commands/show-foundation-event-storm.ts:105 export async function showFoundationEventStormCommand(options)
src/commands/show-foundation-event-storm.ts:132 export function registerShowFoundationEventStormCommand(program: Command)
```

## Behaviour Map

1. Read `${cwd}/spec/foundation.json` via `fileManager.readJSON` with empty-foundation default.
2. If `foundation.eventStorm` is missing → return `{success:true, data:[], message:'No Event Storm data in foundation.json'}`.
3. Filter `foundation.eventStorm.items` removing items where `deleted === true`.
4. If `options.context` provided:
   - Find bounded_context whose `text === options.context`.
   - If found: keep that bounded_context plus any item with `boundedContextId === <id>`.
   - If not found: items = [].
5. If `options.type` provided: filter `item.type === options.type` (case-sensitive snake_case e.g. `domain_event`, `command`, `policy`).
6. CLI wrapper prints `JSON.stringify(data, null, 2)` on success, `'Error:'` prefix on failure.

## CLI Shape (commander)

```
fspec show-foundation-event-storm
  --type <type>     Filter by Event Storm item type
  --context <name>  Filter by bounded context name
```

## Rust Port Plan

- `codelet/fspec-core/src/commands/show_foundation_event_storm.rs::run(args_json, project_root)`
- Reads `spec/foundation.json` as `serde_json::Value` (no strong typing needed — pass-through filter).
- Returns JSON array via DispatchResult.
- CLI bridge `codelet/fspec/src/show_foundation_event_storm.rs` is thin: marshal args + stdout print.
