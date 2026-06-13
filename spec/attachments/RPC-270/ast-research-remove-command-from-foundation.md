# AST Research — remove-command-from-foundation (RPC-270)

Rust parity port of `src/commands/remove-command-from-foundation.ts`. Target file:
`codelet/fspec-core/src/commands/remove_command_from_foundation.rs` (currently a `NotYetPorted` stub).

## 1. TS source anatomy (`src/commands/remove-command-from-foundation.ts`)

`AstGrep typescript 'export async function $NAME($$$ARGS): Promise<$RET> { $$$BODY }'`:
- `removeCommandFromFoundation(contextName, commandName, options)` — core (line 26)
- `removeCommandFromFoundationCommand(...)` — CLI wrapper (line 100)
- `registerRemoveCommandFromFoundationCommand(program)` — Commander.js registration (line 131)

`AstGrep 'data.eventStorm.items.find($$$ARGS)'` → TWO finds at lines 57 and 69 (context, then command).

Core behaviour (lines 31–94), soft-delete semantics. Validation ORDER matters:
1. `cwd`, `foundationPath = ${cwd}/spec/foundation.json`; `fileManager.readJSON(... generic default)`.
2. `fileManager.transaction(foundationPath, data => {...})`:
   - **Guard 1** — if `!data.eventStorm` →
     `throw new Error("Bounded context '<contextName>' not found (no Event Storm data)")`.
   - **Guard 2** — find context: `items.find(i => i.type==='bounded_context' && i.text===contextName
     && !i.deleted)`. Missing → `throw new Error("Bounded context '<contextName>' not found")`.
   - **Guard 3** — find command: `items.find(i => i.type==='command' && i.text===commandName
     && !i.deleted && 'boundedContextId' in i && i.boundedContextId === boundedContext.id)`.
     Missing → `throw new Error("Command '<commandName>' not found in bounded context '<contextName>'")`.
   - On match: `foundCommand.deleted = true` (soft-delete; NO array splice).
3. `generateFoundationMdCommand({ cwd })` — **SKIPPED in Rust** (add_diagram RPC-178 precedent).
4. Returns `{ success: true, message: 'Removed command "<commandName>" from "<contextName>" bounded context' }`.

CLI wrapper (100–126): success `output.log('✓', message)` exit 0; error
`output.error(chalk.red('Error:'), message)` exit 1.

Commander surface (131–154): `remove-command-from-foundation <context-name> <command-name>` (no flags).

### Non-idempotency note
Because Guard 3 requires `!deleted`, removing an already soft-deleted command is treated as
not-found → second call fails. This is intentional parity behaviour (captured in scenario).

## 2. Reference template (`add_bounded_context.rs`, RPC-172) + sibling `add_diagram.rs`

- add_bounded_context demonstrates the `serde_json::Value` round-trip + `serde_json::Map` preserve-order
  pattern, but it APPENDS to work-units.json. We MUTATE an existing item's `deleted` field in
  foundation.json (no append, no nextItemId change).
- add_diagram (RPC-178) is the closest foundation-mutation template: `ensure_foundation_file` +
  `as_object_mut` navigation + `write_json_atomic`. Reuse its error-handling shape (return structured
  `ParseJson`/`InvalidArgs` instead of panics to satisfy clippy).

## 3. IO helpers (verified)

- `io::ensure::ensure_foundation_file(cwd) -> Result<serde_json::Value, FspecCoreError>` (`ensure.rs:86`).
- `io::locked_file::write_json_atomic` (`locked_file.rs:96`) — pretty 2-space, NO trailing newline.
  CORRECT for FileManager eventStorm commands (supervisor confirmed). Only write on successful match.
- No `iso8601_now` needed (remove does not create a timestamp).

## 4. Planned Rust shape

```
#[derive(Deserialize)] #[serde(rename_all="camelCase")]
struct Args { context_name: String, command_name: String }

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```
1. Parse args.
2. `let mut data = ensure_foundation_file(project_root)?;`
3. Guard 1: `data.get("eventStorm")` absent/non-object →
   `InvalidArgs { reason: "Bounded context '{ctx}' not found (no Event Storm data)" }` (NO write).
4. Borrow `items` array. Guard 2: find index of `type=="bounded_context" && text==ctx && deleted!=true`
   → capture its `id`. Missing → `"Bounded context '{ctx}' not found"` (NO write).
5. Guard 3: find index of `type=="command" && text==cmd && deleted!=true &&
   boundedContextId==captured_id`. Missing →
   `"Command '{cmd}' not found in bounded context '{ctx}'"` (NO write).
6. Set that item's `deleted = true`. `write_json_atomic(spec/foundation.json, data)`.
7. Return `{ success: true, message: 'Removed command "{cmd}" from "{ctx}" bounded context' }`.

Borrow-checker note: capture `id` as an owned `serde_json::Value`/u64 first (immutable scan), then do
a second mutable pass to set `deleted=true` — avoids overlapping borrows of `items`.

## 5. Two-front-doors / dispatch (shared-file work — supervisor)

- Dispatcher: `remove-command-from-foundation` currently in `run_stub` calling `run(args_json)`. Must
  move to `run_ported` calling `run(args_json, project_root)`; add to `is_ported` + canonical list.
- CLI bridge `codelet/fspec/src/remove_command_from_foundation.rs` — JSON marshalling only
  `{contextName, commandName}`; NO domain logic.
- Help config + clap subcommand in main.rs; help fixture
  `codelet/fspec/tests/fixtures/help/remove-command-from-foundation.txt`.
