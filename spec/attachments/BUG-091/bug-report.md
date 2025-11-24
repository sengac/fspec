# Bug Report: link-coverage crashes when coverage file stats are missing

## Description
The `fspec link-coverage` command fails with a `TypeError: Cannot set properties of undefined (setting 'coveredScenarios')` when the target coverage file exists but is missing the `stats` object or has an incomplete structure.

## Reproduction Steps
1. Create a feature file (e.g., `test.feature`).
2. Create a coverage file `test.feature.coverage` with minimal content, missing the `stats` object:
   ```json
   {
     "scenarios": []
   }
   ```
3. Run `fspec link-coverage test.feature --scenario "Some Scenario" --test-file test.ts --test-lines 1-10`.

## Expected Behavior
The command should either:
1. Automatically initialize the missing `stats` object and proceed.
2. Or fail gracefully with a descriptive error message indicating the coverage file is malformed.

## Actual Behavior
The command crashes with an unhandled exception:
```
Error: Cannot set properties of undefined (setting 'coveredScenarios')
```

## Context
This was encountered while testing Antigravity support (AGENT-020). I manually created a coverage file to bypass a "Coverage file not found" error, but I didn't include the `stats` object. When I tried to link coverage, it crashed.

## Workaround
Manually adding the `stats` object to the coverage file fixes the issue:
```json
{
  "scenarios": [],
  "stats": {
    "totalScenarios": 0,
    "coveredScenarios": 0,
    "coveragePercent": 0,
    "testFiles": [],
    "implFiles": [],
    "totalLinesCovered": 0
  }
}
```

## Suspected Cause
In `src/commands/link-coverage.ts`, the `updateStats` function likely attempts to write to `coverage.stats.coveredScenarios` without checking if `coverage.stats` exists.

```typescript
function updateStats(coverage: CoverageFile): void {
  // ...
  coverage.stats.coveredScenarios = coveredScenarios; // Crash here if coverage.stats is undefined
  // ...
}
```
