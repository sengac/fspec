# AST Research — `add-question` (RPC-188)

TS reference: `src/commands/add-question.ts` (101 LOC) + `src/commands/add-question-help.ts` (60 LOC).

## TS surface

```
fspec add-question <workUnitId> <question>
```

No flags. Two required positional args.

## TS function signature (export)

```ts
addQuestion(options: { workUnitId: string; question: string; cwd?: string })
  : Promise<{ success: true; questionCount: number; mentionedPeople?: string[] }>
```

## TS algorithm (line-by-line)

1. `cwd = options.cwd || process.cwd()`.
2. `workUnitsFile = join(cwd, 'spec/work-units.json')`.
3. `data = ensureWorkUnitsFile(cwd)` — auto-creates file when missing.
4. Validate `data.workUnits[options.workUnitId]` exists → else throw
   `Work unit '<id>' does not exist`.
5. Validate `workUnit.status === 'specifying'` → else throw
   `Can only add questions during discovery/specification phase. <id> is in '<status>' state.`
6. If `!workUnit.questions` → set `[]`.
7. If `workUnit.nextQuestionId === undefined` → set `0` (backward compat).
8. Build `QuestionItem`:
   ```
   { id: workUnit.nextQuestionId++,
     text: options.question,
     deleted: false,
     createdAt: new Date().toISOString(),
     selected: false }
   ```
9. Push onto `workUnit.questions`.
10. Extract `@mentions` from question text via regex `/@\w+/g`, strip the
    leading `@` to produce `mentionedPeople`.
11. Set `workUnit.updatedAt = new Date().toISOString()`.
12. `fileManager.transaction(workUnitsFile, async fileData => Object.assign(fileData, data))`
    — atomic write under exclusive lock.
13. Return `{ success: true, questionCount, ...(mentionedPeople.length > 0 && { mentionedPeople }) }`.

## CLI wrapper (Commander.js)

`registerAddQuestionCommand(program)`:
- `.command('add-question')`
- `.description('Add a question to a work unit during specification phase')`
- `.argument('<workUnitId>', 'Work unit ID')`
- `.argument('<question>', 'Question text')`
- `.action(async (workUnitId, question) => { await addQuestion(...); output.log('✓ Question added successfully'); })`
- Errors: `output.error('✗ Failed to add question:', error.message); process.exit(1);`

**Note**: CLI wrapper DROPS the result object (no `questionCount` / `mentionedPeople` in stdout). Result only visible via dispatcher.

## TS help (`add-question-help.ts`)

```
name: 'add-question'
description: 'Add a question to a work unit during Example Mapping discovery phase'
usage: 'fspec add-question <workUnitId> <question>'
whenToUse: <long>
commonPatterns: [4 entries]
arguments: workUnitId (required), question (required)
examples: 2
typicalWorkflow: 5-step
relatedCommands: [answer-question, add-rule, add-example, generate-scenarios, show-work-unit]
notes: [3 entries]
```

## Rust port plan

- `commands/add_question.rs`: `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
- Args struct: `work_unit_id: String, question: String` (camelCase rename).
- Use `ensure_work_units_file(project_root)` for load-or-init.
- Use `WorkUnit.extra` for `questions`, `nextQuestionId` (not typed
  fields).
- Use `write_json_atomic(spec/work-units.json, &data)`.
- Use `iso8601_now()` for timestamps.
- Mention extraction: lightweight `\w+` (ASCII alpha/digit/underscore)
  scan after `@`.
- Result shape: `{ success: true, questionCount: N, mentionedPeople?: [..] }` serialized as JSON when `format=json`.
- CLI text path: print `✓ Question added successfully`.

## Shared infra (existing — read-only access)

- `io::ensure::ensure_work_units_file`
- `io::locked_file::write_json_atomic`
- `io::time::iso8601_now`
- `types::work_unit::WorkUnitStatus::Specifying`

## Divergences

- TS `mentionedPeople` uses `/@\w+/g` (JavaScript `\w` = `[A-Za-z0-9_]`).
  Rust port mirrors with explicit ASCII alpha/digit/underscore class
  (no Unicode `\w` semantics).
- `output.log` text-only path; rich JSON only via dispatcher
  `format=json`.
