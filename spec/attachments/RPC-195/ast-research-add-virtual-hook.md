# AST Research — `add-virtual-hook` (RPC-195)

Authoritative TS sources (read once, ported once):

| File | Lines | Role |
|------|-------|------|
| `src/commands/add-virtual-hook.ts` | 137 LOC | Core impl + Commander registration |
| `src/commands/add-virtual-hook-help.ts` | 131 LOC | Help config |
| `src/hooks/script-generation.ts` | 123 LOC | `generateVirtualHookScript` helper |
| `src/types/index.ts:36-42` | — | `VirtualHook` interface |
| `src/types/index.ts:169` | — | `workUnit.virtualHooks?: VirtualHook[]` |

## TS contract (verbatim)

```ts
interface AddVirtualHookOptions {
  workUnitId: string;
  event: string;
  command: string;
  blocking?: boolean;   // defaults false
  gitContext?: boolean; // defaults false
  cwd?: string;
}

interface AddVirtualHookResult {
  success: boolean;
  hookCount: number;
}

interface VirtualHook {
  name: string;
  event: string;
  command: string;
  blocking: boolean;
  gitContext?: boolean; // serialized only when true
}
```

## Algorithm (single source of truth — `addVirtualHook` lines 24-93)

1. `cwd = options.cwd || process.cwd()`.
2. `workUnitsFile = join(cwd, 'spec/work-units.json')`.
3. Load via `ensureWorkUnitsFile(cwd)` — auto-creates empty store on ENOENT.
4. Existence check: `if (!data.workUnits[id])` → throw `Work unit '<id>' does not exist`.
5. Initialize `workUnit.virtualHooks = []` when missing.
6. Hook name derivation:
   ```ts
   const hookName = options.command.split(' ')[0].split('/').pop() || 'hook';
   ```
   Examples:
   - `"eslint src/"` → `"eslint"`
   - `"npm run lint"` → `"npm"`
   - `"./bin/check.sh"` → `"check.sh"`
   - `"/usr/bin/node"` → `"node"`
   - empty string → `"hook"`
7. `commandToStore = options.command`.
8. If `gitContext === true`:
   - Call `generateVirtualHookScript({ workUnitId, hookName, command, gitContext:true, projectRoot:cwd })`.
   - The helper writes `spec/hooks/.virtual/<workUnitId>-<hookName>.sh` (mkdir -p, mode 0o755).
   - `commandToStore = scriptPath.replace(cwd + '/', '')` — relative path.
9. Build hook object: `{ name, event, command: commandToStore, blocking: blocking ?? false }`.
10. If `gitContext === true` → append `gitContext: true` (skipped otherwise — TS soft optional).
11. `workUnit.virtualHooks.push(hook)`.
12. `workUnit.updatedAt = new Date().toISOString()`.
13. Atomic write via `fileManager.transaction(workUnitsFile, …)`.
14. Return `{ success: true, hookCount: workUnit.virtualHooks.length }`.

## Script template (from `script-generation.ts` lines 64-96)

### With `gitContext=true`
```bash
#!/bin/bash
set -e

# Read context JSON from stdin
CONTEXT=$(cat)

# Extract staged and unstaged files from context
STAGED_FILES=$(echo "$CONTEXT" | jq -r '.stagedFiles[]? // empty' 2>/dev/null | tr '\n' ' ')
UNSTAGED_FILES=$(echo "$CONTEXT" | jq -r '.unstagedFiles[]? // empty' 2>/dev/null | tr '\n' ' ')

# Combine all changed files
ALL_FILES="$STAGED_FILES $UNSTAGED_FILES"

# Exit if no files to process
if [ -z "$ALL_FILES" ]; then
  echo "No changed files to process"
  exit 0
fi

# Run command with changed files
<command> $ALL_FILES
```

### Without `gitContext` (NEVER reached from `add-virtual-hook` — only the
`gitContext=true` branch generates a script; the simple branch is dead code
on this path).

## CLI surface (Commander.js)

```
fspec add-virtual-hook <workUnitId> <event> <command>
  [--blocking]      default false
  [--git-context]   default false
```

Action handler success output:
```
✓ Virtual hook added to <workUnitId>
  Total virtual hooks: <hookCount>
```

Error output (stderr) and `process.exit(1)`:
```
✗ Failed to add virtual hook: <error.message>
```

## Rust port plan

| Artifact | Path | Notes |
|----------|------|-------|
| Core impl | `codelet/fspec-core/src/commands/add_virtual_hook.rs` | `run(args_json, project_root)` |
| Help config | `codelet/fspec-core/src/help/configs/add_virtual_hook.rs` | byte-exact w/ TS help |
| CLI bridge | `codelet/fspec/src/add_virtual_hook.rs` | `pub struct CliArgs`, `pub async fn run` |
| Dispatcher test | `codelet/fspec-core/tests/add_virtual_hook.rs` | @step-tagged |
| CLI test | `codelet/fspec/tests/cli_add_virtual_hook.rs` | @step-tagged |
| Help fixture | `codelet/fspec/tests/fixtures/help/add-virtual-hook.txt` | from `node dist/index.js add-virtual-hook --help` |

### Args (camelCase to mirror dispatcher contract)
```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddVirtualHookArgs {
    work_unit_id: String,
    event: String,
    command: String,
    #[serde(default)]
    blocking: bool,
    #[serde(default)]
    git_context: bool,
}
```

### Result
```rust
#[derive(Debug, Serialize)]
struct AddVirtualHookResult { success: bool, hook_count: usize }
```
Serialized as `{"success":true,"hookCount":<n>}` (camelCase).

### Storage shape — `virtualHooks` lives in `workUnit.extra`
Mirrors the pattern documented in `list_virtual_hooks::run` (RPC-252):
```rust
let arr = wu.extra.entry("virtualHooks".to_string())
    .or_insert_with(|| Value::Array(Vec::new()));
```

### Hook-name derivation in Rust
Port of `options.command.split(' ')[0].split('/').pop() || 'hook'`:
```rust
fn derive_hook_name(command: &str) -> String {
    let first_token = command.split(' ').next().unwrap_or("");
    let last_segment = first_token.rsplit('/').next().unwrap_or("");
    if last_segment.is_empty() { "hook".to_string() } else { last_segment.to_string() }
}
```

### Script generation
Implemented inline in this command's module (`generate_virtual_hook_script`).
Creates `spec/hooks/.virtual/`, writes the bash template w/ Unix mode 0o755
(use `std::os::unix::fs::PermissionsExt`), returns the relative-to-cwd path.

### Validation order (MUST match TS for parity)
1. Parse args (serde) — `InvalidArgs { reason: "failed to parse args: …" }`.
2. Load work-units file.
3. Work unit existence — `Work unit '<id>' does not exist`.
4. Mutate + atomic write.
5. Return `{success:true, hookCount:N}`.
