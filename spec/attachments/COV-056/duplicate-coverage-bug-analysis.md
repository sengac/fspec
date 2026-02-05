# COV-056: Duplicate Coverage Entries Bug Analysis

## Summary

The `fspec link-coverage` command does not check for existing test mappings before adding new ones. This results in duplicate coverage entries for the same scenario, which then causes validation failures when trying to advance work unit status.

## How the Bug Was Discovered

While implementing AGENT-008 (Grep tool) in the codelet project, coverage was linked in two phases:

1. **First phase (during testing)**: Tests were written with placeholder code (`const result = '';`) and coverage was linked using `fspec link-coverage` with test file lines but without implementation mappings.

2. **Second phase (after implementation)**: Tests were updated to use actual `executeGrep()` calls, and coverage was linked again with both test file lines AND implementation mappings.

The result was **duplicate test mappings** for each scenario:
- One with `implMappings: []` (from first link)
- One with valid `implMappings` (from second link)

## Reproduction Steps

```bash
# 1. Create a feature with scenarios
fspec create-feature "test-feature"
fspec add-scenario test-feature "Test scenario"

# 2. Link coverage with test file only (no impl)
fspec link-coverage test-feature --scenario "Test scenario" \
  --test-file src/__tests__/test.ts --test-lines 10-20

# 3. Link coverage again with implementation
fspec link-coverage test-feature --scenario "Test scenario" \
  --test-file src/__tests__/test.ts --test-lines 10-20 \
  --impl-file src/impl.ts --impl-lines 50-100

# 4. Check coverage file - will have TWO entries for same test file
```

## Observed Behavior

The `.feature.coverage` JSON file contained duplicate entries:

```json
{
  "scenarios": [
    {
      "name": "Search for pattern returns matching file paths",
      "testMappings": [
        {
          "file": "src/agent/__tests__/grep.test.ts",
          "lines": "62-76",
          "implMappings": []    // <-- First entry (no impl)
        },
        {
          "file": "src/agent/__tests__/grep.test.ts",
          "lines": "62-72",
          "implMappings": [     // <-- Second entry (with impl)
            {
              "file": "src/agent/tools.ts",
              "lines": [267, 268, ...]
            }
          ]
        }
      ]
    }
  ]
}
```

## Impact

1. **Validation failures**: When trying to move work unit to `validating` status, fspec checks for implementation coverage and sees the first entry with empty `implMappings`, causing validation to fail even though the second entry has valid implementation mappings.

2. **Confusing coverage reports**: `fspec show-coverage` shows duplicate entries with "⚠️ No implementation mappings" warnings for entries that actually do have implementation mapped (in the duplicate).

3. **Data inconsistency**: The coverage file contains contradictory information about the same scenario.

## Expected Behavior

When `link-coverage` is called for a scenario + test file combination that already exists:

### Option A: Update existing entry
If the same test file is already linked to the scenario, UPDATE the existing entry rather than adding a new one. This would:
- Merge/update the `lines` range if different
- Add implementation mappings to the existing test mapping

### Option B: Error with guidance
If duplicate would be created, emit an error with guidance:
```
Error: Test file 'src/__tests__/test.ts' is already linked to scenario "Test scenario"

To update implementation mappings for this test:
  fspec link-coverage test-feature --scenario "Test scenario" \
    --test-file src/__tests__/test.ts \
    --impl-file src/impl.ts --impl-lines 50-100

To replace the test mapping entirely, first unlink:
  fspec unlink-coverage test-feature --scenario "Test scenario" \
    --test-file src/__tests__/test.ts
```

### Option C: Warn but allow (with deduplication)
Allow the operation but deduplicate by test file path, keeping the entry with the most complete information (the one with implementation mappings).

## Suggested Fix

Recommend **Option A** as the most user-friendly approach:

1. In `link-coverage` command, before adding a new testMapping:
   - Check if `testMappings` already contains an entry with the same `file` path
   - If yes, update that entry instead of pushing a new one
   - If the existing entry has no `implMappings` and the new command provides impl, add them
   - If the existing entry already has impl for that impl file, update the lines

2. Add validation in `link-coverage` to prevent exact duplicates.

## Files Affected

- `src/commands/link-coverage.ts` (or equivalent) - needs deduplication logic
- Coverage validation logic - may need to handle edge cases

## Workaround Used

The workaround was to:
1. Unlink all coverage for each scenario using `--all` flag
2. Re-link with correct mappings

```bash
fspec unlink-coverage add-grep-tool-for-content-search --scenario "Scenario name" --all
fspec link-coverage add-grep-tool-for-content-search --scenario "Scenario name" \
  --test-file src/agent/__tests__/grep.test.ts --test-lines 62-72 \
  --impl-file src/agent/tools.ts --impl-lines 267-363
```

## Related Work Units

- None identified

## Environment

- fspec version: (current)
- Project: codelet
- Work unit affected: AGENT-008
