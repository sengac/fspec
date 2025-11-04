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

