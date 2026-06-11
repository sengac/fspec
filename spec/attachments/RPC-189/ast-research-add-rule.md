# RPC-189 — add-rule AST research

## TS source: `src/commands/add-rule.ts`

Signature: `export async function addRule(options: AddRuleOptions): Promise<AddRuleResult>`

```ts
interface AddRuleOptions {
  workUnitId: string;
  rule: string;
  cwd?: string;
}
interface AddRuleResult {
  success: boolean;
  ruleCount: number;
}
```

### Behaviours observed

1. `cwd = options.cwd || process.cwd()`. Project root resolution.
2. `await ensureWorkUnitsFile(cwd)` — auto-creates `spec/work-units.json` when missing (using the canonical initial structure).
3. **Validates work unit exists.** If `!data.workUnits[options.workUnitId]` → throw `Error("Work unit '<id>' does not exist")`.
4. **Validates work unit is in specifying state.** If `workUnit.status !== 'specifying'` → throw `Error("Can only add rules during discovery/specification phase. <id> is in '<status>' state.")`.
5. **Initializes `workUnit.rules` array if absent** (`if (!workUnit.rules) { workUnit.rules = [] }`).
6. **Initializes `workUnit.nextRuleId` if `undefined`** (backward compat). Defaults to `0`.
7. Builds a `RuleItem` with `{ id: workUnit.nextRuleId++, text: options.rule, deleted: false, createdAt: new Date().toISOString() }`.
   - Increments `nextRuleId` AFTER assignment (post-increment).
8. Pushes new rule onto `workUnit.rules`.
9. Bumps `workUnit.updatedAt = new Date().toISOString()`.
10. Atomic write via `fileManager.transaction(workUnitsFile, async fileData => { Object.assign(fileData, data); })`.
11. Returns `{ success: true, ruleCount: workUnit.rules.length }`.

### CLI surface (Commander.js)

```ts
program
  .command('add-rule')
  .description('Add a business rule to a work unit during specification phase')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<rule>', 'Business rule description')
  .action(...)
```

- 2 positional args, NO flags.
- On error: `output.error('✗ Failed to add rule:', error.message)` + `process.exit(1)`.
- On success: `output.log('✓ Rule added successfully')`.

### Help — `src/commands/add-rule-help.ts`

- name: `add-rule`
- description: "Add a business rule to a work unit during Example Mapping"
- usage: `fspec add-rule <workUnitId> <rule>`
- whenToUse: "Use during specifying phase when capturing business rules discovered through Example Mapping conversations."
- arguments: workUnitId (required), rule (required)
- examples: 1 example
- relatedCommands: add-question, add-example, generate-scenarios, remove-rule

### RuleItem on-disk shape (TS `ItemWithId`)

```ts
{
  id: number,        // 0-based, auto-incrementing
  text: string,
  deleted: boolean,  // soft-delete flag
  createdAt: string, // ISO 8601
  deletedAt?: string // optional, only when deleted=true
}
```

### Rust port plan

- Rust `WorkUnit` has `#[serde(flatten)] extra: Map<String, Value>`. Both `rules` and `nextRuleId` live in `extra`.
- Reuse `io::ensure::ensure_work_units_file` (auto-creates).
- Reuse `io::locked_file::write_json_atomic` (single atomic write).
- Reuse `io::time::iso8601_now()` for `createdAt` and `updatedAt`.
- Error: validation failures via `FspecCoreError::InvalidArgs { command: "add-rule", reason }`.
- Return JSON `{success, ruleCount}` for dispatcher (caller may pretty-print or not — the existing `add_dependencies` returns single-line JSON).
- CLI text: `✓ Rule added successfully` on success; stderr `Error: <reason>` on failure (via `render_core_error`).

### Insertion order

The RuleItem JSON shape must preserve field order: `id, text, deleted, createdAt`. Use `serde_json::Map` (insertion-order preserving) when building the rule object inside `extra`.
