# RPC-196 — answer-question AST research

## TS source: `src/commands/answer-question.ts`

Signature: `export async function answerQuestion(options: AnswerQuestionOptions): Promise<AnswerQuestionResult>`

```ts
interface AnswerQuestionOptions {
  workUnitId: string;
  index: number;
  answer?: string;
  addTo?: 'rule' | 'assumption' | 'rules' | 'assumptions' | 'none';
  cwd?: string;
}
interface AnswerQuestionResult {
  success: boolean;
  question: string;
  addedTo?: 'rules' | 'assumptions';
  addedContent?: string;
}
```

### Behaviours observed

1. `cwd = options.cwd || process.cwd()`. Project root resolution.
2. `workUnitsFile = join(cwd, 'spec/work-units.json')`.
3. `await ensureWorkUnitsFile(cwd)` — auto-creates `spec/work-units.json`.
4. **Validates work unit exists.** If `!data.workUnits[options.workUnitId]` → `Error("Work unit '<id>' does not exist")`.
5. **Validates status == 'specifying'.** If not → `Error("Can only answer questions during discovery/specification phase. <id> is in '<status>' state.")`.
6. **Validates questions array exists and non-empty.** If `!workUnit.questions || workUnit.questions.length === 0` → `Error("Work unit <id> has no questions")`.
7. **Validates index bounds.** If `index < 0 || index >= length` → `Error("Invalid question index <i>. Valid range: 0-<len-1>")`.
8. **Retrieves question object** at index. Casts to `QuestionItem`. If the value is a raw string or null → `Error("Question format is invalid. Expected QuestionItem object.")`.
9. **Marks question:**
   - `question.selected = true` (always).
   - If `options.answer`:
     - `question.answered = true`
     - `question.answer = options.answer`
10. **Conditionally adds to rules or assumptions** when `options.answer && options.addTo && options.addTo !== 'none'`:
    - If `addTo === 'rule' || addTo === 'rules'`:
      - Init `workUnit.rules = []` if absent.
      - Create proper `RuleItem`:
        ```
        { id: workUnit.nextRuleId++, text: answer, deleted: false, createdAt: now }
        ```
      - Push onto `workUnit.rules`.
      - `addedTo = 'rules'; addedContent = answer`.
    - Else if `addTo === 'assumption' || addTo === 'assumptions'`:
      - Init `workUnit.assumptions = []` if absent.
      - **Assumptions are raw strings**, NOT objects. Push `answer` as a plain string.
      - `addedTo = 'assumptions'; addedContent = answer`.
11. `workUnit.updatedAt = new Date().toISOString()`.
12. Atomic write via `fileManager.transaction`.
13. Returns `{ success: true, question: questionText, ...(addedTo && {addedTo}), ...(addedContent && {addedContent}) }`.

### Question shape (`QuestionItem`)

```ts
{
  id: number,
  text: string,
  deleted: boolean,
  createdAt: string,
  selected?: boolean,
  answered?: boolean,
  answer?: string
}
```

### RuleItem shape (when addTo=rule)

```ts
{
  id: number,        // = workUnit.nextRuleId pre-increment
  text: string,      // = options.answer
  deleted: boolean,  // = false
  createdAt: string  // = now ISO 8601
}
```

Bug-054 reference: previous implementation pushed raw `options.answer` string instead of a `RuleItem` object. The fix (and current TS impl) creates a proper object using `workUnit.nextRuleId++` (post-increment).

### CLI surface (Commander.js)

```ts
program
  .command('answer-question')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<index>', 'Question index (0-based)')
  .option('--answer <answer>', 'Answer text')
  .option('--add-to <type>', 'Add answer to: rule, assumption, or none', 'none')
  .action(...)
```

- 2 required positional args, 2 flag options.
- `--add-to` has default value `'none'`.
- Action calls `answerQuestion({ workUnitId, index: parseInt(index, 10), answer, addTo })`.
- On success: `output.log('✓ Answered question: "<questionText>"')`. If `options.answer`: `output.log('  Answer: "<answer>"')`. If `result.addedTo && result.addedContent`: `output.log(chalk.cyan('  Added to <addedTo>: "<addedContent>"'))`.
- On failure: `output.error(chalk.red('✗ Failed to answer question:'), error.message)` + `process.exit(1)`.

### Help — `src/commands/answer-question-help.ts`

- name: `answer-question`
- description: "Answer a question from Example Mapping and optionally convert to rule or assumption"
- usage: `fspec answer-question <workUnitId> <index> [options]`
- 4 options (--answer, --add-to, --add-to-rules [deprecated], --add-to-assumptions [deprecated])
- 2 examples
- relatedCommands: add-question, add-rule, add-assumption

NOTE: the deprecated `--add-to-rules` / `--add-to-assumptions` flags are documented in `-help.ts` but NOT registered in Commander.js. They're help-doc only — Framing A consideration. The Rust port should mirror the help fixture verbatim but not register those clap flags either (parity with TS Commander.js shell).

### Rust port plan

- Reuse `crate::io::ensure::ensure_work_units_file`, `crate::io::locked_file::write_json_atomic`, `crate::io::time::iso8601_now`.
- `WorkUnit.questions`, `WorkUnit.rules`, `WorkUnit.assumptions`, `WorkUnit.nextRuleId` all live in `extra` (Map<String, Value>).
- Args struct: `AnswerQuestionArgs { workUnitId, index: usize, answer?, addTo? }`. `addTo` is a free-form string with TS-parity values; deserialize as `Option<String>`.
- Status guard: use `WorkUnit.status.as_str() == "specifying"`.
- Question retrieval: `extra["questions"]` as `Value::Array`. Index into it; assert it's `Value::Object`. If absent OR string → error.
- Mutation: edit the question object's `selected`/`answered`/`answer` fields in-place inside the `Value::Object`.
- RuleItem construction: same recipe as `add_rule.rs`. Use `serde_json::Map` to preserve `id, text, deleted, createdAt` order.
- nextRuleId increment: post-increment semantics (read current, then write +1). Default to 0 when missing.
- Assumption push: just `Value::String(answer)`.
- Result JSON: `{success, question, addedTo?, addedContent?}` — only emit `addedTo`/`addedContent` when set (use `#[serde(skip_serializing_if = "Option::is_none")]`).

### Insertion order

The `RuleItem` JSON shape uses ordered map (id, text, deleted, createdAt). The Result JSON uses field declaration order (success, question, addedTo, addedContent).

### Two-front-doors

CLI bridge owns:
- `parseInt(index, 10)` equivalent (clap can do this directly with `usize`).
- Passing `--add-to` default `'none'` (the TS shell defaults it via Commander).
- Output formatting (success message + optional Answer line + optional cyan "Added to" line).

Core owns everything else.
