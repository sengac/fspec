# AST Research — remove-hook (RPC-275)

## TypeScript source (canonical reference)
- `src/commands/remove-hook.ts` — 52 LOC. Commander registration + `removeHook(options)`.
- `src/commands/remove-hook-help.ts` — `CommandHelpConfig` exported as default.
- Reuses `HookConfig` from `src/hooks/types.ts`.

## TS contract (parity targets)
- **Reads `spec/fspec-hooks.json` UNCONDITIONALLY** — `readFile` is NOT wrapped
  in try/catch (unlike `add-hook` and `list-hooks`). ENOENT or malformed JSON
  → the promise rejects, the Commander action propagates the error, the CLI
  exits non-zero with the error printed. Rust parity: surface
  `FspecCoreError::Io` / `FspecCoreError::ParseJson` and let the bridge
  render `Error: <…>` to stderr + exit 1.
- If `config.hooks[event]` is truthy → filter out entries where `h.name === name`.
  - **Empty array is RETAINED** (`hooks[event] = []`). The empty key is NOT
    deleted (TS divergence from `remove-dependency` which `delete`s empty arrays).
- If `config.hooks[event]` is undefined/null → no-op (silently succeed).
- Filtering removes ALL matches by name (parity with multi-append from `add-hook`).
- Idempotent: removing a non-existent name is a no-op success
  (filter `h.name !== name` simply yields the same array).
- Atomic write via `fileManager.transaction` (`Object.assign` overwrite).
- Output: silent — Commander action does NOT print anything.

## Args (camelCase JSON for dispatcher)
- `event: string` (positional)
- `name: string` (positional)
- `cwd?: string` (ignored — project_root supplied by Rust caller)

## Differs from add-hook
| Concern | add-hook | remove-hook |
|---|---|---|
| Missing file | swallow → create | **propagate error** (Io) |
| Invalid JSON | swallow → overwrite | **propagate error** (ParseJson) |
| Missing event key | initialise as `[]` then append | silent no-op |
| Empty array result | n/a | retain (do NOT delete key) |

## Preserve-unknown-fields strategy
Same on-disk shape as add_hook. Use the same `HookFile` / `HookEntry` shape
(local to the command, with `#[serde(flatten)] extra`). Field declaration
order on the wire: `name`, `command`, `blocking`, `timeout` — preserved
across round-trips by virtue of typed-field ordering + `extra` flatten.

## Rust call-site survey
- `codelet/fspec-core/src/io/ensure.rs` does NOT have a `read_hooks_or_error`
  helper. We add it INLINE to the command — direct `std::fs::read_to_string`
  + `serde_json::from_str` — because (a) no other command needs the
  "fail-on-missing" variant yet and (b) introducing it now risks racing
  Worker 2's ensure.rs edits. Map ENOENT → `FspecCoreError::Io { command:
  "remove-hook", source }`. Map parse failure → `FspecCoreError::ParseJson
  { file: "fspec-hooks.json", reason }`.
- Final write: `write_json_atomic`.

## Help/CLI surface
- Commander.js: `<event> <name>` (both required, both positional).
- No options at all (no --command, no --blocking, no --timeout, no --format).
- Rich `formatCommandHelp` block in `remove-hook-help.ts`.

## Test coverage (TS reference)
`src/commands/__tests__/hook-commands.test.ts:172-209` covers the happy path
(remove one of two named entries, preserve the other). Rust port extends to
cover:
- missing file → error,
- invalid JSON → ParseJson error,
- missing event key → silent no-op,
- name not in array → silent no-op,
- removing the only entry leaves `[]` (key NOT deleted),
- preserves unknown top-level fields (global, etc.) and per-entry fields
  (condition, command, blocking, timeout) on adjacent entries,
- preserves insertion order of remaining events.

## Rust port artifact list (per command-port.md §1)
1. `codelet/fspec-core/src/commands/remove_hook.rs` — replace stub.
2. `codelet/fspec/src/remove_hook.rs` — CLI bridge (NEW).
3. `codelet/fspec/src/main.rs` — Mode variant + forward! arm (SHARED — supervisor wires).
4. `codelet/fspec-core/src/help/configs/remove_hook.rs` — CONFIG (NEW).
5. `codelet/fspec/tests/fixtures/help/remove-hook.txt` — TS-captured fixture (NEW).
6. `codelet/fspec/tests/cli_remove_hook.rs` — per-scenario integration tests (NEW).
7. `codelet/fspec-core/tests/remove_hook.rs` — dispatcher-level integration (NEW).
