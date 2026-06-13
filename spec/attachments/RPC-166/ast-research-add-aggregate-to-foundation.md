# AST Research — `add-aggregate-to-foundation` (RPC-166)

Rust port of `src/commands/add-aggregate-to-foundation.ts` → `codelet/fspec-core/src/commands/add_aggregate_to_foundation.rs`.

## 1. Current Rust stub (to be rewritten)

`codelet/fspec-core/src/commands/add_aggregate_to_foundation.rs`:
```rust
pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted { command: "add-aggregate-to-foundation", work_unit: "RPC-166" })
}
```
- Signature MUST change to `run(args_json: &str, project_root: &Path)` to match the foundation-mutation template (`add_diagram.rs`). This requires a **supervisor-owned** edit at `dispatch.rs:427`.
- `commands/mod.rs:9` already declares `pub mod add_aggregate_to_foundation;` — no change needed.

## 2. Reference template — `add_diagram.rs` (closest foundation-mutation port)

`pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` confirmed via AstGrep. Pattern to reuse:
- `let args: T = serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs { command, reason: format!("failed to parse args: {e}") })?;`
- `let mut data: Value = ensure_foundation_file(project_root)?;` (auto-creates canonical generic schema v2.0.0 when missing)
- `data.as_object_mut()` → guard non-object → `ParseJson`.
- Mutate via `serde_json::Map`/`Value` entry API to preserve key order (workspace builds serde_json with `preserve_order`).
- `let path = project_root.join("spec").join("foundation.json"); write_json_atomic(&path, &data)?;`
- Serialize result `json!({ "success": true, "message": ... })`.

`add_bounded_context.rs` is the closest *event-storm-append* template BUT operates on `work-units.json` (different file, uses `existsSync` guard not auto-create). Borrow its **item-append + nextItemId post-increment** logic, NOT its file handling.

## 3. IO helpers available

- `crate::io::ensure::ensure_foundation_file(cwd: &Path) -> Result<serde_json::Value, FspecCoreError>` (`io/ensure.rs:86`) — load-or-init foundation.json, malformed → `ParseJson { file: "foundation.json" }`.
- `crate::io::locked_file::write_json_atomic<T: Serialize>(path, value)` (`io/locked_file.rs:96`) — pretty 2-space, **NO trailing newline**. ✅ CORRECT for FileManager-backed eventStorm commands (TS `FileManager.transaction` writes `JSON.stringify(data, null, 2)` with NO `'\n'`, confirmed at `src/utils/file-manager.ts:361`). Do NOT use `write_json_atomic_trailing_newline` (that's for add-capability/add-persona which append `'\n'`).
- `crate::io::time::iso8601_now()` — 24-char `...Z` ISO timestamp for `createdAt` (used by `add_bounded_context.rs`).

## 4. TS behaviour (`add-aggregate-to-foundation.ts`)

`export async function addAggregateToFoundation(contextName, aggregateName, options)` (line 23).

Logic:
1. `readJSON(foundationPath, defaults)` — validates file exists / seeds defaults.
2. `transaction(foundationPath, data => { ... })`:
   - If `!data.eventStorm` → seed `{ level: 'big_picture', items: [], nextItemId: 1 }`. **⚠ seed nextItemId = 1** (NOT 0 — diverges from the work-unit `add_bounded_context` seed of 0).
   - `boundedContext = items.find(i => i.type === 'bounded_context' && i.text === contextName)` — **NO `!deleted` filter on the ADD path** (parity-critical: add does not exclude soft-deleted contexts; only remove does).
   - If not found → `throw new Error("Bounded context '<contextName>' not found")`.
   - Build aggregate item, `items.push(...)`, `nextItemId++`.
3. `generateFoundationMdCommand({cwd})` — **Rust SKIPS** (RPC-233 unported; Framing A precedent from add_diagram).
4. Returns `{ success: true, message: 'Added aggregate "<aggregateName>" to "<contextName>" bounded context' }`.

### On-disk item shape & KEY ORDER (from TS object literal, lines 75-87)
```
{ id, type, text, boundedContextId, color, deleted, createdAt, [description] }
```
- `id` = `data.eventStorm.nextItemId` (number)
- `type` = `"aggregate"`
- `text` = aggregateName
- `boundedContextId` = `boundedContext.id` (number)
- `color` = `"yellow"` (string literal — contrast bounded_context color = `null`)
- `deleted` = `false`
- `createdAt` = `new Date().toISOString()`
- `description` = only inserted when `options.description` truthy (`...(options.description && { description })`)

⚠ Note: TS uses object-literal property order `id, type, text, boundedContextId, color, deleted, createdAt` then spreads optional `description` LAST. Reproduce exactly with `serde_json::Map` inserts in that order. (This differs from `add_bounded_context.rs` which appends id/deleted/createdAt last — because that command spreads `...itemData` first; here the literal lists `id` first.)

### CLI handler (lines 107-161)
- `addAggregateToFoundationCommand`: on success `output.log('✓', message)` → exit 0; on thrown error `output.error(chalk.red('Error:'), msg)` → exit 1.
- Commander registration: `.argument('<context-name>')`, `.argument('<aggregate-name>')`, `.option('-d, --description <text>')`.

## 5. Args shape (camelCase JSON for dispatcher + CLI bridge)
```
{ contextName: String, aggregateName: String, description?: Option<String> }
```
`#[serde(rename_all = "camelCase")]`. Missing required field → parse error → `InvalidArgs { command: "add-aggregate-to-foundation" }`, whose Display contains `Invalid args for fspec command add-aggregate-to-foundation`.

## 6. Two-front-doors / shared-file impact
- Dispatcher `dispatch.rs:427` + clap CLI both call `commands::add_aggregate_to_foundation::run(args_json, project_root)`.
- Supervisor-owned edits required: `dispatch.rs:427` (add `, project_root`), `fspec/src/main.rs` (mod decl + clap Mode variant `AddAggregateToFoundation{context_name, aggregate_name, description}` + forward! arm + help match arm), `help/configs/mod.rs` (`pub mod add_aggregate_to_foundation;`), and confirm `cargo_shape.rs` registration.
