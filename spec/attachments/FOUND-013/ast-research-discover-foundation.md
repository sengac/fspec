# AST Research: discover-foundation.ts

## Functions Discovered

1. **scanDraftForNextField** (line 31)
   - Purpose: Scans foundation draft for next unfilled placeholder field
   - Returns next field path, completion stats

2. **generateFieldReminder** (line 92)
   - Purpose: Generates field-specific system reminders for AI guidance
   - Takes field path, number, and detected value

3. **discoverFoundation** (line 176)
   - **CRITICAL**: Main discovery function - handles draft creation, scanning, finalization
   - **Implementation point**: After finalization (line ~370), add work unit creation logic
   - Returns: systemReminder, foundation object, validation results, paths

4. **registerDiscoverFoundationCommand** (line 487)
   - Purpose: CLI command registration with Commander.js
   - Handles --finalize flag, --output path, --draft-path, --auto-generate-md

## Implementation Location

**Line ~370-390** in discoverFoundation function:
- After `await writeFile(finalPath, JSON.stringify(foundation, null, 2), 'utf-8')`
- After draft deletion: `await unlink(draftPath)`
- Before FOUNDATION.md generation
- Before completion message

This is where we'll add work unit auto-creation logic.

## Key Utilities Needed

- `getNextWorkUnitId(workUnits, 'FOUND')` - Generate next FOUND-XXX ID
- `readFile/writeFile` - Already imported and used
- `chalk.green()` - For console output styling

## Return Value Updates

The function returns an object - we may need to add:
- `workUnitCreated?: boolean`
- `workUnitId?: string`

To track work unit creation in the return value.
