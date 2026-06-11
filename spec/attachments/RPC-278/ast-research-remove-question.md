# AST Research — `remove-question` (RPC-278)

TS reference: `src/commands/remove-question.ts` (108 LOC) + `src/commands/remove-question-help.ts` (29 LOC).

## TS surface

```
fspec remove-question <workUnitId> <index>
```

No flags. Two required positional args. `<index>` is parsed via `parseInt(value, 10)` and is actually a STABLE ID, not a positional offset.

## TS function signature (export)

```ts
removeQuestion(options: { workUnitId: string; index: number; cwd?: string })
  : Promise<{ success: true; removedQuestion: string; remainingCount: number; message?: string }>
```

## TS algorithm (line-by-line)

1. `cwd = options.cwd || process.cwd()`.
2. `data = ensureWorkUnitsFile(cwd)`.
3. Validate `data.workUnits[id]` exists → `Work unit '<id>' does not exist`.
4. Validate `workUnit.status === 'specifying'` → `Can only remove questions during discovery/specification phase. <id> is in '<status>' state.`
5. Validate `workUnit.questions` exists AND `length > 0` → `Work unit <id> has no questions`.
6. Find question by ID: `workUnit.questions.find(q => q.id === options.index)`.
7. If not found → `Question with ID <id> not found`.
8. **Idempotent path** — if `question.deleted === true`:
   ```
   return { success: true, removedQuestion: question.text,
            remainingCount: workUnit.questions.filter(q => !q.deleted).length,
            message: `Item ID <id> already deleted` }
   ```
   NO write to disk.
9. Otherwise soft-delete: `question.deleted = true; question.deletedAt = new Date().toISOString()`.
10. Bump `workUnit.updatedAt = new Date().toISOString()`.
11. Atomic write via `fileManager.transaction`.
12. Return `{ success: true, removedQuestion, remainingCount: workUnit.questions.filter(q => !q.deleted).length }`.

## CLI wrapper (Commander.js)

`registerRemoveQuestionCommand(program)`:
- `.command('remove-question')`
- `.description('Remove a question from a work unit by index')`
- `.argument('<workUnitId>', 'Work unit ID')`
- `.argument('<index>', 'Question index (0-based)')`
- `.action(async (workUnitId, index) => { const r = await removeQuestion({...,index: parseInt(index,10)}); output.log(chalk.green(\`✓ Removed question: "${r.removedQuestion}"\`)); })`
- Errors: `output.error('✗ Failed to remove question:', error.message); process.exit(1);`

**Note**: CLI wrapper prints `✓ Removed question: "<text>"` — does NOT surface the `message` field for idempotent path. Idempotent path still prints the success line with the original (already-deleted) text.

## TS help (`remove-question-help.ts`)

```
name: 'remove-question'
description: 'Remove a question from Example Mapping by index'
usage: 'fspec remove-question <workUnitId> <index>'
arguments: workUnitId (required), index (required)
examples: 1 (CMD: fspec remove-question AUTH-001 0; OUT: ✓ Removed question from AUTH-001)
relatedCommands: [add-question, answer-question, show-work-unit]
```

Minimal help (no whenToUse, no commonPatterns, no typicalWorkflow, no notes).

## Rust port plan

- `commands/remove_question.rs`: `pub async fn run(args_json, project_root) -> Result<String>`.
- Args struct: `work_unit_id: String, index: u64` (camelCase rename).
- Use `ensure_work_units_file` for load-or-init.
- Read questions from `WorkUnit.extra["questions"]` (Value::Array).
- Look up by `id` field — linear scan.
- If `deleted == true`, return early with the idempotent payload (no disk write).
- Else mutate the array entry in place: set `deleted=true`, add `deletedAt`, write atomically.
- Result JSON shape: `{ success, removedQuestion, remainingCount, message? }`.
- CLI text path: `✓ Removed question: "<text>"`.

## Shared infra (existing — read-only access)

- `io::ensure::ensure_work_units_file`
- `io::locked_file::write_json_atomic`
- `io::time::iso8601_now`

## Divergences

- TS CLI prints the same success line for idempotent path (the `message` field
  is only visible to the dispatcher payload). Rust mirrors this — CLI prints
  `✓ Removed question: "<text>"` even when the underlying item was already
  deleted; the `message` field appears only in the dispatcher JSON.
