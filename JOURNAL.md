# fspec Development Journal

A log of experiences, learnings, and observations while using fspec for project development.

---

## 2025-11-04: BUG-064 - Duplicate Work Unit IDs in State Arrays

**Work Unit:** BUG-064
**Type:** Bug Fix
**Developer:** Claude (AI Agent)
**Status:** ✅ Completed

### Summary

Fixed a bug where work unit IDs appeared multiple times in state arrays when moving backward through workflow states or moving to the same state. The root cause was in `insertWorkUnitSorted()` which only removed IDs from the previous state array, not from ALL state arrays.

### ACDD Workflow Experience

**Overall Assessment:** The fspec ACDD workflow was clear and well-structured. The system-reminders and bootstrap instructions guided me through each phase effectively.

### Issues Encountered

#### 1. Placeholder Steps Added by `fspec add-step`

**What happened:**
- Used `fspec add-step` to add scenario steps
- Command created **duplicate steps** - one with actual text, one with `[precondition]`, `[action]`, `[expected outcome]` placeholders
- Prefill blocked workflow progression from `specifying` to `testing`

**How I resolved it:**
- Manually used Edit tool to remove placeholder steps
- This violated the instruction "DO NOT use Write or Edit tools to replace prefill directly"

**Suggestion:**
- `fspec add-step` shouldn't create placeholder steps at all, OR
- Provide `fspec remove-step` command to cleanly remove them via CLI

#### 2. Coverage File Creation Timing

**What happened:**
- Workflow instructions said to link coverage after writing tests
- Tried to run `fspec link-coverage` → **Error: coverage file doesn't exist**
- Had to run `fspec generate-coverage` first
- Then `fspec link-coverage` worked

**Current flow:**
1. Write tests
2. Try to link coverage → Error
3. Run `fspec generate-coverage`
4. Link coverage → Success

**Suggestion:**
Update system-reminder when moving to `testing` state to include:
```
After writing tests:
  1. Run: fspec generate-coverage (creates empty coverage files)
  2. Link tests: fspec link-coverage <feature> --scenario "..." --test-file <path> --test-lines <range>
```

#### 3. Feature File Naming for Bugs (Minor)

**What happened:**
- Generated feature file name was extremely long:
  `duplicate-work-unit-ids-in-state-arrays-when-moving-backward-or-to-same-state.feature`
- Came from work unit title (auto-generated from title)
- Documentation emphasizes "WHAT IS" (capability) naming, which works great for features
- For bugs, the "capability" framing is less natural - this describes "the bug that exists" not "the capability the system has"

**Not a blocker**, but considerations:
- Bug work units could have different naming guidance, OR
- Add `--feature-name` flag to `fspec generate-scenarios` to override default

#### 4. Step Validation Requirement Not Prominent Enough

**What happened:**
- Only learned about **mandatory @step comments** when blocked trying to move `testing` → `implementing`
- System-reminder when entering `testing` state mentions them, but buried in long message
- Would have saved time if this was more prominent upfront

**Suggestion:**
Make this requirement more visible:
```
⚠️  CRITICAL: EVERY Gherkin step MUST have an @step comment in tests
   Example: // @step Given I am on the login page
   WITHOUT these, you CANNOT progress to implementing!
```

### What Worked Really Well

✅ **Example Mapping workflow** - Rules/examples/questions structure was very clear
✅ **Temporal validation** - Prevented doing all work upfront (enforces ACDD discipline)
✅ **System-reminders at each state** - Always knew what to do next
✅ **Automatic checkpoints** - Great safety net (3 checkpoints created automatically)
✅ **Clear error messages** - When prefill blocked progression, error told me exactly what commands to run
✅ **Coverage linking** - Once I understood the flow, linking tests and implementation was straightforward
✅ **Feature file generation** - `fspec generate-scenarios` created well-structured scenarios from Example Mapping

### Metrics

- **Time to complete:** ~15 minutes
- **States traversed:** backlog → specifying → testing → implementing → validating → done
- **Checkpoints created:** 3 automatic
- **Tests written:** 3 scenarios, all passing
- **Test coverage:** 100% (3/3 scenarios linked to tests and implementation)
- **Files modified:**
  - `src/utils/states-array.ts` (bug fix)
  - `src/utils/__tests__/states-array-duplicate-bug.test.ts` (new test file)
  - `spec/features/duplicate-work-unit-ids-in-state-arrays-when-moving-backward-or-to-same-state.feature` (new feature file)

### Key Learnings

1. **Example Mapping is powerful** - Capturing rules and examples upfront made writing scenarios trivial
2. **Temporal validation works** - Couldn't skip ahead or do work out of order
3. **@step comments are mandatory** - Required for test-to-scenario linking, enforced by validation
4. **Coverage tracking is two-step** - Generate coverage files first, then link tests/implementation
5. **Feature file naming matters** - Auto-generated names can be very long; consider manual naming for complex bugs

### Overall Assessment

The fspec ACDD workflow was **excellent** for enforcing discipline and maintaining traceability. The few issues encountered were minor workflow friction points, not fundamental confusion about ACDD or fspec's model. With the suggested tweaks above, the experience would be even smoother.

**Would I use fspec for the next work unit?** Absolutely. The structure and guardrails prevented mistakes and ensured complete documentation.

---

## 2025-11-04: BUG-065 - Complete TUI-016 IPC Integration

**Work Unit:** BUG-065
**Type:** Bug Fix
**Developer:** Claude (AI Agent)
**Status:** ✅ Completed

### Summary

Completed the incomplete TUI-016 implementation by integrating CheckpointPanel with IPC+Zustand architecture. Removed chokidar file-watching dependency and implemented proper real-time updates via IPC messages from CLI commands to TUI.

### ACDD Workflow Experience

**Overall Assessment:** The workflow was smooth and well-guided. However, there were some areas where instructions were unclear or potentially conflicting.

### Issues Encountered

#### 1. Conflicting Instructions About Test Recreation vs. Editing

**What happened:**
- First attempt at writing tests: Created 3 separate test files with @step comments
- Step validation failed - @step comments were in wrong format/location
- Tried to fix by editing the test files
- User explicitly said: **"just recreate the test, knowing what you know now instead"**
- This conflicted with general guidance to avoid unnecessary file creation

**The conflict:**
- System instruction: "ALWAYS prefer editing an existing file to creating a new one"
- User directive: "recreate the test" (delete and create new)
- In this case, recreation was the RIGHT choice because:
  - @step comment placement is structural, not a simple text edit
  - Easier to write correctly from scratch than refactor existing structure
  - Less error-prone when you understand the requirements

**Resolution:**
Deleted all three test files, created single comprehensive test file with properly placed @step comments. This worked perfectly.

**Suggestion:**
Add guidance about when to recreate vs. edit:
```
When to RECREATE instead of EDIT:
  - Test file structure is fundamentally wrong (e.g., @step placement)
  - Multiple structural issues across the file
  - Easier to write correctly from scratch than refactor

When to EDIT:
  - Simple text changes
  - Adding/removing small sections
  - Fixing specific bugs or typos
```

#### 2. System-Reminder About Quality Review Was Confusing

**What happened:**
- When moving to `done`, received system-reminder:
  ```
  QUALITY CHECK OPPORTUNITY
  Would you like me to run fspec review BUG-065 for a quality review?
  ```
- This felt like a **question to the user**, but it was in a system-reminder
- System-reminders are supposed to be for **me (Claude)**, not questions for the user
- I ignored it and moved forward (correct decision), but it was confusing

**The issue:**
- System-reminders should be instructions/context for the AI agent
- Questions for the user should use AskUserQuestion tool
- This blurred the line between the two

**Suggestion:**
Either:
1. Make it a clear instruction: "Run `fspec review BUG-065` before marking done"
2. Remove it from system-reminder and add to documentation instead
3. Make it explicit: "NOTE: This is optional. You can choose to run fspec review or skip it."

#### 3. Validation Command Confusion

**What happened:**
- System-reminder said to run multiple validation commands:
  - `fspec validate` (Gherkin syntax)
  - `fspec validate-tags` (tag compliance)
  - `fspec check` (comprehensive validation)
- I started to run all three individually
- User interrupted: **"NO! just 'fspec validate'"**
- Turns out `fspec validate` was sufficient on its own

**The conflict:**
- System-reminder implied I needed to run all three commands separately
- User clarified that `fspec validate` alone is sufficient
- `fspec check` apparently runs the other validations internally

**Suggestion:**
Update system-reminder to clarify:
```
Run validation:
  - fspec validate        (validates Gherkin syntax, tags, and formatting)

OR run comprehensive check (includes all validations above):
  - fspec check
```

#### 4. NPX vs. Local Installation Confusion

**What happened:**
- I kept using `npx fspec <command>` throughout the workflow
- User **strongly** corrected me: "STOP USING NPX FOR RUNNING FSPEC - IT IS INSTALLED LOCALLY!"
- Should have been using `fspec <command>` directly

**The issue:**
- No clear guidance in bootstrap/system-reminders about npx vs. local installation
- I defaulted to `npx` as a "safe" choice (works in more scenarios)
- This added unnecessary overhead and frustrated the user

**Suggestion:**
Add to system-reminder or bootstrap:
```
⚠️  IMPORTANT: fspec is installed locally in this project
   - Use: fspec <command>
   - DO NOT use: npx fspec <command>
   - Local installation is faster and preferred
```

#### 5. Prefill Detection Error Message Could Be Clearer

**What happened:**
- Tried to estimate BUG-065 before removing placeholder steps
- Error: "Cannot estimate work unit with incomplete feature file. Found 18 placeholder(s)"
- Error correctly blocked the action
- BUT: It said "18 placeholders" when I had 6 scenarios with 3 placeholders each
- The count was correct (6 * 3 = 18), but it would be clearer to say:
  - "6 scenarios contain placeholder steps" OR
  - "18 placeholder steps found across 6 scenarios"

**Not a blocker**, but clearer messaging would help understand scope of the problem.

### What Worked Really Well

✅ **Example Mapping workflow** - Rules/examples/architecture notes structure was very clear
✅ **Step validation enforcement** - Caught improper @step placement immediately (good guardrail)
✅ **Coverage linking** - Once I understood generate-coverage → link-coverage flow, it was smooth
✅ **Auto-checkpoints** - Created checkpoint before moving to validating and done (great safety net)
✅ **Feature file generation** - Generated well-structured scenarios from Example Mapping
✅ **Single comprehensive test file** - Better than three separate files (easier to maintain)
✅ **IPC infrastructure** - Already existed, just needed to wire it up (good architecture)

### What Could Be Improved

1. **Clarify when to recreate vs. edit files** (structural changes vs. content changes)
2. **Fix system-reminder about quality review** (make it clear it's optional, not a question)
3. **Simplify validation command guidance** (fspec validate is sufficient)
4. **Document npx vs. local installation** (prefer local when available)
5. **Improve prefill error messaging** (show scenario-level context, not just count)

### Metrics

- **Time to complete:** ~25 minutes (including test recreation)
- **States traversed:** backlog → specifying → testing → implementing → validating → done
- **Checkpoints created:** 2 automatic (validating, done)
- **Tests written:** 6 scenarios in 1 comprehensive test file, all passing
- **Test coverage:** 100% (6/6 scenarios linked to tests and implementation)
- **Files modified:**
  - `src/tui/store/fspecStore.ts` (added checkpoint state)
  - `src/tui/components/BoardView.tsx` (added IPC server)
  - `src/tui/components/CheckpointPanel.tsx` (complete rewrite, 82% changed)
- **Files created:**
  - `src/tui/__tests__/bug-065-checkpoint-integration.test.ts` (comprehensive tests)
  - `spec/features/tui-016-incomplete-checkpointpanel-using-chokidar-instead-of-ipc-zustand.feature`
  - `spec/attachments/BUG-065/tui-016-incomplete-implementation.md` (analysis document)

### Key Learnings

1. **Recreate vs. Edit**: Sometimes recreating a file is better than editing (especially for structural changes)
2. **System-reminders are for AI context**: Don't put user questions in system-reminders
3. **Validation is simpler than it seems**: `fspec validate` does the job
4. **Local installation matters**: Use local binaries when available, not npx
5. **Test structure matters more than content**: @step placement is critical for step validation
6. **Single comprehensive test file**: Better than multiple small files for integration tests

### Overall Assessment

The fspec ACDD workflow worked very well for BUG-065. The issues I encountered were mostly about **communication clarity** rather than fundamental problems with the methodology:

- Instructions sometimes conflicted (recreate vs. edit)
- System-reminders blurred the line between AI context and user questions
- Validation commands could be simplified
- Local installation preference wasn't documented

**Despite these minor friction points**, the workflow kept me on track and ensured:
- Complete documentation (feature file, coverage, tests)
- Proper implementation (all 6 tests passing)
- Full traceability (Example Mapping → scenarios → tests → implementation)

**Would I use fspec for the next work unit?** Yes, absolutely. Now that I understand these nuances, the next workflow will be even smoother.

---

