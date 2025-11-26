# BUG-093: VAL-005 1:1 Validation Checks Across Entire Work Unit Instead of Per-Feature

## Summary

The VAL-005 validation (1 feature file = 1 test file) incorrectly aggregates all test files across all feature files in a work unit, then fails if there's more than 1 test file total. This breaks the intended design where each individual feature file should map to exactly one test file.

## Steps to Reproduce

1. Create a work unit with multiple feature files (e.g., 15 feature files)
2. Create 15 corresponding test files (1 test file per feature file - proper 1:1 mapping)
3. Link coverage for all scenarios using `fspec link-coverage`
4. Attempt to move to implementing: `fspec update-work-unit-status WORK-UNIT implementing`

## Expected Behavior

The validation should pass because each feature file has exactly one test file (1:1 mapping per feature).

## Actual Behavior

Error is thrown:
```
✗ Failed to update work unit status: Multiple test files detected for feature file.

Feature: spec/features/feature-1.feature, spec/features/feature-2.feature, ... (15 total)
Test files (15): src/__tests__/feature-1.test.ts, src/__tests__/feature-2.test.ts, ... (15 total)

Design intent: 1 feature file = 1 test file (1:1 mapping)
```

## Root Cause

In `src/commands/update-work-unit-status.ts` lines 885-952:

```typescript
// BUG-061: Read coverage files to find test files (language-agnostic)
const workUnitTestFiles = new Set<string>();  // <-- Single set for ALL features

for (const feature of matchingFeatures) {
  // ... reads coverage file ...
  if (coverage.scenarios && Array.isArray(coverage.scenarios)) {
    for (const scenario of coverage.scenarios) {
      if (scenario.testMappings && Array.isArray(scenario.testMappings)) {
        for (const mapping of scenario.testMappings) {
          if (mapping.file) {
            workUnitTestFiles.add(mapping.file);  // <-- Adds to global set
          }
        }
      }
    }
  }
}

// VAL-005: Enforce 1:1 mapping (1 feature file = 1 test file)
if (workUnitTestFiles.size > 1) {  // <-- BUG: Checks total count, not per-feature
  throw new Error(`Multiple test files detected for feature file...`);
}
```

The bug is that `workUnitTestFiles` is a single `Set<string>` that collects ALL test files from ALL feature files. Then the check `workUnitTestFiles.size > 1` fails when there are multiple features with their own test files.

## Proposed Fix

Move the 1:1 validation INSIDE the feature loop, checking each feature independently:

```typescript
for (const feature of matchingFeatures) {
  const featureTestFiles = new Set<string>();  // <-- Per-feature set

  // ... read coverage file ...

  if (coverage.scenarios && Array.isArray(coverage.scenarios)) {
    for (const scenario of coverage.scenarios) {
      if (scenario.testMappings && Array.isArray(scenario.testMappings)) {
        for (const mapping of scenario.testMappings) {
          if (mapping.file) {
            featureTestFiles.add(mapping.file);
            workUnitTestFiles.add(mapping.file);  // Still collect for later validation
          }
        }
      }
    }
  }

  // VAL-005: Enforce 1:1 mapping PER FEATURE
  if (featureTestFiles.size > 1) {
    throw new Error(
      `Multiple test files detected for feature file.\n\n` +
      `Feature: ${feature.filePath}\n` +
      `Test files (${featureTestFiles.size}): ${Array.from(featureTestFiles).join(', ')}\n\n` +
      `Design intent: 1 feature file = 1 test file (1:1 mapping)`
    );
  }

  if (featureTestFiles.size === 0) {
    throw new Error(
      `No test files linked to feature ${feature.filePath}.\n\n` +
      `Use: fspec link-coverage ...`
    );
  }
}
```

## Affected File

- `src/commands/update-work-unit-status.ts` lines 885-952

## Test Case

A test should verify:
1. Work unit with 1 feature + 1 test file = PASS
2. Work unit with 1 feature + 2 test files = FAIL (correct behavior)
3. Work unit with 3 features + 3 test files (1 each) = PASS (currently fails - bug)
4. Work unit with 3 features where one has 2 test files = FAIL (correct behavior)
