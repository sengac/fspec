# AST Research — remove-init-files (RPC-276)

## TS source of truth
- `src/commands/remove-init-files.ts` (202 LOC)
- `src/commands/remove-init-files-help.ts`
- `src/utils/agentRegistry.ts` (AGENT_REGISTRY — 20 agents) + `getAgentById`
- `src/utils/agentDetection.ts` (detectAgents)

## Exported core
```ts
export async function removeInitFiles(cwd, options:{keepConfig}): Promise<string[]>
```
plus `executeRemoveInitFiles({keepConfig?, promptKeepConfig?})` (interactive wrapper).

## Algorithm (removeInitFiles)
1. `detectInstalledAgent(cwd)`:
   - read `spec/fspec-config.json` if exists; if parses & has `.agent` → return it.
   - else `detectAgents(cwd)` → first agent whose any `detectionPaths` entry exists in cwd; return its id or null.
2. If no agent → `throw new Error('No fspec agent installation detected. Nothing to remove.')`.
3. `agent = getAgentById(id)`; if unknown → `throw Error(`Unknown agent: ${id}`)`.
4. `removeAgentFiles(cwd, agent.id)`:
   - `rm(join(cwd,'spec',agent.docTemplate), {force:true})` → push `spec/${docTemplate}` (e.g. `spec/CLAUDE.md`).
   - filename = `agent.slashCommandFormat==='toml' ? 'fspec.toml' : 'fspec.md'`.
   - `rm(join(cwd, agent.slashCommandPath, filename), {force:true})` → push `${slashCommandPath}${filename}`
     (e.g. `.claude/commands/fspec.md`).
   - `{force:true}` = idempotent; missing files do NOT error.
5. If `!options.keepConfig`: `rm(join(cwd,'spec','fspec-config.json'), {force:true})` → push `spec/fspec-config.json`.
6. return `filesRemoved` (array of relative path strings, in push order).

## CLI action (registerRemoveInitFilesCommand)
- options: `--keep-config` / `--no-keep-config` → commander yields `options.keepConfig` (true|false|undefined).
- `executeRemoveInitFiles({ keepConfig })`:
  - if keepConfig !== undefined → use it.
  - else if promptKeepConfig (test) → call it.
  - else → interactive Ink ConfirmPrompt (NOT portable to Rust headless — see ASK).
- Success: `output.log('✓ Successfully removed fspec init files')` then per file `  - ${file}`; `process.exit(0)`.
- Error: `output.error(chalk.red('✗ Failed to remove init files:'), error.message); process.exit(1)`.

## Rust port plan
- Need the agent registry data in Rust. **ASK SUPERVISOR**: is there an existing
  Rust port of AGENT_REGISTRY (from `init` command, RPC for init)? Check
  `codelet/fspec-core/src/commands/init.rs`. If yes, reuse it. If no, I will create a
  NEW non-shared module `codelet/fspec-core/src/agents.rs` OR a local const table inside
  `remove_init_files.rs` (preferred to avoid touching shared mod.rs). Will inline the
  needed subset (id, docTemplate, slashCommandPath, slashCommandFormat, detectionPaths).
- `Args { keep_config: Option<bool> }` (camelCase `keepConfig`). The interactive prompt
  path is NOT reproducible in headless Rust; when `keep_config` is None we must pick a
  default. **ASK SUPERVISOR** for desired default (likely keepConfig=false to match the
  destructive default, or require the flag). Proposed: when None, treat as `false`
  (remove config) — matching `--no-keep-config` semantics — OR error requiring explicit
  flag. Will document chosen behaviour in feature file.
- File deletion: use std::fs::remove_file with ENOENT tolerated (force:true parity).
  Mirror a mutation command's IO — `rm -f` equivalent = `match remove_file(p) { Err(e) if e.kind()==NotFound => Ok, ... }`.
- detectInstalledAgent: read spec/fspec-config.json (serde_json::Value), check `.agent`;
  else scan detectionPaths with `path.exists()`.
- Result envelope `{ filesRemoved: Vec<String> }`.
- CLI bridge renders success lines + exit 0, error → exit 1.

## Shared modules reused
- `crate::error::FspecCoreError`
- std::fs for deletion (no shared write helper needed — these are unconditional rm -f).
- Possibly init.rs agent table (ASK).
