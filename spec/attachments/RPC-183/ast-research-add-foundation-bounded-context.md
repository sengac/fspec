# AST Research — add-foundation-bounded-context (RPC-183)

Rust port of `src/commands/add-foundation-bounded-context.ts` → `codelet/fspec-core/src/commands/add_foundation_bounded_context.rs`.

## 1. Current Rust stub (to be rewritten)

`codelet/fspec-core/src/commands/add_foundation_bounded_context.rs`:
```rust
pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted { command: "add-foundation-bounded-context", work_unit: "RPC-183" })
}
```
New signature MUST be `run(args_json: &str, project_root: &Path)` (matches `add_diagram::run`). **SHARED-FILE**: `dispatch.rs` call-site must pass `project_root`.

## 2. TS source semantics (`add-foundation-bounded-context.ts`)

- Reads `<cwd>/spec/foundation.json` with a generic v2.0.0 default.
- Inside a `transaction`, seeds `data.eventStorm` if missing:
  ```ts
  { level: 'big_picture', items: [], nextItemId: 1 }   // NOTE: starts at 1
  ```
- Builds item (object-literal insertion order → JSON key order):
  ```ts
  { id: nextItemId, type: 'bounded_context', text, color: null, deleted: false, createdAt: new Date().toISOString() }
  ```
- `items.push(boundedContext); data.eventStorm.nextItemId++;`
- Calls `generateFoundationMdCommand({ cwd })` (auto-regen FOUNDATION.md).
- Returns `{ success: true, message: 'Added bounded context "<text>" to foundation Event Storm' }`.
- CLI wrapper prints `output.log('✓', result.message)` exit 0; on throw prints `output.error(chalk.red('Error:'), msg)` exit 1.
- Commander registration: `add-foundation-bounded-context <text>` — single positional, NO options.

### Type confirmation (`src/types/index.ts:99` + `generic-foundation.ts:24-38`)
```ts
EventStormBoundedContext extends Omit<EventStormItemBase,'color'> { type:'bounded_context'; color:null; description?; itemIds?; }
FoundationEventStorm extends EventStormBase { level:'big_picture' }   // items[], nextItemId
```

## 3. Divergences vs work-unit event storm (add_bounded_context.rs, RPC-172)

| Aspect | work-unit (add_bounded_context.rs) | foundation (THIS, RPC-183) |
|---|---|---|
| Target file | `spec/work-units.json` | `spec/foundation.json` |
| Level | `process_modeling` | `big_picture` |
| nextItemId seed | **0** | **1** |
| Missing-file behaviour | error "not found. Run fspec init first." (existsSync guard, NO auto-create) | **auto-create** default schema via `ensure_foundation_file` |
| Item key order | type, color, text, [desc], [ts], [bc], id, deleted, createdAt | **id, type, text, color, deleted, createdAt** |
| Status guard | done/blocked rejected | n/a (no work-unit) |
| Result key | `boundedContextId` | `message` |

The TS object-literal order differs from the work-unit command — confirmed verbatim from `add-foundation-bounded-context.ts:70-77`.

## 4. IO helpers available (verified via AstGrep)

- `src/io/ensure.rs:86` — `pub fn ensure_foundation_file(cwd: &Path) -> Result<serde_json::Value, FspecCoreError>` — load-or-init, writes canonical generic v2.0.0 default when missing. Use this (auto-create parity).
- `src/io/locked_file.rs:96` — `pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), FspecCoreError>` — 2-space pretty, **NO trailing newline** (correct for FileManager eventStorm commands per supervisor note). `write_json_atomic_trailing_newline` exists but is NOT for us.
- `src/io/time.rs:34` — `pub fn iso8601_now() -> String` → `"YYYY-MM-DDThh:mm:ss.000Z"` (24 chars, ends 'Z').

## 5. Implementation plan (core)

1. Parse args `{ text: String }` (serde, camelCase) — single field.
2. `let mut data = ensure_foundation_file(project_root)?;` (Value).
3. `root_obj.entry("eventStorm")` → seed `{level:'big_picture', items:[], nextItemId:1}` if absent / non-object (entry API like add_diagram's architectureDiagrams coercion).
4. `let id = es["nextItemId"].as_u64().unwrap_or(1);`
5. Build `Map` in order: id, type='bounded_context', text, color=Null, deleted=false, createdAt=iso8601_now().
6. Push to `items` (coerce to array if needed); set `nextItemId = id + 1`.
7. `write_json_atomic(spec/foundation.json, &data)`.
8. Return `{ success:true, message:"Added bounded context \"<text>\" to foundation Event Storm" }`.

Round-trip whole doc as `serde_json::Value` (preserve_order) → unknown top-level fields preserved byte-for-byte.

## 6. Framing A (confirmed w/ supervisor)

Core does NOT regenerate `spec/FOUNDATION.md` (generate-foundation-md = unported stub RPC-233). CLI bridge prints `  Regenerated: spec/FOUNDATION.md` parity line (add_diagram bridge precedent at `codelet/fspec/src/add_diagram.rs:59`).

## 7. Two-front-doors / bridge

`codelet/fspec/src/add_foundation_bounded_context.rs` (NEW): clap `Mode` variant `add-foundation-bounded-context <text>`, marshals JSON `{text}`, dispatches, renders `✓ <message>` + regen line on success, `Error: <reason>` exit 1 on failure. NO domain logic.

## 8. Shared-file requests (supervisor-only)

- `dispatch.rs` + wiring: route command → `run(args_json, project_root)`.
- `help/configs/mod.rs`: register new help config (config file authored in owned path).
- `fspec/src/main.rs`: clap Mode variant.
- `cargo_shape.rs`: command/help-fixture inventory if asserted.

## 9. Test fixture shape (dispatch test, from add_diagram.rs:1-50 pattern)

`empty_foundation()` helper writes `{version, project, ...}` with no eventStorm; assert `read_foundation()["eventStorm"]["items"][0]` fields + key order.
