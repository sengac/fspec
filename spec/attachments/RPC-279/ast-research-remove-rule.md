# RPC-279 — remove-rule AST research

## TS source: `src/commands/remove-rule.ts`

Signature: `export async function removeRule(options: RemoveRuleOptions): Promise<RemoveRuleResult>`

### Behaviours observed

1. `cwd = options.cwd || process.cwd()`.
2. `await ensureWorkUnitsFile(cwd)` — auto-creates `spec/work-units.json` when missing.
3. **Validates work unit exists**: `!data.workUnits[options.workUnitId]` → throw `Error("Work unit '<id>' does not exist")`.
4. **Validates work unit is in specifying state**: `workUnit.status !== 'specifying'` → throw `Error("Can only remove rules during discovery/specification phase. <id> is in '<status>' state.")`.
5. **Validates rules array exists and is non-empty**: `!workUnit.rules || workUnit.rules.length === 0` → throw `Error("Work unit <id> has no rules")`.
6. **Find rule by ID** (stable index, not array index): `workUnit.rules.find(r => r.id === options.index)`. If not found → throw `Error("Rule with ID <index> not found")`.
7. **Idempotency**: if `rule.deleted === true` already, returns `{success:true, removedRule: rule.text, remainingCount: <non-deleted count>, message: "Item ID <index> already deleted"}` WITHOUT mutating disk.
8. **Soft-delete**: set `rule.deleted = true` and `rule.deletedAt = new Date().toISOString()`.
9. Bump `workUnit.updatedAt`.
10. Atomic write via `fileManager.transaction`.
11. Returns `{success:true, removedRule: <text>, remainingCount: <non-deleted count>}` (no `message`).

### CLI

- 2 positional args: `<workUnitId> <index>`.
- `index` is parsed via `parseInt(index, 10)`.
- On success: `output.log("✓ Removed rule: \"<removedRule>\"")` (note: TS-help reference says "Removed rule from AUTH-001" but actual code uses `Removed rule: "<text>"`).
- On failure: `output.error("✗ Failed to remove rule:", error.message)` + `process.exit(1)`.

### Help

- name: `remove-rule`
- description: "Remove a business rule from Example Mapping by index"
- usage: `fspec remove-rule <workUnitId> <index>`
- whenToUse: NONE
- arguments: workUnitId, index ("Rule index (from show-work-unit)")
- examples: 1 example, output `✓ Removed rule from AUTH-001` (note divergence from actual stdout)
- relatedCommands: add-rule, show-work-unit

### Rust port plan

- Reuse `ensure_work_units_file`, `write_json_atomic`, `iso8601_now`.
- `rules` stored as `Vec<Value>` in `WorkUnit.extra`.
- Find by `id` field (u64).
- Idempotent: if `deleted == true` → return early WITHOUT writing.
- Error variants via `FspecCoreError::InvalidArgs`.
- Note: TS uses `parseInt(index, 10)` so the dispatcher receives a number for `index`. Use `i64` (TS allows negative; we'll mirror).
- Return JSON `{success, removedRule, remainingCount, [message]}`.
- CLI stdout: `✓ Removed rule: "<text>"`.
