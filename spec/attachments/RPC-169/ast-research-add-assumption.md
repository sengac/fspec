# RPC-169 — add-assumption AST research

## TS source: `src/commands/add-assumption.ts`

Signature: `export async function addAssumption(options: AddAssumptionOptions): Promise<AddAssumptionResult>`

### Behaviours observed

1. `cwd = options.cwd || process.cwd()`.
2. `await ensureWorkUnitsFile(cwd)`.
3. **Validates work unit exists**: throws `Error("Work unit '<id>' does not exist")`.
4. **Validates specifying state**: throws `Error("Can only add assumptions during discovery/specification phase. <id> is in '<status>' state.")`.
5. **Initialises `workUnit.assumptions` to empty array if absent**.
6. Pushes raw string `options.assumption` (NOT a stable-id item — assumptions are plain strings per types/index.ts:160 `assumptions?: string[]`).
7. Bumps `workUnit.updatedAt`.
8. Atomic write.
9. Returns `{success:true, assumptionCount: workUnit.assumptions.length}`.

### CLI

- 2 positional args. Commander uses `<work-unit-id>` kebab-case (becomes `workUnitId` in action callback) and `<assumption>`.
- On success: `output.log('✓ Assumption added successfully')`.
- On failure: `output.error('✗ Failed to add assumption:', error.message)` + `process.exit(1)`.

### Help

- name: `add-assumption`
- description: "Add an assumption to a work unit during specification"
- usage: `fspec add-assumption <work-unit-id> <assumption>`
- whenToUse: "Use to document assumptions made during specification that may need validation later."
- arguments: work-unit-id, assumption
- examples: 1 example
- relatedCommands: add-rule, add-question, show-work-unit

### Rust port plan

- Same shape as `add-rule` but simpler (no stable-id object — just a string array).
- `assumptions` stored as `Vec<Value::String>` in `WorkUnit.extra`.
- Return JSON `{success, assumptionCount}`.
- CLI stdout: `✓ Assumption added successfully`.
