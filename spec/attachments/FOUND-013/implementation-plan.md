# FOUND-013: Auto-create Big Picture Event Storming Work Unit After Foundation Finalization

## Overview

Automatically create a work unit after `discover-foundation --finalize` completes successfully, prompting AI agents to conduct Big Picture Event Storming to populate the foundation.json eventStorm field.

## Problem

Currently, when `fspec discover-foundation --finalize` completes:
- ✅ foundation.json is created with project metadata, personas, capabilities
- ❌ eventStorm field remains unpopulated (optional field, never prompted)
- ❌ AI agents have no guidance to conduct Big Picture Event Storming
- ❌ Domain architecture (bounded contexts, aggregates, events) is not captured

## Solution

After successful foundation finalization, automatically create a work unit that:
1. Appears in backlog with clear title and description
2. Prompts AI to use existing Event Storming commands
3. Explains why Big Picture Event Storming matters (tag ontology, domain architecture)

## Implementation Details

### File to Modify
**`src/commands/discover-foundation.ts`**

### Location in Code
After validation passes and foundation.json is written (line ~300-350), add work unit creation logic.

### Work Unit Auto-Creation Logic

```typescript
// After successful foundation.json creation
if (options.finalize && validationResult.valid) {
  await writeFile(outputPath, JSON.stringify(validatedDraft, null, 2));

  // NEW: Auto-create Big Picture Event Storming work unit
  const workUnitsPath = path.join(cwd, 'spec', 'work-units.json');
  const workUnits = JSON.parse(await readFile(workUnitsPath, 'utf-8'));

  // Find next FOUND-XXX ID
  const nextId = getNextWorkUnitId(workUnits, 'FOUND');

  // Create work unit
  workUnits.workUnits[nextId] = {
    id: nextId,
    title: 'Conduct Big Picture Event Storming for Foundation',
    description: `Complete the foundation by capturing domain architecture through Big Picture Event Storming.

Use these commands to populate foundation.json eventStorm field:
- fspec add-foundation-bounded-context <name>
- fspec add-aggregate-to-foundation <context> <aggregate>
- fspec add-domain-event-to-foundation <context> <event>
- fspec show-foundation-event-storm

Why this matters:
- Establishes bounded contexts for domain-driven design
- Enables tag ontology generation from domain model
- Provides foundation for architectural documentation
- Supports EXMAP-004 tag discovery workflow

See spec/CLAUDE.md "Big Picture Event Storming" section for detailed guidance.`,
    type: 'story',
    status: 'backlog',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  await writeFile(workUnitsPath, JSON.stringify(workUnits, null, 2));

  console.log(chalk.green(`✓ Created work unit ${nextId}: Big Picture Event Storming`));
  console.log(chalk.dim(`  Run: fspec show-work-unit ${nextId}`));
}
```

### Output Changes

**Before:**
```
✓ Generated spec/foundation.json
✓ Generated spec/FOUNDATION.md
✓ Foundation discovered and validated successfully
```

**After:**
```
✓ Generated spec/foundation.json
✓ Generated spec/FOUNDATION.md
✓ Foundation discovered and validated successfully
✓ Created work unit FOUND-XXX: Big Picture Event Storming
  Run: fspec show-work-unit FOUND-XXX
```

## Acceptance Criteria

1. ✅ Work unit is created ONLY when `--finalize` flag is used
2. ✅ Work unit is created ONLY when validation passes
3. ✅ Work unit ID uses FOUND prefix with next available number
4. ✅ Work unit type is 'story'
5. ✅ Work unit status is 'backlog'
6. ✅ Work unit description includes:
   - Clear explanation of what to do
   - List of Event Storming commands to use
   - Explanation of why Big Picture Event Storming matters
   - Reference to CLAUDE.md documentation
7. ✅ Console output confirms work unit creation
8. ✅ Console output shows command to view work unit details

## Dependencies

- **FOUND-014**: CLAUDE.md must be updated first (reference in description)
- **Existing commands**: Uses existing Event Storming commands (no new commands needed)

## Testing

### Unit Tests
- Test work unit creation when `--finalize` flag used
- Test work unit NOT created without `--finalize`
- Test work unit NOT created when validation fails
- Test correct FOUND-XXX ID generation

### Integration Tests
- Run full discover-foundation workflow
- Verify work unit appears in backlog
- Verify work unit has correct structure
- Verify console output matches expected format

## Related Work Units

- **EXMAP-004**: Event Storming + Tag Integration (depends on foundation Event Storm)
- **FOUND-014**: CLAUDE.md documentation (must be completed first)
- **FOUND-015**: Bootstrap guidance (complementary)

## Notes

- Reuses existing Event Storming infrastructure (SOLID/DRY)
- No new commands needed - just automation + prompting
- Work unit approach makes it discoverable and trackable
- AI agents can see it in Kanban board, interactive TUI
