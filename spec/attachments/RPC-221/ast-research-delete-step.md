# AST Research — delete-step (RPC-221)

## TS source: `src/commands/delete-step.ts`

Exported API:
- `deleteStep(options: { feature, scenario, step, cwd? }): Promise<{ success, message?, error? }>`
- `deleteStepCommand(feature, scenario, step)` — Commander action (exit 0/1)
- `registerDeleteStepCommand(program)` — `delete-step <feature> <scenario> <step>`

NOTE: the registered command takes `<step>` text (NOT an index — the help file's `<index>` argument doc is WRONG; the actual action signature is `(feature, scenario, step)` matching by text). Mirror the ACTUAL runtime behaviour (match by step text), keep help fixture byte-exact to TS anyway.

## Behaviour walk-through

1. **Path resolution** (27-35): identical to delete-scenario.
2. **Read** (38-49): ENOENT → `error="Feature file not found: <absPath>"`; other IO errors throw.
3. **Parse** (52-72): parse error → `error="Invalid Gherkin syntax: <msg>"`; no feature → `error="Feature file does not contain a valid Feature"`.
4. **Find scenario** (75-84): `child.scenario.name === scenario`. Not found → `error="Scenario '<scenario>' not found in feature file"`.
5. **Find step** (87-99): match `s.text === step` OR `(s.keyword + s.text).trim() === step.trim()`. Not found → `error="Step '<step>' not found in scenario '<scenario>'"`.
   - In gherkin-0.16, `Step.keyword` includes trailing space (e.g. `"Given "`), `Step.value` is the text. So `keyword + value` = full step line.
6. **Remove line** (102-112): `lineIndex = stepLine - 1`; remove single line.
7. **Collapse blank runs** (115-127): allow ≤ 2 consecutive empty.
8. **Re-validate** (132-142): re-parse; error → `error="Deletion would result in invalid Gherkin: <msg>"`.
9. **Write** (145): write newContent.
10. **Success** (147-151): message = `Successfully deleted step from scenario '<scenario>' in <fileName>`.

NO coverage update for delete-step.

## Rust mapping

- Reuse `parse_feature_lenient`.
- `Step.position.line` (1-based), `Step.keyword`, `Step.value`.
- Match logic: `step.value == arg || format!("{}{}", step.keyword, step.value).trim() == arg.trim()`.
- Error/success JSON same envelope shape as delete-scenario.

## Shared-file needs
- None. dispatch arm `commands::delete_step::run(args_json)` → 2-arg `(args_json, project_root)` (supervisor wiring).
