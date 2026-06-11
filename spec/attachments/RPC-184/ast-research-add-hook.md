# AST Research — add-hook (RPC-184)

## TypeScript source (canonical reference)
- `src/commands/add-hook.ts` — 96 LOC. Commander registration + `addHook(options)`.
- `src/commands/add-hook-help.ts` — `CommandHelpConfig` exported as default.
- `src/hooks/types.ts` — `HookConfig`, `HookDefinition`, `HookCondition`, `GlobalConfig`.

## TS contract (parity targets)
- Reads `spec/fspec-hooks.json` if present.
- On read/parse failure the **TS catches BOTH errors silently** and starts with `config = { hooks: {} }`.
  Important: even an *invalid* JSON file is OVERWRITTEN with the new config (existing hooks lost).
- Initialises `config.hooks[event]` as `[]` if missing.
- Appends a new entry: `{ name, command, blocking, timeout }`.
  - `blocking` is always present (Commander default `false`).
  - `timeout` is `undefined` if not supplied. `JSON.stringify` omits `undefined` →
    on-disk the field is absent.
- `mkdir(join(cwd, 'spec'), { recursive: true })` — auto-creates `spec/` dir.
- Final atomic write via `fileManager.transaction` (Object.assign overwrite).
- Output: silent — the Commander action does NOT print anything on success.
- TS does NOT enforce unique name (parity: same name may be appended twice).
- TS does NOT validate event name or script existence.

## Args (camelCase JSON for dispatcher)
- `event: string` (positional)
- `name: string` (positional)
- `command: string` (required option `--command <path>`)
- `blocking: boolean` (default `false`)
- `timeout?: number` (Commander parses `parseInt(value, 10)`)
- `cwd?: string` (ignored — project_root is supplied by Rust caller)

## Rust call-site survey
- `codelet/fspec-core/src/commands/list_hooks.rs` already models a partial
  `HookFile` / `HookEntry` (read-only, swallow-all). Since list-hooks is the
  ONLY other reader and it is intentionally narrow (just `name`), introducing
  a new shared `types/hooks.rs` for the full `HookConfig` shape is NOT required
  yet. We will keep `add_hook`'s on-disk shape **local** as `HookFile` /
  `HookEntry` with `#[serde(flatten)] extra` to preserve unknown fields
  (`global`, `condition`, etc.). Field declaration order on the wire:
  `name`, `command`, `blocking`, then optional `timeout` (matches TS literal).
- `codelet/fspec-core/src/io/locked_file.rs::{read_or_init_json, write_json_atomic}`
  give us atomic load-or-init + atomic write semantics matching the TS
  `fileManager.transaction` round-trip.

## Divergence vs `list_hooks`
| Concern | list_hooks (read) | add_hook (write) |
|---|---|---|
| Missing file | swallow → `{events:[], message}` | create new `{hooks:{}}` then write |
| Invalid JSON | swallow → empty payload | **overwrite** with new `{hooks:{event:[{...}]}}` |
| `spec/` missing | leave untouched | `mkdir spec` then write |
| Output | renders | silent (zero stdout on CLI path) |
| Atomic write | none | `write_json_atomic` |

## Preserve-unknown-fields strategy
On-disk JSON may contain `global: {...}`, top-level keys, and per-event hook
entries with `condition` / `blocking` / `timeout`. We use
`#[serde(flatten)] pub extra: serde_json::Map<String, Value>` on both
`HookFile` and `HookEntry` so a load → modify → save cycle does not drop
adjacent fields. `IndexMap<String, Vec<HookEntry>>` for `hooks` preserves
event key insertion order (matching JS object-literal semantics).

## Help/CLI surface
- Commander.js: `<event> <name>`, `--command <path>` (required),
  `--blocking` (boolean flag, default false), `--timeout <seconds>` (parseInt).
- No --format flag, no --workspace flag, no --help text customisation
  beyond the rich `formatCommandHelp` block in `add-hook-help.ts`.

## Test coverage (TS reference)
`src/commands/__tests__/hook-commands.test.ts:138-169` covers the happy path
(append to empty config, single entry, blocking=true). We extend coverage in
the Rust port to cover:
- create-from-missing (no file, no spec dir),
- swallow-on-invalid (overwrite path),
- preserve global config / unknown fields,
- timeout omitted vs supplied,
- multiple appends to same event preserve order,
- multiple events preserve insertion order.

## Rust port artifact list (per command-port.md §1)
1. `codelet/fspec-core/src/commands/add_hook.rs` — replace stub.
2. `codelet/fspec/src/add_hook.rs` — CLI bridge (NEW).
3. `codelet/fspec/src/main.rs` — Mode variant + forward! arm (SHARED — supervisor wires).
4. `codelet/fspec-core/src/help/configs/add_hook.rs` — CONFIG (NEW).
5. `codelet/fspec/tests/fixtures/help/add-hook.txt` — TS-captured fixture (NEW).
6. `codelet/fspec/tests/cli_add_hook.rs` — per-scenario integration tests (NEW).
7. `codelet/fspec-core/tests/add_hook.rs` — dispatcher-level integration (NEW).
