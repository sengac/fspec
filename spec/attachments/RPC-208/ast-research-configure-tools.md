# AST Research — `configure-tools` (RPC-208)

Source of truth: `src/commands/configure-tools.ts` (244 lines). This documents the
actual TS functions, options, persisted config shape, and the exact strings that
the Rust port (`codelet/fspec-core/src/commands/configure_tools.rs`) must
reproduce. Captured for the specifying→testing gate.

## Exported functions (AST)

| Function | Signature | Role |
|----------|-----------|------|
| `checkTestCommand` | `(cwd: string) => Promise<CheckResult>` | Hook/check helper — emits a `system-reminder` about the configured test command (or its absence). NOT the port target. |
| `checkQualityCommands` | `(cwd: string) => Promise<CheckResult>` | Hook/check helper — emits a `system-reminder` about configured quality-check commands (or absence). NOT the port target. |
| `configureTools` | `(options: ConfigureToolsOptions) => Promise<CheckResult \| void>` | **The port target.** Writes the tools config (or short-circuits on `reconfigure`). |
| `registerConfigureToolsCommand` | `(program: Command) => Promise<void>` | Commander registration / CLI bridge. |

> Scope note: RPC-208 ports `configureTools` (write path) + the CLI registration.
> `checkTestCommand` / `checkQualityCommands` are separate hook helpers and are
> out of scope for this card.

## Options / flags (`ConfigureToolsOptions` + Commander)

```ts
interface ConfigureToolsOptions {
  testCommand?: string;        // --test-command <command>
  qualityCommands?: string[];  // --quality-commands <commands...>  (multi-value)
  reconfigure?: boolean;       // --reconfigure (flag)
  cwd: string;                 // injected (process.cwd())
}
```

Commander definitions (`registerConfigureToolsCommand`):
- `.command('configure-tools')`
- `.description('Configure test and quality check commands for platform-agnostic workflow')`
- `--test-command <command>` — "Test command to run (e.g., \"npm test\", \"pytest\", \"cargo test\")"
- `--quality-commands <commands...>` — "Quality check commands to run (e.g., \"eslint .\" \"prettier --check .\")"
- `--reconfigure` — "Re-detect tools and update configuration"

On `.action`: calls `configureTools(...)`; then `if (!options.reconfigure)` prints
`✓ Tool configuration saved to spec/fspec-config.json` via `output.log`.

## Persisted config shape

Config path: `<cwd>/spec/fspec-config.json`.

```ts
interface ToolsConfig {
  test?: { command: string };
  qualityCheck?: { commands: string[] };
}
interface ConfigFile {
  agent?: string;     // defaults to 'claude' when file absent
  tools?: ToolsConfig;
}
```

Serialized with `JSON.stringify(config, null, 2)` (2-space indent, **no trailing
newline**), preserving field order `agent` then `tools`.

## `configureTools` control flow (line-by-line)

1. Destructure `{ testCommand, qualityCommands, reconfigure, cwd }`.
2. `configPath = join(cwd, 'spec', 'fspec-config.json')`; `specDir = join(cwd, 'spec')`.
3. If `specDir` missing → `mkdirSync(specDir, { recursive: true })` (spec/ created before any write).
4. **`reconfigure` short-circuit** (lines 143-157): return a `CheckResult`
   `{ type: 'system-reminder', message: formatAgentOutput(cwd, RECONFIGURE_MESSAGE) }`
   **WITHOUT writing the config and WITHOUT regenerating templates.**
5. `config = { agent: 'claude' }` default.
6. If `configPath` exists → `config = JSON.parse(readFileSync(...))` (read-modify-write; preserves `agent` + unknown fields).
7. If `!config.tools` → `config.tools = {}`.
8. If `testCommand` → `config.tools.test = { command: testCommand }` (applied only when provided).
9. If `qualityCommands` → `config.tools.qualityCheck = { commands: qualityCommands }` (applied only when provided).
10. `writeFileSync(configPath, JSON.stringify(config, null, 2), 'utf-8')`.
11. CONFIG-003 side effect (lines 179-190): if `config.agent`, dynamically import
    `getAgentById` + `installAgentFiles` and silently regenerate agent templates.
    Returns `void` on this path.

## RECONFIGURE_MESSAGE (exact literal, lines 148-154)

```
RECONFIGURE TOOLS

Use Read/Glob tools to detect test frameworks and quality check tools, then run:

  fspec configure-tools --test-command <cmd>
  fspec configure-tools --quality-commands '<tool1>' '<tool2>' '<tool3>'
```

## Divergences captured for the port

- **D3 (deferred):** the CONFIG-003 silent template regeneration (`installAgentFiles`
  via `init`) is NOT ported — `init`/`installAgentFiles` is not yet ported. Config
  write parity is preserved; only the template side effect is deferred.
- **D4 (bug-for-bug):** the reconfigure branch calls `formatAgentOutput(cwd, ...)`
  passing the `cwd` **string** where an `AgentConfig` is expected. As a result the
  message is **NOT** wrapped in `<system-reminder>` tags — it falls through to the
  plain prefixed-text branch. The Rust port reproduces this exactly (reconfigure
  output is NOT wrapped), with a `// TODO(parity-bug RPC-208-D4)` marker at the site.

## Two-front-doors

CLI bridge (`codelet/fspec/src/configure_tools.rs`) marshals JSON
`{testCommand?, qualityCommands?, reconfigure?}` (omitting `None`) only; both the
dispatcher and the standalone binary converge on
`commands::configure_tools::run`. No domain logic in the bridge.
