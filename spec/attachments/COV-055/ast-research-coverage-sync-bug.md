# AST Research: Coverage File Synchronization Bug

## Investigation Summary

Investigated coverage checking system focusing on bug where removing scenarios/tests causes coverage checker errors even after unlinking.

## Root Cause Analysis

The bug has THREE contributing factors:

### Bug #1: coverage-file.ts - updateCoverageFile() Only Adds, Never Removes
**Location:** `/Users/rquast/projects/fspec/src/utils/coverage-file.ts:103-171`

The `updateCoverageFile()` function only adds new scenarios, it never removes deleted ones:

```typescript
// Lines 138-147
const newScenarios: CoverageScenario[] = [];
for (const name of currentScenarioNames) {
  if (!existingScenarioNames.has(name)) {
    newScenarios.push({ name, testMappings: [] });
  }
}

// Lines 155-161: Only appends new scenarios
const updatedCoverage: CoverageFile = {
  scenarios: [...existingCoverage.scenarios, ...newScenarios],
  // ...
};
```

**Missing Logic:** Should also detect and remove scenarios that exist in coverage file but not in feature file.

### Bug #2: delete-scenario.ts & delete-scenarios-by-tag.ts - No Coverage Cleanup
**Locations:**
- `/Users/rquast/projects/fspec/src/commands/delete-scenario.ts` (entire file)
- `/Users/rquast/projects/fspec/src/commands/delete-scenarios-by-tag.ts` (entire file)

Neither deletion command updates the corresponding `.feature.coverage` file.

**Expected Behavior:** After deleting scenario from feature file, should:
1. Load corresponding `.feature.coverage` file
2. Remove scenario entry from `coverage.scenarios` array
3. Recalculate stats
4. Write updated coverage file

### Bug #3: update-work-unit-status.ts - Validates Against Stale Data
**Location:** `/Users/rquast/projects/fspec/src/commands/update-work-unit-status.ts:1074-1082`

The coverage checker validates coverage file against itself, not against the actual feature file:

```typescript
// This checks coverage file scenarios
const uncoveredScenarios = coverage.scenarios.filter(
  (scenario: any) =>
    !scenario.testMappings || scenario.testMappings.length === 0
);
```

**Missing Logic:** Should first validate that all scenarios in coverage file still exist in the feature file before checking for uncovered scenarios.

## Reproduction Scenario

```bash
# Setup
fspec add-scenario test "Scenario A"
fspec add-scenario test "Scenario B"
fspec link-coverage test --scenario "A" --test-file test.ts --test-lines 1-10

# Trigger bug
fspec delete-scenario test "B"  # ✓ Deletes from test.feature
                                 # ✗ Coverage file STILL has "Scenario B"

fspec update-work-unit-status WORK-001 validating
# ❌ ERROR: "Scenario B is not covered"
# But Scenario B doesn't exist in feature file anymore!
```

## Files That Need Changes

1. **src/utils/coverage-file.ts** - Add logic to remove deleted scenarios in `updateCoverageFile()`
2. **src/commands/delete-scenario.ts** - Add coverage cleanup after line 162
3. **src/commands/delete-scenarios-by-tag.ts** - Add coverage cleanup for bulk deletions
4. **src/commands/update-work-unit-status.ts** - Validate coverage against feature file first
5. **src/commands/update-scenario.ts** - Rename coverage entry when scenario renamed

## Edge Cases Identified

1. **Scenario renamed**: Coverage file keeps old name, feature file has new name → appears as both uncovered and orphaned
2. **Feature file deleted but coverage remains**: Coverage checker fails with confusing error
3. **Coverage file manually edited**: No validation ensures scenarios exist in feature file
4. **Concurrent unlinking and deletion**: Coverage file ends up with orphaned scenario entries
