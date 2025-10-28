# LOCK-002 Remaining Refactoring Checklist

## Refactoring Pattern

For ALL files listed below, perform these exact steps:

1. **Read the file** using Read tool
2. **Remove writeFile import**: Change `import { writeFile } from 'fs/promises'` to remove `writeFile`
3. **Add fileManager import**: Add `import { fileManager } from '../utils/file-manager'`
4. **Find ALL writeFile calls**: Search for every `await writeFile(...)` in the file
5. **Replace with transaction**:
   ```typescript
   // OLD:
   await writeFile(filePath, JSON.stringify(data, null, 2));

   // NEW (LOCK-002):
   await fileManager.transaction(filePath, async fileData => {
     Object.assign(fileData, data);
   });
   ```

## Files Completed (11 files)

✅ src/utils/ensure-files.ts
✅ src/tui/store/fspecStore.ts
✅ src/commands/create-story.ts
✅ src/commands/create-bug.ts
✅ src/commands/create-task.ts
✅ src/commands/update-work-unit-status.ts
✅ src/commands/update-work-unit.ts
✅ src/commands/delete-work-unit.ts
✅ src/commands/delete-epic.ts
✅ src/commands/epics.ts
✅ src/commands/prioritize-work-unit.ts
✅ src/commands/repair-work-units.ts
✅ src/commands/set-user-story.ts (just completed)
✅ src/commands/add-dependency.ts (just completed)

## Work Unit Commands (23 remaining)

### High Priority - Core Operations
- [ ] **src/commands/remove-dependency.ts** - 1 writeFile call (line ~122)
- [ ] **src/commands/clear-dependencies.ts** - 1 writeFile call
- [ ] **src/commands/add-attachment.ts** - 1 writeFile call
- [ ] **src/commands/remove-attachment.ts** - 1 writeFile call
- [ ] **src/commands/update-work-unit-estimate.ts** - 1 writeFile call

### Example Mapping Related
- [ ] **src/commands/add-rule.ts** - 1 writeFile call
- [ ] **src/commands/remove-rule.ts** - 1 writeFile call
- [ ] **src/commands/add-example.ts** - 1 writeFile call
- [ ] **src/commands/remove-example.ts** - 1 writeFile call
- [ ] **src/commands/add-question.ts** - 1 writeFile call
- [ ] **src/commands/remove-question.ts** - 1 writeFile call
- [ ] **src/commands/answer-question.ts** - 1 writeFile call
- [ ] **src/commands/add-assumption.ts** - 1 writeFile call

### Architecture & Documentation
- [ ] **src/commands/add-architecture-note.ts** - 1 writeFile call
- [ ] **src/commands/remove-architecture-note.ts** - 1 writeFile call
- [ ] **src/commands/add-architecture.ts** - 1 writeFile call

### Dependency Operations
- [ ] **src/commands/add-dependencies.ts** - Likely multiple writeFile calls
- [ ] **src/commands/dependencies.ts** - Check for any writeFile calls
- [ ] **src/commands/export-dependencies.ts** - May have writeFile for export

### Other Work Unit Operations
- [ ] **src/commands/auto-advance.ts** - Likely has writeFile for status updates
- [ ] **src/commands/work-unit.ts** - Check for any writeFile calls
- [ ] **src/commands/workflow-automation.ts** - May have writeFile calls

### Query/Reporting (may be read-only)
- [ ] **src/commands/query-work-units.ts** - CHECK if has writeFile (may be read-only)

## Virtual Hook Commands (5 files)

- [ ] **src/commands/add-virtual-hook.ts** - 1 writeFile call
- [ ] **src/commands/remove-virtual-hook.ts** - 1 writeFile call
- [ ] **src/commands/clear-virtual-hooks.ts** - 1 writeFile call
- [ ] **src/commands/copy-virtual-hooks.ts** - Likely 2 writeFile calls (source + dest)
- [ ] **src/commands/add-hook.ts** - 1 writeFile call
- [ ] **src/commands/remove-hook.ts** - 1 writeFile call

## Prefix and Epic Commands (2 remaining)

- [ ] **src/commands/update-prefix.ts** - 1 writeFile call
- [ ] **src/commands/create-prefix.ts** - 1 writeFile call
- [ ] **src/commands/create-epic.ts** - 1 writeFile call

## Tag Commands (5 files)

- [ ] **src/commands/register-tag.ts** - 1 writeFile call
- [ ] **src/commands/update-tag.ts** - 1 writeFile call
- [ ] **src/commands/delete-tag.ts** - 1 writeFile call
- [ ] **src/commands/retag.ts** - May have writeFile calls
- [ ] **src/commands/add-tag-to-feature.ts** - Gherkin file write (different pattern - NOT JSON)
- [ ] **src/commands/add-tag-to-scenario.ts** - Gherkin file write (different pattern - NOT JSON)
- [ ] **src/commands/remove-tag-from-feature.ts** - Gherkin file write (different pattern - NOT JSON)
- [ ] **src/commands/remove-tag-from-scenario.ts** - Gherkin file write (different pattern - NOT JSON)

**NOTE**: Tag commands that modify Gherkin .feature files (add-tag-to-feature, etc.) do NOT need fileManager since they write text files, not JSON. Only JSON write operations need refactoring.

## Foundation Commands (4 files)

- [ ] **src/commands/update-foundation.ts** - 1 writeFile call (foundation.json)
- [ ] **src/commands/show-foundation.ts** - CHECK if has writeFile (may be read-only)
- [ ] **src/commands/add-diagram.ts** - 1 writeFile call (foundation.json)
- [ ] **src/commands/delete-diagram.ts** - 1 writeFile call (foundation.json)
- [ ] **src/commands/add-diagram-json-backed.ts** - 1 writeFile call
- [ ] **src/commands/discover-foundation.ts** - 1 writeFile call (foundation.json)
- [ ] **src/commands/init.ts** - Multiple writeFile calls (creates initial JSON files)

## Example Mapping Commands (3 files)

- [ ] **src/commands/generate-scenarios.ts** - 1 writeFile call (example-mapping.json)
- [ ] **src/commands/export-example-map.ts** - 1 writeFile call (export JSON)
- [ ] **src/commands/import-example-map.ts** - 1 writeFile call (example-mapping.json)
- [ ] **src/commands/example-mapping.ts** - Check for any writeFile calls

## Feature/Scenario Commands (15 files)

**NOTE**: Most of these write Gherkin .feature files (text), NOT JSON. Only refactor JSON writes.

- [ ] **src/commands/create-feature.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/show-feature.ts** - CHECK if has writeFile (may be read-only)
- [ ] **src/commands/add-scenario.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/delete-scenario.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/delete-scenarios-by-tag.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/update-scenario.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/add-step.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/delete-step.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/update-step.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/add-background.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/format.ts** - Gherkin file write (NOT JSON - skip unless has JSON writes)
- [ ] **src/commands/link-coverage.ts** - 1 writeFile call (work-units.json)
- [ ] **src/commands/unlink-coverage.ts** - 1 writeFile call (work-units.json)
- [ ] **src/commands/show-acceptance-criteria.ts** - CHECK if has writeFile (may be read-only)

## Query/Reporting Commands (6 files)

**NOTE**: Most of these are read-only, but check for export operations.

- [ ] **src/commands/export-work-units.ts** - Export operation (may write output file)
- [ ] **src/commands/query.ts** - CHECK if has writeFile (likely read-only)
- [ ] **src/commands/generate-summary-report.ts** - Report generation (may write output file)
- [ ] **src/commands/generate-foundation-md.ts** - Markdown generation (NOT JSON - skip)
- [ ] **src/commands/generate-tags-md.ts** - Markdown generation (NOT JSON - skip)

## Metrics/Telemetry Commands (3 files)

- [ ] **src/commands/record-iteration.ts** - 1 writeFile call (metrics JSON)
- [ ] **src/commands/record-metric.ts** - 1 writeFile call (metrics JSON)
- [ ] **src/commands/record-tokens.ts** - 1 writeFile call (metrics JSON)
- [ ] **src/commands/estimation.ts** - CHECK if has writeFile

## Summary Statistics

- **Total files to refactor**: ~76 files initially identified
- **Files with JSON writes that need refactoring**: ~50-60 (estimate)
- **Files with Gherkin/text writes (skip)**: ~15-20 (estimate)
- **Files that are read-only (skip)**: ~5-10 (estimate)
- **Completed so far**: 14 files
- **Remaining**: ~62 files to check and refactor

## Critical Notes

1. **ONLY refactor JSON file operations** - Files that write .feature files (Gherkin) or .md files do NOT use fileManager
2. **Read BEFORE Edit** - Always use Read tool first to see the file contents
3. **ALL writeFile calls must be replaced** - Don't miss any writeFile operations in a file
4. **Pattern is consistent** - Remove writeFile import, add fileManager import, replace writeFile calls with transaction
5. **Test after batches** - After every 10-15 files, run `npm test` to verify nothing broke
6. **Object.assign pattern** - Use `Object.assign(fileData, data)` when modifications were done in-memory first
7. **Direct mutation pattern** - Mutate `fileData` directly in transaction callback when appropriate

## Next Steps

1. Start with HIGH PRIORITY work unit commands (remove-dependency, clear-dependencies, etc.)
2. Move through categories systematically
3. Check each file to determine if it writes JSON or text files
4. Only refactor JSON write operations
5. Run tests after each category is complete
6. Update this checklist as files are completed
