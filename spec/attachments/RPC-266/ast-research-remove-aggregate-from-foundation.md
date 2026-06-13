# AST Research — `remove-aggregate-from-foundation` (RPC-266)

Rust port of `src/commands/remove-aggregate-from-foundation.ts` → `codelet/fspec-core/src/commands/remove_aggregate_from_foundation.rs`.

## 1. Current Rust stub (to be rewritten)

`codelet/fspec-core/src/commands/remove_aggregate_from_foundation.rs`:
```rust
pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted { command: "remove-aggregate-from-foundation", work_unit: "RPC-266" })
}
```
- Signature MUST change to `run(args_json: &str, project_root: &Path)`. Requires **supervisor-owned** edit at `dispatch.rs:575`.
- `commands/mod.rs:109` already declares `pub mod remove_aggregate_from_foundation;` — no change needed.

## 2. Reference template

Same foundation-mutation template as RPC-166: `add_diagram.rs` `run(args_json, project_root)` → `ensure_foundation_file` → `Value` mutation (preserve_order) → `write_json_atomic`. This command only flips a boolean (`deleted = true`); no item construction, no nextItemId touch.

## 3. IO helpers (same as RPC-166)

- `crate::io::ensure::ensure_foundation_file(project_root)` → `Value`.
- `crate::io::locked_file::write_json_atomic(&path, &data)` — pretty 2-space, **NO trailing newline** (FileManager parity confirmed).
- No timestamp helper needed (soft-delete does not set/update any timestamp on the item in TS).

## 4. TS behaviour (`remove-aggregate-from-foundation.ts`)

`export async function removeAggregateFromFoundation(contextName, aggregateName, options)` (line 26).

Logic inside `transaction`:
1. **If `!data.eventStorm`** → `throw new Error("Bounded context '<contextName>' not found (no Event Storm data)")`. ⚠ Distinct message with the `(no Event Storm data)` suffix — separate scenario from the plain not-found below.
2. `boundedContext = items.find(i => i.type === 'bounded_context' && i.text === contextName && !i.deleted)` — **`!deleted` filter IS applied here** (unlike the add path). If not found → `throw new Error("Bounded context '<contextName>' not found")`.
3. `aggregate = items.find(i => i.type === 'aggregate' && i.text === aggregateName && !i.deleted && 'boundedContextId' in i && i.boundedContextId === boundedContext.id)`. If not found → `throw new Error("Aggregate '<aggregateName>' not found in bounded context '<contextName>'")`.
4. `aggregate.deleted = true` — soft-delete; item REMAINS in `items`.
5. `generateFoundationMdCommand({cwd})` — **Rust SKIPS** (Framing A).
6. Returns `{ success: true, message: 'Removed aggregate "<aggregateName>" from "<contextName>" bounded context' }`.

### Matching rules (parity-critical)
- Already-soft-deleted bounded context (`deleted: true`) → treated as not-found → `"Bounded context '<ctx>' not found"`.
- Already-soft-deleted aggregate → treated as not-found → `"Aggregate '<agg>' not found in bounded context '<ctx>'"`.
- Aggregate matching is scoped by `boundedContextId === boundedContext.id`, so an identically-named aggregate in a DIFFERENT context is untouched.
- `'boundedContextId' in item` guard before comparing — items lacking the field are skipped.

### CLI handler (lines 100-153)
- On success `output.log('✓', message)` → exit 0; on error `output.error(chalk.red('Error:'), msg)` → exit 1.
- Commander registration: `.argument('<context-name>')`, `.argument('<aggregate-name>')` — **no options**.

## 5. Args shape (camelCase JSON)
```
{ contextName: String, aggregateName: String }
```
`#[serde(rename_all = "camelCase")]`. Missing field → `InvalidArgs { command: "remove-aggregate-from-foundation" }` → Display contains `Invalid args for fspec command remove-aggregate-from-foundation`.

## 6. Implementation sketch (Value-based traversal)

```text
let mut data = ensure_foundation_file(project_root)?;
let es = data.get("eventStorm") -> else Err("Bounded context '<ctx>' not found (no Event Storm data)")
let items = es["items"].as_array() (else same no-data error / treat as empty)
// pass 1: find bc id
let bc_id = items.iter().find(|i| type==bounded_context && text==ctx && !deleted).map(id)
            -> else Err("Bounded context '<ctx>' not found")
// pass 2: find aggregate index
let idx = items.iter().position(|i| type==aggregate && text==agg && !deleted && boundedContextId==bc_id)
          -> else Err("Aggregate '<agg>' not found in bounded context '<ctx>'")
items[idx]["deleted"] = true;  // mutate in place via as_object_mut
write_json_atomic(&spec/foundation.json, &data)?;
```
Borrow-checker note: resolve `bc_id` (a copyable u64/i64) in an immutable pass, then take `as_array_mut` for the mutating pass to avoid overlapping borrows.

## 7. Two-front-doors / shared-file impact
- Dispatcher `dispatch.rs:575` + clap CLI both call `commands::remove_aggregate_from_foundation::run(args_json, project_root)`.
- Supervisor-owned edits required: `dispatch.rs:575` (add `, project_root`), `fspec/src/main.rs` (mod decl + clap Mode variant `RemoveAggregateFromFoundation{context_name, aggregate_name}` + forward! arm + help match arm), `help/configs/mod.rs` (`pub mod remove_aggregate_from_foundation;`), confirm `cargo_shape.rs`.
