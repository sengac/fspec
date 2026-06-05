# AST Research — RPC-252 list-virtual-hooks port

## TS source signature (src/commands/list-virtual-hooks.ts)

```ts
export async function listVirtualHooks(
  options: ListVirtualHooksOptions
): Promise<ListVirtualHooksResult>
```

- Pattern: `export async function listVirtualHooks($$$ARGS): Promise<$RET> { $$$BODY }`
- Match: `/Users/rquast/projects/fspec/src/commands/list-virtual-hooks.ts:18`

Body summary:
1. Resolve `cwd = options.cwd || process.cwd()`
2. `data = await ensureWorkUnitsFile(cwd)` (auto-creates empty store on missing)
3. If `!data.workUnits[id]` → throw `Error("Work unit '<id>' does not exist")`
4. `hooks = workUnit.virtualHooks || []`
5. Group into `hooksByEvent: Record<string, VirtualHook[]>` preserving insertion order
6. Return `{hooks, hooksByEvent}`

## Reference Rust port signature (codelet/fspec-core/src/commands/list_work_units.rs)

```rust
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

- Pattern: `pub async fn run($$$ARGS) -> Result<$OK, $ERR> { $$$BODY }`
- Match: `/Users/rquast/projects/fspec/codelet/fspec-core/src/commands/list_work_units.rs:42`

Pattern adopted for the port:
1. Parse args (workUnitId required, format optional) via `serde_json::from_str` → `InvalidArgs` on failure
2. `let data = ensure_work_units_file(project_root)?` (shared helper)
3. Lookup `data.work_units.get(&args.work_unit_id)` → `InvalidArgs { reason: "Work unit '<id>' does not exist" }` if absent
4. Read `wu.extra["virtualHooks"]` as `Value::Array`; iterate to build typed `VirtualHook` Vec
5. Group into `IndexMap<String, Vec<VirtualHook>>` keyed by `event` preserving insertion order
6. Render either JSON (2-space `to_string_pretty`) or text (sentinel / header + per-event sections with `[blocking]`/`[non-blocking]`/`[git-context]` badges)

## VirtualHook shape (src/types/index.ts:36)

```ts
interface VirtualHook {
  name: string;
  event: string;
  command: string;
  blocking: boolean;
  gitContext?: boolean;
}
```

Rust mirror:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VirtualHook {
    name: String,
    event: String,
    command: String,
    blocking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "gitContext")]
    git_context: Option<bool>,
}
```
