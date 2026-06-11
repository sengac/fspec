# AST Research — `remove-virtual-hook` (RPC-283)

Authoritative TS sources:

| File | Lines | Role |
|------|-------|------|
| `src/commands/remove-virtual-hook.ts` | 103 LOC | Core impl + Commander registration |
| `src/commands/remove-virtual-hook-help.ts` | 96 LOC | Help config |
| `src/hooks/script-generation.ts` | 123 LOC | `cleanupVirtualHookScript` helper |
| `src/types/index.ts:36-42` | — | `VirtualHook` interface |

## TS contract (verbatim)

```ts
interface RemoveVirtualHookOptions {
  workUnitId: string;
  hookName: string;
  cwd?: string;
}

interface RemoveVirtualHookResult {
  success: boolean;
  remainingCount: number;
}
```

## Algorithm (single source of truth — `removeVirtualHook` lines 21-78)

1. `cwd = options.cwd || process.cwd()`.
2. `workUnitsFile = join(cwd, 'spec/work-units.json')`.
3. Load via `ensureWorkUnitsFile(cwd)`.
4. Existence check on work unit: `Work unit '<id>' does not exist`.
5. **No hooks check**: if `!workUnit.virtualHooks || workUnit.virtualHooks.length === 0`:
   → throw `No virtual hooks configured for <workUnitId>`.
6. Snapshot `initialLength = workUnit.virtualHooks.length`.
7. **Filter out** every entry whose `name === hookName`. This means **all hooks
   with that name are removed**, not just the first — important parity note,
   because the help docs say "If multiple hooks have same name at different
   events, only first is removed" which is FALSE per the code (the code uses
   `.filter()`, not `.findIndex() + .splice(_, 1)`). We mirror the CODE.
8. If `virtualHooks.length === initialLength` (nothing removed):
   → throw `Virtual hook '<hookName>' not found in <workUnitId>`.
9. Best-effort cleanup of the script file (ignore all errors):
   ```ts
   try { await cleanupVirtualHookScript({ workUnitId, hookName, projectRoot:cwd }); }
   catch {}
   ```
10. `workUnit.updatedAt = new Date().toISOString()`.
11. Atomic write via `fileManager.transaction`.
12. Return `{ success: true, remainingCount: virtualHooks.length }`.

## Script cleanup (from `script-generation.ts` lines 110-123)

Computes path `spec/hooks/.virtual/<workUnitId>-<hookName>.sh`, calls `unlink`,
swallows `ENOENT`, rethrows other errors. **But the caller swallows EVERY
error** via the outer `try/catch {}`, so in practice the helper never
propagates failure. We replicate that in Rust: ignore all I/O errors from
the script-removal path.

## CLI surface (Commander.js)

```
fspec remove-virtual-hook <workUnitId> <hookName>
```

Success output:
```
✓ Removed virtual hook '<hookName>' from <workUnitId>
  Remaining virtual hooks: <remainingCount>
```

Error path:
```
✗ Failed to remove virtual hook: <error.message>
```
+ `process.exit(1)`.

## Rust port plan

| Artifact | Path |
|----------|------|
| Core impl | `codelet/fspec-core/src/commands/remove_virtual_hook.rs` |
| Help config | `codelet/fspec-core/src/help/configs/remove_virtual_hook.rs` |
| CLI bridge | `codelet/fspec/src/remove_virtual_hook.rs` |
| Dispatcher test | `codelet/fspec-core/tests/remove_virtual_hook.rs` |
| CLI test | `codelet/fspec/tests/cli_remove_virtual_hook.rs` |
| Help fixture | `codelet/fspec/tests/fixtures/help/remove-virtual-hook.txt` |

### Args
```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveVirtualHookArgs {
    work_unit_id: String,
    hook_name: String,
}
```

### Result
```rust
#[derive(Debug, Serialize)]
struct RemoveVirtualHookResult { success: bool, remaining_count: usize }
```
Serialized as `{"success":true,"remainingCount":<n>}`.

### Storage shape — read/mutate `wu.extra["virtualHooks"]`
```rust
let arr = wu.extra.get_mut("virtualHooks").and_then(|v| v.as_array_mut());
```
- Missing OR empty array → return `No virtual hooks configured for <id>` (TS
  treats `!hooks || length === 0` identically).
- Apply `arr.retain(|h| h.get("name") != Some(&Value::String(hook_name)))`.

### Script cleanup
Implemented inline as `cleanup_virtual_hook_script(workUnitId, hookName,
projectRoot)`. Path = `<projectRoot>/spec/hooks/.virtual/<workUnitId>-<hookName>.sh`.
Use `std::fs::remove_file` — match `Ok(_)` or `NotFound` and ignore; any
other I/O error also silently dropped (TS swallows all).

### Validation order
1. Parse args.
2. Load work-units.
3. Work unit existence.
4. virtualHooks missing/empty check.
5. Filter; verify count changed.
6. Cleanup script (best-effort).
7. Bump updatedAt + atomic write.
8. Return result.
