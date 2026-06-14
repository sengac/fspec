# AST Research — `validate-work-units` (RPC-325)

## TS source
- `src/commands/validate-work-units.ts` — `validateWorkUnits({cwd})` + `registerValidateWorkUnitsCommand`.
- Help: `validate-work-units-help.ts` (HAS custom help -> needs help config module).

## Behaviour summary
- Loads `spec/work-units.json` via `ensureWorkUnitsFile(cwd)` (AUTO-CREATES if missing, escalates parse error). Returns `{ valid, checks[], errors? }`.
- `checks` array records check NAMES executed (in push order): schema, uniqueIds, parentChild, exampleMapping, dependencies. (state-value + state-index checks do NOT push a name.)
- `errors[]` collected; `valid = errors.length===0`. errors field only present when errors.length>0.

### Check 1: schema (push 'schema')
- workUnits missing/not object -> 'Invalid work units data structure: missing or invalid workUnits field'
- states missing/not object -> 'Invalid work units data structure: missing or invalid states field'

### Check 2: uniqueIds (push 'uniqueIds')
- Object.keys(workUnits) length vs Set size — duplicates (cannot happen in JS object, parity only) -> 'Duplicate work unit IDs detected'

### Check 3: parentChild (push 'parentChild')
For each [id, wu]:
- wu.parent set & not in workUnits -> 'Work unit <id> references non-existent parent: <parent>'
- wu.parent set & parent.children missing/not-includes id -> 'Work unit <id> has parent <parent>, but parent doesn't list it as a child'
- wu.children: for each childId: not in workUnits -> 'Work unit <id> references non-existent child: <childId>'; else if child.parent !== id -> 'Work unit <id> lists <childId> as child, but child doesn't have it as parent'

### Check 4: valid state values (NO push)
allowedStates = backlog, specifying, testing, implementing, validating, done, blocked.
- wu.status not in allowedStates -> 'Invalid status value for <id>: <status>. Allowed values: <list joined ', '>'

### Check 5: state index consistency (NO push)
For each [id, wu]:
- stateArray = states[status]; if missing or !includes(id) -> 'State consistency error: Work unit <id> has status '<status>' but is not in states.<status> array. Run 'fspec repair-work-units' to fix inconsistencies.'
- for each [stateName, stateIds] in states: if stateName !== status && stateIds.includes(id) -> 'State consistency error: Work unit <id> has status '<status>' but is in '<stateName>' array. Run 'fspec repair-work-units' to fix inconsistencies.'

### Check 6: exampleMapping (push 'exampleMapping')
For each wu:
- rules: not array -> '<id>: rules must be an array'; else each non-string or empty-trim at index i -> '<id>: rules array contains empty strings or non-strings at index <i>'
- examples: same pattern
- questions: not array -> '<id>: questions must be an array'; each must be object (not null) else '<id>: questions[<i>] must be a QuestionItem object with {text, selected, answer?}, got <typeof>'; text non-empty string else '<id>: questions[<i>].text must be a non-empty string'; selected boolean else '<id>: questions[<i>].selected must be a boolean'; answer if defined must be non-empty string else '<id>: questions[<i>].answer must be a non-empty string if provided'
- assumptions: array-of-non-empty-strings pattern -> '<id>: assumptions array contains empty strings or non-strings at index <i>'

### Check 7: dependencies (push 'dependencies')
For each wu: blocks, blockedBy, dependsOn, relatesTo — each must be array; each entry non-empty string. Messages:
- '<id>: blocks must be an array' / '<id>: blocks array contains empty strings or non-strings at index <i>'
- same for blockedBy, dependsOn, relatesTo

### CLI registration
- NO arguments, NO options (help mentions --fix but registration does NOT implement it — help doc is aspirational/canon under Framing A? Actually registration has no .option). So clap exposes NO flags.
- Action: validateWorkUnits({}); if valid -> output.log('✓ All work units are valid'); else output.error(chalk.red('✗ Found N validation errors')) + each '  - <error>' to stderr, process.exit(1). Catch -> output.error('✗ Failed to validate work units:', msg), exit 1.

## Output contract (CLI)
- valid: stdout '✓ All work units are valid', exit 0.
- invalid: stderr '✗ Found <N> validation errors' then '  - <error>' per error, exit 1.
- exception: stderr '✗ Failed to validate work units: <msg>', exit 1.

## Help (validate-work-units-help.ts)
- name validate-work-units; description 'Validate work unit data integrity and relationships'; usage 'fspec validate-work-units [options]'.
- options: [{flag:'--fix', description:'Automatically fix issues where possible'}]  <-- NOTE: help documents --fix but command does NOT implement it.
- whenToUse, one example (output '✓ All work units valid\n  Checked 42 work units' — NOTE diverges from actual CLI output '✓ All work units are valid'), relatedCommands [repair-work-units, check].
- HAS custom help -> needs help config module.

## DECISION NEEDED (Framing A): help advertises --fix flag + example output that the actual CLI does NOT produce. Per command-port.md §10 Framing A "help doc is canon" applies when TS CLI is broken because action discards result. Here action is NOT broken — it just doesn't implement --fix. Help fixture is captured from `node dist/index.js validate-work-units --help` so the fixture is canon for the --help text regardless. But the clap surface: should --fix exist? TS registration has NO .option('--fix'), so Commander.js help would NOT show --fix... yet validate-work-units-help.ts (rich help) lists it. The byte-exact fixture decides. CAPTURE FIXTURE FIRST in PHASE B to resolve. Plan: clap exposes NO functional flags (parity with registration). If fixture shows --fix in OPTIONS we still only render help text; the flag is accepted-and-ignored OR we add a no-op --fix to clap. RESOLVE with supervisor after fixture capture.

## Rust wiring intent
- Reuse `ensure_work_units_file` (auto-create + escalate parse). WorkUnitsData typed: version, meta, work_units (IndexMap), states (WorkUnitStates), extra. work_units status is a typed enum WorkUnitStatus — invalid status values would FAIL deserialization in Rust (TS keeps raw string). PARITY RISK: Check 4 (invalid status value) cannot trigger if Rust rejects bad status at parse. Need to read status from `extra`/raw Value to mirror TS tolerance, OR accept that ensure_work_units_file parse-errors on bad status. REQUEST to supervisor: confirm strategy — likely parse work-units.json as raw serde_json::Value in this command (NOT typed WorkUnitsData) so all 7 checks mirror TS's untyped JSON.parse exactly. The ad-hoc fields (rules, examples, questions, assumptions, blocks, blockedBy, dependsOn, relatesTo, parent, children) all live in `extra` anyway.
- DECISION: implement against raw serde_json::Value (mirror TS dynamic access) rather than typed WorkUnitsData, since every check inspects untyped/ad-hoc fields and TS never narrows. ensure_work_units_file still used to get the load-or-init + parse-escalation, then re-serialize to Value — OR add a raw loader. Will model in-command by serializing the loaded data back to Value, but bad-status would already have failed. SHARED-FILE REQUEST: add io helper to load work-units.json as raw Value with ensure semantics (auto-create default, escalate parse). Propose io::ensure::ensure_work_units_value(cwd) -> Result<Value>.

## Files
- core impl codelet/fspec-core/src/commands/validate_work_units.rs
- help config codelet/fspec-core/src/help/configs/validate_work_units.rs
- CLI bridge codelet/fspec/src/validate_work_units.rs
- core test codelet/fspec-core/tests/validate_work_units.rs
- CLI test codelet/fspec/tests/cli_validate_work_units.rs
- help fixture codelet/fspec/tests/fixtures/help/validate-work-units.txt
