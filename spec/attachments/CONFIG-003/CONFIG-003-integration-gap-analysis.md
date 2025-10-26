# CONFIG-002 Integration Gap Analysis

**Work Unit:** CONFIG-003
**Related Feature:** CONFIG-002 (Conversational Test and Quality Check Tool Detection)
**Status:** Complete implementation exists, but not integrated into workflow
**Date:** 2025-10-27

---

## Executive Summary

The conversational tool detection feature (CONFIG-002) is **fully implemented and tested** but **not integrated into the actual workflow**. The check functions that emit system-reminders to guide AI agents exist but are never called, rendering the feature non-functional in practice.

---

## The Gap: What Exists vs. What's Missing

### ✅ What EXISTS (Implemented & Tested)

1. **`fspec configure-tools` Command**
   - Location: `src/commands/configure-tools.ts:177-214`
   - Registers command with Commander.js
   - Options: `--test-command`, `--quality-commands`, `--reconfigure`
   - Stores config in `spec/fspec-config.json`
   - **Status:** ✅ Works perfectly

2. **`checkTestCommand()` Function**
   - Location: `src/commands/configure-tools.ts:32-94`
   - Checks if `spec/fspec-config.json` has `tools.test.command` configured
   - Emits system-reminder if missing: "No test command configured. Use Read/Glob to detect..."
   - Emits system-reminder if present: "Run tests: <command>"
   - **Status:** ✅ Implemented, tested, exported

3. **`checkQualityCommands()` Function**
   - Location: `src/commands/configure-tools.ts:96-127`
   - Checks if `spec/fspec-config.json` has `tools.qualityCheck.commands` configured
   - Emits system-reminder with chained commands
   - **Status:** ✅ Implemented, tested, exported

4. **Test Coverage**
   - Location: `src/commands/__tests__/configure-tools.test.ts`
   - 13 scenarios covered (100% coverage per .coverage file)
   - All tests passing
   - **Status:** ✅ Complete

5. **Placeholder Replacement Logic**
   - Location: `src/utils/templateGenerator.ts:153-185`
   - Replaces `<test-command>` and `<quality-check-commands>` in generated docs
   - Reads from `spec/fspec-config.json`
   - **Status:** ✅ Exists (but may not be fully active)

### ❌ What's MISSING (Not Integrated)

1. **No Call Site in Workflow**
   - File: `src/commands/update-work-unit-status.ts`
   - **Problem:** Never imports or calls `checkTestCommand()` or `checkQualityCommands()`
   - **Impact:** AI never receives system-reminders during workflow transitions
   - **Expected Behavior:** When work unit moves to `validating`, check functions should run

2. **Placeholders Still in spec/CLAUDE.md**
   - File: `spec/CLAUDE.md`
   - Lines with `<test-command>`: 525, 532, 1126, 1235, 1589, 1619, 1658, 1665, 1724, 1734, 2011, 2022
   - Lines with `<quality-check-commands>`: 1126, 1236, 1589, 1619, 1658, 1665, 1724, 1734
   - **Problem:** Placeholders not replaced with actual commands
   - **Impact:** Documentation tells AI to use placeholders, not actual commands

3. **No Integration with --sync-version**
   - **Idea:** Use `fspec --sync-version` to trigger tool configuration check
   - **Current Behavior:** `--sync-version` updates version in slash command files
   - **Desired Behavior:** Could also emit system-reminder if tools not configured

---

## Intended Workflow (Per CONFIG-002 Spec)

### Scenario 1: First Time (No Config)

```
1. AI moves work unit to 'validating' status
   → fspec update-work-unit-status AUTH-001 validating

2. fspec checks spec/fspec-config.json
   → No tools.test.command found

3. fspec emits system-reminder:
   <system-reminder>
   NO TEST COMMAND CONFIGURED

   No test command configured. Use Read/Glob to detect test framework, then run:

     fspec configure-tools --test-command <cmd>

   If no test tools detected, search for current best practices:
     Query: "best <platform> testing tools 2025"
   </system-reminder>

4. AI sees reminder, uses Read/Glob to detect framework:
   → Reads package.json, finds "test": "vitest"

5. AI runs configuration:
   → fspec configure-tools --test-command "npm test"

6. fspec writes config:
   → spec/fspec-config.json now has tools.test.command = "npm test"

7. AI proceeds with validation:
   → npm test
```

### Scenario 2: Subsequent Times (Config Exists)

```
1. AI moves work unit to 'validating' status
   → fspec update-work-unit-status AUTH-002 validating

2. fspec checks spec/fspec-config.json
   → Finds tools.test.command = "npm test"

3. fspec emits system-reminder:
   <system-reminder>
   RUN TESTS

   Run tests: npm test
   </system-reminder>

4. AI runs tests:
   → npm test
```

### Current Reality (What Actually Happens)

```
1. AI moves work unit to 'validating' status
   → fspec update-work-unit-status AUTH-001 validating

2. Nothing happens (no checks performed)

3. AI reads spec/CLAUDE.md documentation
   → Sees placeholder: <test-command>

4. AI guesses what to run based on project context
   → Probably npm test (because it's a Node.js project)
   → No systematic detection or configuration
```

---

## Code Analysis: Where Integration Should Happen

### File: src/commands/update-work-unit-status.ts

**Current State:**
- No imports from `configure-tools.ts`
- No calls to check functions
- No system-reminders about tool configuration

**Required Changes:**

```typescript
// Add import at top of file
import { checkTestCommand, checkQualityCommands } from './configure-tools.js';

// In updateWorkUnitStatus function, after status change success:
// (Around where other state-specific logic happens)

if (newStatus === 'validating') {
  // Emit test command reminder
  const testResult = await checkTestCommand(cwd);
  console.log(testResult.message); // Outputs system-reminder

  // Emit quality check reminder
  const qualityResult = await checkQualityCommands(cwd);
  console.log(qualityResult.message); // Outputs system-reminder
}
```

**Testing Requirements:**
- Add integration test verifying system-reminders emitted
- Test both "config missing" and "config exists" scenarios
- Verify formatAgentOutput() wraps in correct tags for Claude

---

## Placeholder Replacement Status

### File: spec/CLAUDE.md

**Current Placeholders Found:**

| Placeholder | Line Numbers | Context |
|-------------|--------------|---------|
| `<test-command>` | 525, 532, 1126, 1235, 2011, 2022 | ACDD workflow examples |
| `<quality-check-commands>` | 1126, 1236, 1589, 1619, 1658, 1665, 1724, 1734 | Virtual hooks, validation steps |

**Replacement Logic Exists:**
- Location: `src/utils/templateGenerator.ts:153-185`
- Function: `replaceToolPlaceholders(content, config)`
- Reads: `spec/fspec-config.json` for `tools.test.command` and `tools.qualityCheck.commands`
- Replaces: All instances of placeholders with configured commands

**Integration Status:**
- ✅ Logic exists
- ❓ Called during `fspec init`?
- ❓ Updates spec/CLAUDE.md on the fly?
- ❌ Current spec/CLAUDE.md still has placeholders (evidence it's not working)

**Verification Needed:**
1. Does `fspec init` call `replaceToolPlaceholders()`?
2. Should it update spec/CLAUDE.md after `fspec configure-tools`?
3. Or should placeholders remain for projects without tools configured?

---

## Acceptance Criteria for CONFIG-003 Fix

Based on CONFIG-002 scenarios (especially scenario 1, 3, 5, 6):

### Must Have:

1. **System-Reminder Integration**
   - ✅ `checkTestCommand()` called when status → validating
   - ✅ `checkQualityCommands()` called when status → validating
   - ✅ System-reminders emitted to AI agent
   - ✅ Formatted with `formatAgentOutput()` for agent compatibility

2. **Placeholder Replacement**
   - ✅ spec/CLAUDE.md has `<test-command>` replaced if config exists
   - ✅ spec/CLAUDE.md has `<quality-check-commands>` replaced if config exists
   - ✅ Placeholders remain if config missing (prompt AI to configure)

3. **Test Coverage**
   - ✅ Integration test: status → validating emits reminders
   - ✅ Integration test: config missing → prompt to configure
   - ✅ Integration test: config exists → emit actual commands
   - ✅ Update feature file coverage mappings

4. **Documentation**
   - ✅ Update spec/CLAUDE.md section explaining tool detection
   - ✅ Verify help files use correct placeholders
   - ✅ Update CONFIG-002 implementation status

### Optional Enhancements:

1. **--sync-version Integration**
   - Consider emitting tool config check during version sync
   - Helpful for onboarding new AI agents to project

2. **Other Workflow States**
   - Should `implementing` state also emit test reminders?
   - Should `testing` state emit quality check reminders?

---

## Test Commands to Verify Fix

### Before Fix (Current Behavior):

```bash
# 1. Create clean test environment
rm -f spec/fspec-config.json

# 2. Try to validate a work unit
./dist/index.js update-work-unit-status RES-003 validating

# Expected: No tool-related system-reminders
# Actual: (confirm this)
```

### After Fix (Expected Behavior):

```bash
# 1. Create clean test environment
rm -f spec/fspec-config.json

# 2. Try to validate a work unit
./dist/index.js update-work-unit-status RES-003 validating

# Expected output should include:
# <system-reminder>
# NO TEST COMMAND CONFIGURED
# ...
# </system-reminder>

# 3. Configure tools
./dist/index.js configure-tools --test-command "npm test"

# 4. Try validation again
./dist/index.js update-work-unit-status RES-003 validating

# Expected output should include:
# <system-reminder>
# RUN TESTS
# Run tests: npm test
# </system-reminder>
```

---

## Implementation Checklist

### Phase 1: Wire Up Check Functions

- [ ] Import `checkTestCommand` in `update-work-unit-status.ts`
- [ ] Import `checkQualityCommands` in `update-work-unit-status.ts`
- [ ] Call both functions when `newStatus === 'validating'`
- [ ] Output system-reminders to console
- [ ] Add integration test for system-reminder emission
- [ ] Verify `formatAgentOutput()` wraps correctly for Claude

### Phase 2: Verify Placeholder Replacement

- [ ] Trace `fspec init` to see if it calls `replaceToolPlaceholders()`
- [ ] If not, add call to replacement logic
- [ ] Test placeholder replacement with config present
- [ ] Test placeholder retention with config missing
- [ ] Update spec/CLAUDE.md if needed
- [ ] Add test verifying dynamic replacement

### Phase 3: Update Coverage

- [ ] Link new tests to CONFIG-002 scenarios
- [ ] Update `.feature.coverage` file
- [ ] Run `fspec show-coverage conversational-test-and-quality-check-tool-detection`
- [ ] Verify 100% scenario coverage maintained

### Phase 4: Documentation

- [ ] Update spec/CLAUDE.md with tool detection explanation
- [ ] Add note about `fspec configure-tools` in workflow docs
- [ ] Update help files if needed
- [ ] Mark CONFIG-003 as done

---

## Related Files

### Implementation:
- `src/commands/configure-tools.ts` (check functions)
- `src/commands/update-work-unit-status.ts` (integration point)
- `src/utils/templateGenerator.ts` (placeholder replacement)
- `src/utils/agentRuntimeConfig.ts` (formatAgentOutput)

### Tests:
- `src/commands/__tests__/configure-tools.test.ts` (unit tests)
- Need: Integration test in `update-work-unit-status.test.ts`

### Documentation:
- `spec/features/conversational-test-and-quality-check-tool-detection.feature`
- `spec/CLAUDE.md` (placeholder replacement target)

### Coverage:
- `spec/features/conversational-test-and-quality-check-tool-detection.feature.coverage`

---

## Questions for Resolution

1. **When should system-reminders be emitted?**
   - Only during `validating` state?
   - Also during `implementing` or `testing` states?
   - During `fspec --sync-version`?

2. **Should placeholders be replaced in spec/CLAUDE.md?**
   - Yes, if config exists (shows actual commands)
   - No, keep placeholders (prompts AI to configure)
   - Which approach is better?

3. **Should quality checks be mandatory?**
   - Currently optional (no error if missing)
   - Should validation require quality checks configured?

---

## Conclusion

CONFIG-002 is **95% complete** but **0% functional** due to missing integration. The fix is straightforward:

1. Add 3 lines of code to `update-work-unit-status.ts`
2. Verify placeholder replacement works
3. Add integration test
4. Update coverage

**Estimated effort:** 2-3 story points (simple integration, already tested)

**Impact:** Enables the entire conversational tool detection feature for all AI agents using fspec across any platform (Node.js, Python, Rust, Go, etc.)
