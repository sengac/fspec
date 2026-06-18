# AST Research — RPC-286 `research` command port (LIST-only scope)

Performed with the AstGrep tool during discovery. Confirms the TS surface to
port and the Rust shared infrastructure to reuse.

## TS source surface (what to port)

### `src/utils/config-resolution.ts` — resolveConfig
```
src/utils/config-resolution.ts:74:1:export function resolveConfig(
```
- Signature: `resolveConfig(toolName, options: ConfigOptions = {}): ResolvedConfig`.
- Precedence (lowest→highest, later `Object.assign` wins):
  1. DEFAULTS[toolName]               (e.g. perplexity.model = "sonar")
  2. PROJECT config  `${cwd}/spec/fspec-config.json` → `.research[toolName]`
  3. USER config     `~/.fspec/fspec-config.json`     → `.research[toolName]`
  4. ENV vars        via ENV_VAR_MAPPINGS[toolName]   (sets `config.source = 'ENV'`)
- Returns a flat object; `source` field tracks the winning layer.
- **All blocking `fs.existsSync` / `fs.readFileSync` — no async, no spawn.** Safe to port
  with `std::fs` under `poll_sync_future`.

### `src/commands/research-tool-list.ts` — listResearchTools + TOOL_REGISTRY
- `listResearchTools(cwd?, showAll=false, userConfigPath?)` is a **synchronous** function
  (AstGrep `export function listResearchTools(...)` did not match the multi-line generic
  arrow shape but the symbol is a plain sync `export function`; confirmed by reading file).
- `TOOL_REGISTRY` static table — required fields per tool:
  - `ast`: `[]`            → always CONFIGURED ('✓')
  - `perplexity`: `[apiKey]`
  - `jira`: `[url, token]`
  - `confluence`: `[url, token]`
  - `stakeholder`: `[teamsWebhook]`
- A tool is CONFIGURED iff `required.every(f => config[f] is non-empty)`.
- ENV_VAR_MAPPINGS provide setup guidance strings for unconfigured tools.

### `src/commands/research.ts` — command entry + registration
- LIST mode (no `--tool`): prints `Available Research Tools:` header (CLI path) and returns
  `{tools, executed:false, discoveryMethod:'registry'}`.
- EXECUTE mode (`--tool`): `discoverResearchTools` (spec/research-scripts/*) → `executeResearchTool`
  via `child_process.spawn`; bundled tools via `getResearchTool` (network/NAPI/dynamic import).
  **OUT OF SCOPE** — not portable under single-poll dispatch.
- Pre-execution validation `if (!tool) throw 'Research tool not found: <name>'` → IN SCOPE.

### `src/commands/research-help.ts` — help config (byte-parity quirks)
- Config uses field names (`name`, `title`, `cause`/`solution`, `typicalWorkflow: string[]`)
  that the formatter does NOT read (`flag`, `pattern`, `fix`, single `typicalWorkflow`),
  so the captured TS `--help` emits literal `undefined` for option flags / pattern titles /
  error fixes and comma-joins the workflow array. The Rust CONFIG reproduces these verbatim.

## Rust shared infrastructure (what to reuse)

### `codelet/fspec-core/src/help/mod.rs` — formatter (already ported)
```
codelet/fspec-core/src/help/mod.rs:88:1:pub fn format_command_help(config: &CommandHelpConfig) -> String {
```
- Byte-faithful port of TS `formatCommandHelp`. My `help/configs/research.rs` CONFIG, when
  passed to this function, reproduces `tests/fixtures/help/research.txt` exactly (verified by
  an offline reconstruction diff: formatted string + console.log "\n" == fixture).

### Dispatcher / canonical / bridge
- `dispatch_command(DispatchRequest{command,args_json,project_root}) -> DispatchResult{success,data,error,..}`.
- Core `run` returning `Err(FspecCoreError)` surfaces as `success=false`, `error=Some(Display)`.
  `InvalidArgs{command:"research", reason:"Research tool not found: does-not-exist"}` Display
  = `Invalid args for fspec command research: Research tool not found: does-not-exist` → contains
  the asserted substring.

## Async/process verdict (poll_sync_future safety)
- LIST mode + pre-spawn validation = blocking `std::fs` + static tables only → SAFE.
- EXECUTE mode (network https / async NAPI ast / dynamic JS import / child_process.spawn await)
  = genuinely async/un-portable → DEFERRED per supervisor scope decision (Option 1, LIST-only).
