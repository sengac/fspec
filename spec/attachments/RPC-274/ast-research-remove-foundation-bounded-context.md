# AST Research — remove-foundation-bounded-context (RPC-274)

Rust port of `src/commands/remove-foundation-bounded-context.ts` → `codelet/fspec-core/src/commands/remove_foundation_bounded_context.rs`.

## 1. Current Rust stub (to be rewritten)

```rust
pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted { command: "remove-foundation-bounded-context", work_unit: "RPC-274" })
}
```
New signature MUST be `run(args_json: &str, project_root: &Path)`. **SHARED-FILE**: `dispatch.rs` call-site.

## 2. TS source semantics (`remove-foundation-bounded-context.ts`)

Options: `{ cwd?, cascade? }`. Argv: `remove-foundation-bounded-context <context-name> [--cascade]`.

Flow inside `transaction`:
1. If `!data.eventStorm` → throw `Bounded context '<name>' not found (no Event Storm data)`.
2. Find target: `items.find(i => i.type==='bounded_context' && i.text===contextName && !i.deleted)`. If none → throw `Bounded context '<name>' not found`.
3. Count children: `items.filter(i => !i.deleted && 'boundedContextId' in i && i.boundedContextId === target.id)`.
4. If `childItems.length > 0 && !cascade` → throw `Bounded context '<name>' has <n> child items. Use --cascade to remove the context and all its children.`
5. `target.deleted = true`.
6. If `cascade && childItems.length>0` → each child `.deleted = true`.
7. `generateFoundationMdCommand({ cwd })`.
8. Return `{ success:true, message: 'Removed bounded context "<name>"<cascadeMsg> from foundation Event Storm' }` where `cascadeMsg = cascade ? ' and all its children' : ''`.

CLI: `output.log('✓', result.message)` exit 0; throw → `output.error(chalk.red('Error:'), msg)` exit 1.

NOTE: TS reads foundation with a v2.0.0 default BEFORE the transaction (so a totally-missing foundation.json gets auto-created, then hits the `!data.eventStorm` throw → "not found (no Event Storm data)"). The Rust `ensure_foundation_file` matches this (auto-creates, then errors on missing eventStorm).

## 3. Children carry `boundedContextId` (verified)

`grep boundedContextId src/commands/add-{aggregate,domain-event,command}-to-foundation.ts`:
```
boundedContextId: number;
boundedContextId: boundedContext.id,
```
So child aggregates/events/commands store a NUMERIC `boundedContextId` = parent context id. The match in remove uses `'boundedContextId' in item` + numeric equality. (Do NOT confuse with `boundedContext?: string` NAME field on EventStormItemBase.)

## 4. Soft-delete, not splice

`target.deleted = true` — item stays in `items[]`. Already-deleted items are excluded from the match filter (`!i.deleted`) → re-removing a deleted context = "not found".

## 5. IO helpers (shared with RPC-183, verified via AstGrep)

- `src/io/ensure.rs:86` `ensure_foundation_file(cwd) -> Value` (auto-create default schema).
- `src/io/locked_file.rs:96` `write_json_atomic` (2-space, no trailing newline — correct per supervisor note).
- No timestamp needed (soft-delete only mutates `deleted`).

## 6. Implementation plan (core)

1. Parse args `{ contextName: String, cascade: Option<bool> }` (serde camelCase).
2. `let mut data = ensure_foundation_file(project_root)?;`
3. Get `eventStorm` object → if absent: `Err InvalidArgs "Bounded context '<name>' not found (no Event Storm data)"` (NO write).
4. Borrow `items` array. Find index of first `{type=='bounded_context', text==name, deleted!=true}`. None → `Err "...not found"` (NO write).
5. Read target `id` (u64).
6. Collect child indices: `!deleted && item.get("boundedContextId").as_u64()==Some(target_id)`.
7. If `!children.is_empty() && cascade != Some(true)` → `Err "...has <n> child items. Use --cascade..."` (NO write).
8. Set `items[target_idx]["deleted"]=true`; if cascade set each child `deleted=true`.
9. `write_json_atomic(...)` — only on success.
10. Return `{success:true, message}` with cascade suffix.

Borrow note: collect indices first (immutable scan) then mutate by index to satisfy borrow checker, since both target & children live in the same `items` array.

## 7. Framing A

Core does NOT touch FOUNDATION.md. CLI bridge prints `  Regenerated: spec/FOUNDATION.md`.

## 8. Two-front-doors / bridge

`codelet/fspec/src/remove_foundation_bounded_context.rs` (NEW): clap `<context-name> [--cascade]`, marshals JSON `{contextName, cascade?}`, dispatches, renders `✓ <message>` + regen line / `Error: <reason>` exit 1. NO domain logic.

## 9. Shared-file requests (supervisor-only)

- `dispatch.rs` wiring → `run(args_json, project_root)`.
- `help/configs/mod.rs` registration.
- `fspec/src/main.rs` clap Mode variant (with `--cascade` bool flag).
- `cargo_shape.rs` inventory if asserted.

## 10. Test scenarios (dispatch + cli)

childless remove; non-empty refuse (no cascade); cascade removes context+children; no-match not-found; already-deleted not-found; no-eventStorm not-found. Byte-equality assertion on error paths (read foundation before/after). Children built with `boundedContextId` = parent id in test fixtures.
