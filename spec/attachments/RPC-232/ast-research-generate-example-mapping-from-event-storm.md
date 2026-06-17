# AST Research — generate-example-mapping-from-event-storm (RPC-232)

## TS Source
`src/commands/generate-example-mapping-from-event-storm.ts` (213 LOC) + `-help.ts`.

## Behaviour summary
`generateExampleMappingFromEventStorm({ workUnitId, cwd? })` → `{ success, error?, rulesAdded?, examplesAdded?, questionsAdded? }`:
1. `cwd = options.cwd || process.cwd()`. Path = `spec/work-units.json`.
2. If file missing → `{ success:false, error:'spec/work-units.json not found. Run fspec init first.' }` (NO auto-create → Option B).
3. Wraps mutation in `fileManager.transaction(file, async data => {...})`:
   - Validate `workUnits[workUnitId]` exists → else throw `Work unit <id> not found`.
   - Validate `workUnit.eventStorm && workUnit.eventStorm.items` → else throw `Work unit <id> has no Event Storm data`.
   - Init `rules`, `examples`, `questions` arrays if absent.
   - Init `nextRuleId`, `nextExampleId`, `nextQuestionId` to 0 if `undefined` (backward compat).
   - For each non-deleted `eventStorm.items` item:
     - **type==='policy'** with `when && then`: push rule
       `{ id: nextRuleId++, text: "System must <then> after <when>", deleted:false, createdAt:ISO }`
       where `<when>`/`<then>` are `pascalCaseToSentence(...)`. `rulesAdded++`.
     - **type==='event'**: BUG-089 — DISABLED. NO examples generated (commented out). `examplesAdded` stays 0.
     - **type==='hotspot'** with `concern`: BUG-088 — preserve concern text trimmed, append `?` only if not already ending with `?`. Push question
       `{ id: nextQuestionId++, text: "@human: <concern>", deleted:false, answer:undefined, createdAt:ISO }`. `questionsAdded++`.
   - `workUnit.updatedAt = ISO`; `workUnitsData.meta.lastUpdated = ISO` (if meta present).
4. On success → `{ success:true, rulesAdded, examplesAdded, questionsAdded }`.
5. catch → `{ success:false, error: error.message }`.

## CLI registration / output
`.command('generate-example-mapping-from-event-storm').argument('<workUnitId>', 'Work unit ID')`. Single positional, no flags.
On success logs:
`Generated Example Mapping for <id>:\n  Rules added: <n>\n  Examples added: <n>\n  Questions added: <n>`.
On `!success` → `logger.error(error)` then `process.exit(1)`.

## pascalCaseToSentence
```
text.replace(/([A-Z])/g, ' $1').trim().toLowerCase()
```
Inserts a space before EACH uppercase letter, trims, lowercases. e.g. `UserRegistered` → `user registered`, `SendWelcomeEmail` → `send welcome email`.

## Rust port plan
- Option B missing-file (inline `path.exists()`).
- Read `WorkUnitsData`; walk `wu.extra.get("eventStorm")` → object → `items` array (parity with show_event_storm.rs `extra` walk). If missing → `Work unit <id> has no Event Storm data`.
- rules/examples/questions/nextXId all live in `wu.extra` (parity with add_rule.rs / add_example.rs / add_question.rs `extra` access).
- Field order on rule object: `id, text, deleted, createdAt`. Question: `id, text, deleted, answer(=null? or absent), createdAt` — MUST match TS object literal: `{ id, text, deleted, answer:undefined, createdAt }`. NOTE: `answer:undefined` → JSON.stringify OMITS undefined keys, so on-disk the question has NO `answer` key. Verify against add_question.rs output shape (it omits answer). So emit `id, text, deleted, createdAt` only.
- Skip `item.deleted === true`.
- BUG-089: do NOT generate examples (examplesAdded=0).
- BUG-088: hotspot concern trimmed + conditional `?`.
- `pascalCaseToSentence` — small helper; need a shared text-formatting fn. Check if one exists in fspec-core; if not, ASK supervisor for shared `pascal_case_to_sentence` (also reused by other event-storm transforms). Flag.
- Single `write_json_atomic` at end.
- meta.lastUpdated bump — meta lives where? Check WorkUnitsData typed `meta` field or `extra`.

## SHARED-FN REQUEST
`pascalCaseToSentence` helper — flag to supervisor for a shared `crate::text_format::pascal_case_to_sentence` (or inline if trivial + approved).

## Shared types reused
- `crate::types::work_unit::WorkUnitsData`, `WorkUnit.extra`
- `crate::io::locked_file::write_json_atomic`
- `crate::io::time::iso8601_now`
- `crate::error::FspecCoreError`
