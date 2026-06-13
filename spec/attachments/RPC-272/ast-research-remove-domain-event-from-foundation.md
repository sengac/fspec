# AST Research — remove-domain-event-from-foundation (RPC-272)

## TS source of truth
- `src/commands/remove-domain-event-from-foundation.ts`
- `src/commands/remove-domain-event-from-foundation-help.ts`

## Twin (already ported, COPY SHAPE)
- `codelet/fspec-core/src/commands/remove_command_from_foundation.rs` (RPC-270)
- `codelet/fspec/src/remove_command_from_foundation.rs`
- `codelet/fspec-core/src/help/configs/remove_command_from_foundation.rs`
- tests: `codelet/fspec-core/tests/remove_command_from_foundation.rs`, `codelet/fspec/tests/cli_remove_command_from_foundation.rs`

## Behaviour (parity with TS, lines from remove-domain-event-from-foundation.ts)
1. cwd default; foundationPath = `${cwd}/spec/foundation.json`.
2. readJSON with INLINE minimal default (same as twin).
3. transaction:
   - NO eventStorm field → throw
     `Bounded context '<contextName>' not found (no Event Storm data)` (line 50-54), NO write
   - find bounded context: `type==='bounded_context' && text===contextName && !deleted` (line 57-62)
     missing → throw `Bounded context '<contextName>' not found` (line 65)
   - find domain event: `type==='event' && text===eventName && !deleted &&
     'boundedContextId' in item && item.boundedContextId === ctx.id` (line 69-77)
     missing → throw
     `Domain event '<eventName>' not found in bounded context '<contextName>'` (line 80-82)
   - set `domainEvent.deleted = true` (soft-delete, line 85)
4. Auto-regenerate FOUNDATION.md (line 89) — DIVERGENCE: skipped in Rust per add_diagram (RPC-178) precedent.
5. result `{ success:true, message:'Removed domain event "<eventName>" from "<contextName>" bounded context' }`

An already soft-deleted event is treated as not-found (non-idempotent on 2nd call).

## KEY DIFFERENCES vs remove_command_from_foundation twin
| aspect | command twin | THIS (domain event) |
|--------|-------------|---------------------|
| item `type` matched | `"command"` | `"event"` |
| 2nd positional arg | `commandName` / `<command-name>` | `eventName` / `<event-name>` |
| not-found error noun | `Command` | `Domain event` |
| message verb noun | `command` | `domain event` |

## CLI registration (TS lines 131-153)
`fspec remove-domain-event-from-foundation <context-name> <event-name>`  (NO options)

## Marshalled dispatcher JSON
`{ contextName, eventName }`  (camelCase)
