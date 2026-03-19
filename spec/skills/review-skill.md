# fspec Review — Epic-Wide ACDD Compliance Review

Deep-review an epic's parent story and all child work units for ACDD compliance, code quality, and spec alignment. Uses parallel subordinate agents for review, then fixes issues sequentially.

## How to Use This Skill

Reference this file with `@` in your prompt:

```
@spec/skills/review-skill.md SCHED-001          # Review epic starting from parent SCHED-001
@spec/skills/review-skill.md PROV-001 --dry-run  # Review only, don't fix anything
```

---

## Execute Now

When this skill is activated, run the full review workflow below against the work unit ID provided by the user. If `--dry-run` is specified, produce findings but do not apply fixes.

---

## Phase 1: Discovery — Build the Review Plan

### Step 1.1: Load the Parent Work Unit

```
Fspec: show-work-unit <PARENT-ID>
```

Extract:
- Work unit ID, title, type, status, description
- Example map: rules, examples, questions, assumptions
- Architecture notes
- Attachments (especially `ast-research-*` files)
- Linked feature files (from tags or coverage)
- Child work units (from `parent` field queries)

### Step 1.2: Find All Child Work Units

```
Fspec: list-work-units
```

Filter for work units whose `parent` field matches the parent ID. Also check the epic field if children use the same epic.

### Step 1.3: Determine Review Order

Sort children by dependency order:
1. Check `Fspec: dependencies <CHILD-ID>` for each child
2. Topologically sort: dependencies first, dependents last
3. If no explicit dependencies, sort by work unit ID (numeric suffix)

Build the **review manifest** — an ordered list:

```
Review Order:
1. <PARENT-ID> (parent story)
2. <CHILD-001> (no dependencies)
3. <CHILD-002> (depends on CHILD-001)
4. <CHILD-003> (depends on CHILD-001)
5. ...
```

### Step 1.4: Gather File Inventory Per Work Unit

For each work unit in the manifest, collect:
- **Feature file(s)**: from `spec/features/` matching the `@WORK-UNIT-ID` tag
- **Test file(s)**: from coverage data (`Fspec: show-coverage <feature-name>`)
- **Implementation file(s)**: from coverage data and architecture notes
- **Attachment(s)**: from `spec/attachments/<WORK-UNIT-ID>/`

---

## Phase 2: Parallel Review — Spawn Worker Agents

### Step 2.1: Spawn Review Workers

For each work unit in the manifest, spawn a subordinate agent:

```
AgentManager(action='spawn', role='You are an ACDD compliance reviewer for the fspec project. You perform deep code review against Gherkin feature files, example maps, and coding standards. You report findings in a structured format. You do NOT fix code — you only identify issues.')
```

### Step 2.2: Send Review Tasks

Send each worker its review task via `AgentManager(action='message')`. The message must include:

```
Review work unit <WORK-UNIT-ID>: "<TITLE>"

## Your Review Checklist

Perform ALL of the following checks. Use DeepSearch and Read to examine every file thoroughly. Do NOT skip any check.

### A. Feature File Compliance
1. Read the feature file: <feature-file-path>
2. Verify every scenario has correct Given/When/Then ordering
   - Given steps set up preconditions (MUST come before When)
   - When steps describe actions (exactly one per scenario ideally)
   - Then steps assert outcomes (MUST come after When)
   - And/But after Then = additional assertions, NOT preconditions
3. Check for placeholder text: [role], [action], [benefit], etc.
4. Verify architecture doc string is present and accurate
5. Verify @WORK-UNIT-ID tag is present on the feature

### B. Example Map Alignment
Use Fspec tool to show the work unit: Fspec show-work-unit <WORK-UNIT-ID>
1. Every rule in the example map should be reflected in at least one scenario
2. Every example in the example map should map to a scenario
3. No unanswered questions should remain (red cards)
4. Architecture notes should match the actual implementation approach

### C. Test Coverage Compliance
1. Read each test file: <test-file-paths>
2. Every Gherkin scenario MUST have a corresponding test
3. Every test MUST have @step comments matching the Gherkin steps exactly
4. @step comment text must match the feature file step text (not paraphrased)
5. Tests must actually test what the scenario describes (not trivial assertions)
6. Check Fspec coverage: Fspec show-coverage <feature-name>
   - All scenarios must be linked
   - Test file line ranges must point to actual test code
   - Implementation file line ranges must point to actual implementation

### D. Implementation Quality
Read each implementation file: <impl-file-paths>
1. **SOLID principles:**
   - Single Responsibility: each function/module has one job
   - Open/Closed: extensible without modification
   - Liskov: subtypes substitutable
   - Interface Segregation: no fat interfaces
   - Dependency Inversion: depend on abstractions
2. **DRY:** Search for duplicate logic across the codebase
   - Use DeepSearch to find functions with similar names
   - Use DeepSearch to find duplicated patterns
3. **No shortcuts:** Search for TODO, FIXME, HACK, XXX, unimplemented!(), todo!()
4. **No half-written code:** All code paths must be complete
5. **Wired up end-to-end:** Trace WHO CALLS the new code
   - Is it reachable from a user action or entry point?
   - Are all integration points connected?
6. **Type safety:** No `any` types (TypeScript), no `as unknown as` casts
7. **Error handling:** All async operations have try/catch or .catch()
8. **File size:** No file over 300 lines
9. **Import style:** No file extensions in TS imports, no require(), type-only imports use `import type`

### E. Build & Test Verification
1. Check if tests pass: run `npm test -- --reporter=verbose <test-file>` for TypeScript or `cargo test <test-name>` for Rust
2. If Rust code: run `cargo build` in the relevant crate directory
3. If TypeScript code: run `npm run build` to verify compilation

### F. Cross-Cutting Concerns
1. Are there any functions/types that should be shared but are duplicated?
2. Does the implementation match the architecture notes from specifying?
3. Are there any security concerns (unsanitized input, exposed secrets)?
4. Are there any performance concerns (unbounded loops, missing pagination)?

## Output Format

Report your findings in EXACTLY this format:

---
# Review: <WORK-UNIT-ID> — <TITLE>

## Status: <PASS | FAIL | WARN>

## 🔴 Critical Issues (Must Fix)
<numbered list, or "None">

## 🟡 Warnings (Should Fix)
<numbered list, or "None">

## 🟢 Observations (Nice to Have)
<numbered list, or "None">

## Coverage Verification
- Feature file: <path> — <OK | ISSUE: description>
- Test file(s): <paths> — <OK | ISSUE: description>
- Impl file(s): <paths> — <OK | ISSUE: description>
- Scenario coverage: <N/M scenarios covered>

## Files Reviewed
<list of every file you read>
---
```

### Step 2.3: Collect Results

Poll each worker with `AgentManager(action='get_status')` until all are complete. Then read each worker's findings from their last message.

```
AgentManager(action='get_status', session_id='<worker-id>')
```

When status shows idle (response generated), the review is done. Read the response.

### Step 2.4: Close Workers

```
AgentManager(action='close', session_id='<worker-id>')
```

---

## Phase 3: Consolidate Findings

### Step 3.1: Aggregate All Findings

Merge all worker reports into a single document organized by work unit (in review order). Create a consolidated findings file:

```
Write: spec/attachments/<PARENT-ID>/review-findings.md
```

Format:

```markdown
# Epic Review: <PARENT-ID> — <PARENT-TITLE>

**Date:** <ISO date>
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** <count>

## Summary
- 🔴 Critical: <count> issues across <count> work units
- 🟡 Warnings: <count> issues across <count> work units
- 🟢 Observations: <count>

## Work Unit Results

### <WORK-UNIT-ID>: <TITLE> — <PASS|FAIL|WARN>
<findings from worker>

### <NEXT-WORK-UNIT-ID>: ...
...
```

### Step 3.2: Attach to Parent Work Unit

```
Fspec: add-attachment <PARENT-ID> spec/attachments/<PARENT-ID>/review-findings.md --description "Epic-wide ACDD compliance review"
```

### Step 3.3: If Dry Run, Stop Here

If `--dry-run` was specified, print the consolidated findings and stop. Do not proceed to Phase 4.

---

## Phase 4: Fix Issues (Sequential)

Process fixes **one work unit at a time**, in review order. For each work unit with 🔴 Critical or 🟡 Warning issues:

### Step 4.1: Move Work Unit Backward

If the work unit is in `done` or `validating`:

```
Fspec: update-work-unit-status <WORK-UNIT-ID> implementing
```

### Step 4.2: Fix Each Issue

Work through issues in priority order (🔴 first, then 🟡):

**Feature file issues** (step ordering, missing tags, placeholder text):
- Read the current feature file
- Rewrite with correct Given/When/Then ordering
- Validate: `Fspec: validate`

**Test issues** (missing @step comments, wrong step text, missing scenarios):
- Read the current test file
- Fix @step comments to match feature file exactly
- Add missing test scenarios
- Run tests to verify they pass

**Implementation issues** (DRY violations, missing error handling, type safety):
- Read the implementation file
- Apply the fix
- Run tests to verify nothing broke
- Run build to verify compilation

**Coverage issues** (wrong line ranges, missing links):
- Unlink incorrect coverage: `Fspec: unlink-coverage`
- Re-link with correct line ranges: `Fspec: link-coverage`

### Step 4.3: Re-validate

After all fixes for a work unit:

1. Run the relevant tests
2. Run the build
3. Verify coverage: `Fspec: show-coverage <feature-name>`
4. Validate feature file: `Fspec: validate`
5. Validate tags: `Fspec: validate-tags`

### Step 4.4: Advance Work Unit

```
Fspec: update-work-unit-status <WORK-UNIT-ID> validating
Fspec: update-work-unit-status <WORK-UNIT-ID> done
```

### Step 4.5: Move to Next Work Unit

Repeat Steps 4.1–4.4 for the next work unit with issues.

---

## Phase 5: Final Verification

### Step 5.1: Run Full Test Suite

```bash
npm test          # All TypeScript tests
cargo test        # All Rust tests (if applicable)
npm run build     # TypeScript compilation
```

### Step 5.2: Update Review Findings

Update the review-findings.md attachment with fix results:

```markdown
## Fix Results

### <WORK-UNIT-ID>: <TITLE>
- 🔴 Issue 1: <description> → ✅ Fixed: <what was done>
- 🟡 Issue 2: <description> → ✅ Fixed: <what was done>

### <NEXT-WORK-UNIT-ID>: ...
...

## Final Verification
- All tests pass: ✅
- Build succeeds: ✅
- Coverage complete: ✅
- Feature files valid: ✅
- Tags valid: ✅
```

### Step 5.3: Report to User

Print a summary table:

```
┌─────────────┬─────────────────────────────────┬────────┬───────────┐
│ Work Unit   │ Title                           │ Status │ Issues    │
├─────────────┼─────────────────────────────────┼────────┼───────────┤
│ SCHED-001   │ Scheduled Workflow Automation    │ ✅ PASS │ 0 fixed   │
│ SCHED-002   │ Schedule Persistence & Schema    │ ✅ PASS │ 2 fixed   │
│ SCHED-003   │ Core Scheduler Engine           │ ⚠️ WARN │ 1 fixed   │
│ ...         │ ...                             │ ...    │ ...       │
└─────────────┴─────────────────────────────────┴────────┴───────────┘
```

---

## Worker Agent Role Template

Use this exact role when spawning review workers:

```
You are an ACDD compliance reviewer for the fspec project.

Your job is to deeply review a single work unit against its feature file, example map, and the project's coding standards. You use DeepSearch and Read tools extensively to examine every file.

## Critical Standards You Enforce

### Gherkin Quality
- Given/When/Then steps must be correctly ordered
- Preconditions MUST be Given steps, not And-after-Then
- Every scenario must be testable and specific
- Architecture doc strings must be present

### Test Quality
- Every scenario needs a test with @step comments
- @step text must EXACTLY match the Gherkin step text
- Tests must verify actual behavior, not trivial assertions
- Test file header must reference the feature file

### Code Quality (TypeScript)
- No `any` types — use proper types always
- No `as unknown as` — use type guards or generics
- No `require()` — only ES6 imports
- No file extensions in imports
- No `console.log` in source (only chalk for CLI output)
- No `var` — only const/let
- No `==` or `!=` — only `===` and `!==`
- Always use curly braces for if/else
- Use `interface` not `type` for object shapes
- Use `import type` for type-only imports
- All promises must be awaited or voided
- No unused variables
- Files under 300 lines

### Code Quality (Rust)
- No unwrap() in production code (use ? or expect with message)
- No todo!() or unimplemented!() in production code
- Proper error handling with Result types
- No dead code or unused imports

### ACDD Compliance
- Example map rules → scenarios → tests → implementation (traceable chain)
- Coverage links must point to correct line ranges
- Architecture notes must match actual implementation
- No unanswered questions in example map

You report findings in a structured format. You do NOT fix code — you only identify issues with specific file paths and line numbers.
```

---

## Handling Edge Cases

### Parent Has No Children
Review just the parent work unit. Skip dependency sorting.

### Work Unit Has No Feature File
Flag as 🔴 Critical: "No feature file found with @WORK-UNIT-ID tag"

### Work Unit Is a Task (Not Story/Bug)
Tasks don't require feature files or example maps. Review only:
- Implementation quality (D)
- Build verification (E)
- Cross-cutting concerns (F)

### Work Unit Is a Bug
Bugs have relaxed requirements:
- Example map is optional (rules/examples may be sparse)
- AST research attachments are optional
- Still require feature file, tests, and implementation quality

### Worker Agent Fails or Times Out
If a worker doesn't respond after extended time:
1. Close the stuck worker
2. Spawn a replacement
3. Re-send the review task
4. If it fails again, perform that review directly with DeepSearch

### Too Many Children (>8 work units)
Batch workers in groups of 4 to avoid overloading:
1. Spawn and complete workers for batch 1 (first 4)
2. Close batch 1 workers
3. Spawn and complete workers for batch 2 (next 4)
4. Continue until all reviewed
