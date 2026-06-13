# AST Research — add-command-to-foundation (RPC-175)

Rust parity port of `src/commands/add-command-to-foundation.ts`. Target file:
`codelet/fspec-core/src/commands/add_command_to_foundation.rs` (currently a `NotYetPorted` stub).

## 1. TS source anatomy (`src/commands/add-command-to-foundation.ts`)

`AstGrep typescript 'export async function $NAME($$$ARGS): Promise<$RET> { $$$BODY }'`:
- `addCommandToFoundation(contextName, commandName, options)` — core logic (line 23)
- `addCommandToFoundationCommand(contextName, commandName, options)` — CLI wrapper (line 107)
- `registerAddCommandToFoundationCommand(program)` — Commander.js registration (line 138)

Core behaviour (lines 28–101):
1. `cwd = options.cwd ?? process.cwd()`, `foundationPath = ${cwd}/spec/foundation.json`.
2. `fileManager.readJSON(foundationPath, <generic v2.0.0 default>)` — validates the file is loadable
   (auto-default supplies a fallback shape).
3. `fileManager.transaction(foundationPath, data => {...})` — atomic read-modify-write:
   - Seed `data.eventStorm = { level: 'big_picture', items: [], nextItemId: 1 }` IF absent.
   - Find bounded context: `items.find(i => i.type === 'bounded_context' && i.text === contextName)`.
     **NOTE: NO `!deleted` filter on add (diverges from remove).**
   - If not found → `throw new Error("Bounded context '<contextName>' not found")`.
   - Build the command item (object-literal insertion order):
     `{ id: nextItemId, type: 'command', text: commandName, boundedContextId: bc.id,
        color: 'blue', deleted: false, createdAt: new Date().toISOString(),
        ...(description && { description }) }`.
   - `items.push(command); nextItemId++` (post-increment).
4. `generateFoundationMdCommand({ cwd })` — **SKIPPED in Rust** (add_diagram RPC-178 precedent).
5. Returns `{ success: true, message: 'Added command "<commandName>" to "<contextName>" bounded context' }`.

CLI wrapper (107–133): on success `output.log('✓', message)` + `process.exit(0)`; on thrown error
`output.error(chalk.red('Error:'), message)` + `process.exit(1)`.

Commander surface (138–156): `add-command-to-foundation <context-name> <command-name>
[-d, --description <text>]`.

## 2. Reference template (`add_bounded_context.rs`, RPC-172)

`AstGrep` shows `run(args_json, project_root)` + private `append_event_storm_item(...)`. KEY
DIFFERENCES for THIS port:
- add_bounded_context targets **work-units.json** with an `existsSync` guard (no auto-create) and
  seeds `nextItemId=0` / `level=process_modeling`. **We target foundation.json**, auto-create via
  `ensure_foundation_file`, and seed `nextItemId=1` / `level=big_picture`.
- bounded_context item `color` = JSON **null**. **Our command item `color` = JSON string `"blue"`.**
- It validates work-unit status (done/blocked). **We validate bounded-context existence by name.**
- Item-body built with `serde_json::Map` (workspace `serde_json` has `preserve_order`) to reproduce
  TS insertion order — reuse this technique exactly.

## 3. IO helpers (verified via AstGrep / Grep)

- `io::ensure::ensure_foundation_file(cwd: &Path) -> Result<serde_json::Value, FspecCoreError>`
  (`ensure.rs:86`) — load-or-init generic schema v2.0.0 default. Use this (NOT the work-units path).
- `io::locked_file::write_json_atomic<T: Serialize>(path, value)` (`locked_file.rs:96`) — pretty
  2-space, **NO trailing newline**. CORRECT for FileManager-backed eventStorm commands (supervisor
  confirmed: TS uses `JSON.stringify(...,2)` with no `'\n'`). Do NOT use
  `write_json_atomic_trailing_newline` (that's for add-capability/persona which append `'\n'`).
- `io::time::iso8601_now() -> String` (`time.rs:34`) — 24-char `...Z` timestamp for `createdAt`.

## 4. Planned Rust shape

```
#[derive(Deserialize)] #[serde(rename_all="camelCase")]
struct Args { context_name: String, command_name: String, #[serde(default)] description: Option<String> }

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```
1. Parse args (InvalidArgs on failure).
2. `let mut data = ensure_foundation_file(project_root)?;`
3. Navigate/seed `data["eventStorm"]` as object {level:"big_picture", items:[], nextItemId:1} if absent
   or non-object.
4. Scan `items` for `type=="bounded_context" && text==context_name` → capture `id` (as u64/Value).
   Missing → `InvalidArgs { reason: "Bounded context '{context_name}' not found" }` (NO write).
5. Read `nextItemId` (default 1). Build item Map in order: id, type, text, boundedContextId, color
   (String "blue"), deleted (false), createdAt (iso8601_now), [description]. Push, set nextItemId+1.
6. `write_json_atomic(&project_root.join("spec/foundation.json"), &data)?`.
7. Return JSON `{ success: true, message }`.

## 5. Two-front-doors / dispatch (shared-file work — supervisor)

- Dispatcher: `add-command-to-foundation` currently in `run_stub` (dispatch.rs:439) calling
  `run(args_json)`. Must move to `run_ported` calling `run(args_json, project_root)`; add to
  `is_ported` predicate + canonical ported list.
- CLI bridge `codelet/fspec/src/add_command_to_foundation.rs` — JSON marshalling only
  `{contextName, commandName, description?}`; NO domain logic.
- Help config + clap subcommand in main.rs; help fixture
  `codelet/fspec/tests/fixtures/help/add-command-to-foundation.txt`.
