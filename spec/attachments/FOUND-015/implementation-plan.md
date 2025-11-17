# FOUND-015: Update Bootstrap Output with Big Picture Event Storming Guidance

## Overview

Enhance `fspec bootstrap` command output to include guidance about Big Picture Event Storming workflow, making it visible to AI agents during initial context loading.

## Problem

Current `fspec bootstrap` output:
- ✅ Outputs complete workflow documentation from CLAUDE.md
- ✅ Includes foundation discovery guidance
- ✅ Includes work unit-level Event Storming guidance
- ❌ Does NOT emphasize Big Picture Event Storming workflow
- ❌ Does NOT remind AI to check for foundation.json eventStorm field
- ❌ AI agents may skip foundation Event Storming unless explicitly prompted

## Solution

Update `fspec bootstrap` to emit a system-reminder when:
1. foundation.json exists
2. foundation.json eventStorm field is empty/missing
3. No active FOUND-XXX work unit for Event Storming exists

The reminder prompts AI to conduct Big Picture Event Storming or pick up the auto-created work unit.

## Implementation Details

### File to Modify
**`src/commands/bootstrap.ts`**

### Location in Code
After outputting CLAUDE.md content, before final "You are now operating in fspec mode" message.

### Detection Logic

```typescript
// Check if Big Picture Event Storm needed
async function shouldPromptEventStorm(cwd: string): Promise<{
  needed: boolean;
  reason: string;
  workUnitId?: string;
}> {
  const foundationPath = path.join(cwd, 'spec', 'foundation.json');
  const workUnitsPath = path.join(cwd, 'spec', 'work-units.json');

  // Check if foundation.json exists
  if (!fs.existsSync(foundationPath)) {
    return { needed: false, reason: 'No foundation.json' };
  }

  const foundation = JSON.parse(await readFile(foundationPath, 'utf-8'));

  // Check if eventStorm field is populated
  if (foundation.eventStorm && foundation.eventStorm.items && foundation.eventStorm.items.length > 0) {
    return { needed: false, reason: 'Event Storm already populated' };
  }

  // Check if there's an active FOUND-XXX work unit for Event Storming
  const workUnits = JSON.parse(await readFile(workUnitsPath, 'utf-8'));
  const eventStormWorkUnit = Object.values(workUnits.workUnits).find(
    (wu: any) =>
      wu.id.startsWith('FOUND-') &&
      wu.title.toLowerCase().includes('event storm') &&
      wu.status !== 'done'
  );

  if (eventStormWorkUnit) {
    return {
      needed: true,
      reason: 'Event Storm work unit exists',
      workUnitId: eventStormWorkUnit.id,
    };
  }

  return { needed: true, reason: 'Event Storm needed but no work unit' };
}
```

### System-Reminder Content

When Big Picture Event Storm is needed:

```typescript
const eventStormStatus = await shouldPromptEventStorm(cwd);

if (eventStormStatus.needed) {
  if (eventStormStatus.workUnitId) {
    // Work unit exists - prompt to work on it
    output += wrapInSystemReminder(`
BIG PICTURE EVENT STORMING NEEDED

foundation.json eventStorm field is empty. A work unit has been created for this:

Work Unit: ${eventStormStatus.workUnitId}

Next steps:
  1. View work unit: fspec show-work-unit ${eventStormStatus.workUnitId}
  2. Move to specifying: fspec update-work-unit-status ${eventStormStatus.workUnitId} specifying
  3. Conduct Big Picture Event Storming using foundation commands:
     - fspec add-foundation-bounded-context <name>
     - fspec add-aggregate-to-foundation <context> <aggregate>
     - fspec add-domain-event-to-foundation <context> <event>
     - fspec show-foundation-event-storm

See spec/CLAUDE.md "Step 1.5a: Big Picture Event Storming" for detailed guidance.

Why this matters:
- Establishes bounded contexts for domain architecture
- Enables tag ontology generation (EXMAP-004)
- Provides foundation for architectural documentation

DO NOT skip this step. It is critical for domain-driven development.
`);
  } else {
    // No work unit - suggest creating one or running Event Storm directly
    output += wrapInSystemReminder(`
BIG PICTURE EVENT STORMING NEEDED

foundation.json exists but eventStorm field is empty.

You should conduct Big Picture Event Storming to establish domain architecture.

Option 1 (Recommended): Create a work unit to track this
  fspec create-story FOUND "Conduct Big Picture Event Storming for Foundation"

Option 2: Conduct Event Storm directly
  fspec add-foundation-bounded-context <name>
  fspec add-aggregate-to-foundation <context> <aggregate>
  fspec add-domain-event-to-foundation <context> <event>
  fspec show-foundation-event-storm

See spec/CLAUDE.md "Step 1.5a: Big Picture Event Storming" for detailed guidance.

Why this matters:
- Establishes bounded contexts for domain architecture
- Enables tag ontology generation (EXMAP-004)
- Provides foundation for architectural documentation
`);
  }
}
```

### Bootstrap Output Changes

**Before** (when foundation exists but no Event Storm):
```
# fspec Command - Kanban-Based Project Management
[... full CLAUDE.md content ...]

You are now operating in fspec mode.
```

**After** (when foundation exists but no Event Storm):
```
# fspec Command - Kanban-Based Project Management
[... full CLAUDE.md content ...]

<system-reminder>
BIG PICTURE EVENT STORMING NEEDED

foundation.json eventStorm field is empty. A work unit has been created for this:

Work Unit: FOUND-XXX

Next steps:
  1. View work unit: fspec show-work-unit FOUND-XXX
  2. Move to specifying: fspec update-work-unit-status FOUND-XXX specifying
  3. Conduct Big Picture Event Storming using foundation commands:
     - fspec add-foundation-bounded-context <name>
     - fspec add-aggregate-to-foundation <context> <aggregate>
     - fspec add-domain-event-to-foundation <context> <event>
     - fspec show-foundation-event-storm

See spec/CLAUDE.md "Step 1.5a: Big Picture Event Storming" for detailed guidance.

Why this matters:
- Establishes bounded contexts for domain architecture
- Enables tag ontology generation (EXMAP-004)
- Provides foundation for architectural documentation

DO NOT skip this step. It is critical for domain-driven development.
</system-reminder>

You are now operating in fspec mode.
```

## Acceptance Criteria

1. ✅ System-reminder emitted when foundation.json exists
2. ✅ System-reminder emitted when eventStorm field is empty
3. ✅ System-reminder references work unit ID if one exists
4. ✅ System-reminder provides clear next steps
5. ✅ System-reminder references CLAUDE.md documentation
6. ✅ System-reminder explains why Big Picture Event Storm matters
7. ✅ System-reminder does NOT appear when:
   - foundation.json does not exist
   - eventStorm field is already populated
   - Event Storm work unit is marked "done"
8. ✅ Reminder appears AFTER CLAUDE.md content, BEFORE "fspec mode" message

## Dependencies

- **FOUND-014**: CLAUDE.md must be updated first (referenced in reminder)
- **FOUND-013**: Work unit auto-creation (reminder references work unit ID)

## Testing

### Unit Tests
- Test reminder emitted when eventStorm empty and work unit exists
- Test reminder emitted when eventStorm empty and no work unit
- Test reminder NOT emitted when eventStorm populated
- Test reminder NOT emitted when no foundation.json
- Test reminder NOT emitted when work unit status is "done"

### Integration Tests
- Run `fspec bootstrap` in project with empty eventStorm field
- Verify system-reminder appears with correct work unit ID
- Run `fspec bootstrap` in project with populated eventStorm
- Verify NO system-reminder appears

### AI Agent Testing
- Bootstrap fspec in test project
- Verify AI agent sees and understands the reminder
- Verify AI agent takes action (views work unit or starts Event Storm)

## Edge Cases

### Case 1: Foundation just created, no work unit yet
**Scenario**: `discover-foundation --finalize` just ran but FOUND-013 not implemented yet

**Behavior**: Emit reminder suggesting Option 2 (conduct Event Storm directly)

### Case 2: Work unit exists but marked "done"
**Scenario**: Event Storm work unit was completed and marked done

**Behavior**: Do NOT emit reminder (Event Storm is complete)

### Case 3: Multiple FOUND-XXX Event Storm work units
**Scenario**: Someone created multiple work units about Event Storming

**Behavior**: Reference the first non-done work unit found

## Related Work Units

- **FOUND-013**: Auto-create Event Storm work unit (provides work unit ID)
- **FOUND-014**: CLAUDE.md documentation (referenced in reminder)
- **EXMAP-004**: Tag ontology generation (benefit of Event Storm)

## Notes

- This is a "nudge" system - reminds AI but doesn't force action
- System-reminder format ensures high visibility in AI agent context
- Complements FOUND-013 (auto work unit) and FOUND-014 (documentation)
- Together, these 3 stories form complete guidance system
