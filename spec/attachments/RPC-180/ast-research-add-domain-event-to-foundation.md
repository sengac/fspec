# AST Research — add-domain-event-to-foundation (RPC-180)

## TS source of truth
- `src/commands/add-domain-event-to-foundation.ts`
- `src/commands/add-domain-event-to-foundation-help.ts`

## Twin (already ported, COPY SHAPE)
- `codelet/fspec-core/src/commands/add_command_to_foundation.rs` (RPC-175)
- `codelet/fspec/src/add_command_to_foundation.rs`
- `codelet/fspec-core/src/help/configs/add_command_to_foundation.rs`
- tests: `codelet/fspec-core/tests/add_command_to_foundation.rs`, `codelet/fspec/tests/cli_add_command_to_foundation.rs`

## Behaviour (parity with TS, lines from add-domain-event-to-foundation.ts)
1. cwd default = process.cwd(); foundationPath = `${cwd}/spec/foundation.json`.
2. `fileManager.readJSON(path, default)` with INLINE minimal default
   (version 2.0.0 / project / problemSpace / solutionSpace). Same default as twin.
3. transaction:
   - seed `eventStorm` if missing → `{level:'big_picture', items:[], nextItemId:1}` (line 57-63)
   - find bounded context: `item.type === 'bounded_context' && item.text === contextName`
     (NO !deleted filter on the add path) (line 66-68)
   - missing context → throw `Bounded context '<contextName>' not found` (line 71), NO write
   - build domain event item (lines 75-87):
     `{ id: nextItemId, type:'event', text: eventName, boundedContextId: ctx.id,
        color:'orange', deleted:false, createdAt: new Date().toISOString(),
        ...(description && {description}) }`
   - push item; nextItemId++ (lines 90-91)
4. Auto-regenerate FOUNDATION.md (line 96) — DIVERGENCE: skipped in Rust per add_diagram (RPC-178) precedent.
5. result `{ success:true, message: 'Added domain event "<eventName>" to "<contextName>" bounded context' }`

## KEY DIFFERENCES vs add_command_to_foundation twin
| aspect | command twin | THIS (domain event) |
|--------|-------------|---------------------|
| item `type` | `"command"` | `"event"` |
| item `color` | `"blue"` | `"orange"` |
| 2nd positional arg | `commandName` / `<command-name>` | `eventName` / `<event-name>` |
| message verb noun | `command` | `domain event` |
| help notes/patterns | command/blue/imperative | event/orange/past-tense |

## On-disk item key order (serde_json::Map preserve_order)
`id, type, text, boundedContextId, color, deleted, createdAt, [description]`

## CLI registration (TS lines 138-161)
`fspec add-domain-event-to-foundation <context-name> <event-name> [-d, --description <text>]`

## Marshalled dispatcher JSON
`{ contextName, eventName, description? }`  (camelCase)
