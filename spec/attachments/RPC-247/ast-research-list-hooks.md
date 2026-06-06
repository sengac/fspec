# AST Research — `list-hooks` (RPC-247)

## Source of Truth

- TS Source: `src/commands/list-hooks.ts` (54 LOC)
- Help: `src/commands/list-hooks-help.ts`
- Commander.js registration: `src/cli/program.ts:280` — `registerListHooksCommand(program)`
- Types: `src/hooks/types.ts` — `HookConfig`, `HookDefinition`, etc.

## TypeScript Behaviour (verbatim from `list-hooks.ts`)

```ts
export interface ListHooksOptions {
  cwd?: string;
}

export interface ListHooksResult {
  events: Array<{
    event: string;
    hooks: string[];
  }>;
  message?: string;
}

export async function listHooks(
  options: ListHooksOptions = {}
): Promise<ListHooksResult> {
  const cwd = options.cwd ?? process.cwd();
  const configPath = join(cwd, 'spec', 'fspec-hooks.json');

  try {
    const configContent = await readFile(configPath, 'utf-8');
    const config = JSON.parse(configContent) as HookConfig;

    const events = Object.entries(config.hooks).map(([event, hooks]) => ({
      event,
      hooks: hooks.map(h => h.name),
    }));

    return { events };
  } catch (error: unknown) {
    // Config file doesn't exist
    return {
      events: [],
      message: 'No hooks are configured',
    };
  }
}
```

## Commander.js Registration

```ts
program
  .command('list-hooks')
  .description('List all configured lifecycle hooks')
  .action(async (options: { cwd?: string }) => {
    await listHooks(options);
  });
```

**No `.option(...)` calls — flag-less CLI surface (parity with `list-prefixes`).**

## Critical Behavioural Observations

### 1. Single catch-all error handler

The TS uses a bare `try/catch` block. The comment says "Config file doesn't exist" — BUT the catch swallows **both ENOENT AND any JSON parse error**. Every failure produces:

```json
{ "events": [], "message": "No hooks are configured" }
```

This is wider than `list-prefixes` (which escalates parse errors). The bare `catch (error: unknown)` matches everything — no `if (err.code === 'ENOENT')` guard.

### 2. `config.hooks` is `Record<string, HookDefinition[]>` — preserves insertion order

`Object.entries(config.hooks)` returns events in the order they appear in the JSON file. Insertion-order preservation matters for parity.

### 3. The mapping shape per event

For each event, we emit `{ event: string, hooks: string[] }` — the `hooks` array contains the **`name` field** of each HookDefinition (not the entire definition, not the command).

### 4. The action body discards the return value

The Commander action calls `await listHooks(options)` but **does not render any output**. The TS CLI surface is essentially a no-op stub — nothing is printed to stdout. The data is purely consumed via direct API calls (e.g. by tests like `src/commands/__tests__/hook-commands.test.ts`).

This is unlike `list-prefixes` where the action prints. For the Rust port, the CLI bridge MUST decide what to render. **Decision**: for parity with `list-prefixes`, we should render a human-readable text format AND support `format=json` at the dispatcher.

Confirming TS rendering: the action in `list-hooks.ts:51-53` is:
```ts
.action(async (options: { cwd?: string }) => {
  await listHooks(options);
});
```

**No output at all.** But the help file (`list-hooks-help.ts`) advertises an example output:
```
Configured Hooks:

pre-update-work-unit-status:
  - validate-feature-file
  - check-blockers

post-implementing:
  - run-tests
  - lint-code
```

And the "no hooks" example shows: `No hooks are configured`.

Given the **TS action is a no-op**, the safest Rust port:
- **Dispatcher (LLM tool-call path)**: Returns structured JSON identical to `ListHooksResult` — `{ events: [...], message?: string }`.
- **CLI bridge (shell path)**: Renders a text representation matching the help-file example AND/OR prints raw JSON. Since TS prints nothing, we render text matching the documented help example as the closest "user-intent" parity.

For the message field: TS includes `message: 'No hooks are configured'` ONLY when the catch fires (file missing OR parse error). When the file exists & parses BUT `config.hooks = {}`, the TS returns `{ events: [] }` with **NO message**. This is a subtle behavioural quirk we must preserve.

## Shape of `spec/fspec-hooks.json`

From `src/hooks/types.ts`:
```ts
export interface HookConfig {
  global?: GlobalConfig;
  hooks: Record<string, HookDefinition[]>;
}

export interface HookDefinition {
  name: string;
  command: string;
  blocking?: boolean;
  timeout?: number;
  condition?: HookCondition;
}
```

NOTE: The legacy/example file at `spec/fspec-hooks.json` in this repo uses a DIFFERENT shape — some events contain `[{matcher: "Bash", hooks: [...]}]` rather than `[{name: ..., command: ...}]`. That alternate shape is for **Claude Code session lifecycle hooks**, not the fspec-internal pre/post-command hooks that `list-hooks` enumerates. The TS `listHooks` impl casts `JSON.parse(configContent) as HookConfig` and accesses `h.name` — if an entry is missing `name`, `h.name` is `undefined` and gets coerced into the array. So an entry like `{matcher: ..., hooks: [...]}` would push `undefined` into the names array.

**For the Rust port**: we must decide how to handle this. Safest path: use serde with `HookDefinition` having all optional fields, and emit `null` (or skip) for entries with no `name`. To preserve TS parity exactly (warts and all), we emit `null` for missing names. Better: only emit entries that successfully deserialize as HookDefinition; skip alternate-shape entries silently. Trade-off TBD in rules.

**Decision**: For Phase-1 port, treat the JSON as `Record<String, Vec<serde_json::Value>>` and pluck `.name` from each element as an `Option<String>`. Emit `null` for missing names (parity with TS undefined → null over JSON wire), or stringify as empty string for the text path. We will adopt: **skip entries with missing name in the text output but include `null` in the JSON output** — keeps the JSON shape stable for downstream consumers and the text path readable.

Actually re-reading TS: `hooks.map(h => h.name)` — TS coerces `undefined` → `undefined`. `JSON.stringify` of `undefined` in an array slot becomes `null`. So:
- TS → JSON: missing-name entries surface as `null` in the array.

**Rust port mirrors this**: missing `name` → `null` (i.e. `Option<String>::None`).

## Rendering — Rust port decisions

### Dispatcher (JSON format)

```json
{
  "events": [
    { "event": "pre-implementing", "hooks": ["lint", "test"] },
    { "event": "post-implementing", "hooks": ["notify"] }
  ]
}
```

Or for the empty/error path:
```json
{
  "events": [],
  "message": "No hooks are configured"
}
```

### Dispatcher (text format) and CLI bridge

Empty case:
```
No hooks are configured
```

Populated case (matches `list-hooks-help.ts` example output):
```
Configured Hooks:

pre-update-work-unit-status:
  - validate-feature-file
  - check-blockers

post-implementing:
  - run-tests
  - lint-code
```

## Key Files in Rust Codebase (for the port)

- New impl: `codelet/fspec-core/src/commands/list_hooks.rs` (replace NotYetPorted stub)
- New dispatcher test: `codelet/fspec-core/tests/list_hooks.rs`
- New CLI bridge: `codelet/fspec/src/list_hooks.rs`
- New CLI integration test: `codelet/fspec/tests/cli_list_hooks.rs`
- Shared types: optionally `codelet/fspec-core/src/types/hook.rs` (new) — but for Phase 1 we can stay command-scoped since the shape is simple

## Shared-file changes needed (Phase C reporting)

1. `codelet/fspec-core/src/canonical.rs` — add `"list-hooks"` to `PORTED_COMMANDS` array
2. `codelet/fspec-core/src/dispatch.rs` — add `"list-hooks" => commands::list_hooks::run(args_json, project_root).await` to `run_ported`; remove the existing `"list-hooks" => commands::list_hooks::run(args_json).await` line from `run_stub` (or comment as ported)
3. `codelet/fspec/src/main.rs`:
   - Add `mod list_hooks;`
   - Add `Mode::ListHooks` enum variant with `#[command(name = "list-hooks", about = "List all configured lifecycle hooks")]`
   - Add `Some(Mode::ListHooks) => { let args = list_hooks::CliArgs::default(); ... }` arm to main
   - Add doc-line in `long_about` for `list-hooks`
4. `codelet/fspec/tests/cargo_shape.rs`:
   - Extend the locked file-layout list (`"list_hooks.rs"`) in two places
   - Extend the long-about comment counter

The signature for the new dispatcher arm follows `list-prefixes` exactly — no new shared helpers needed.
