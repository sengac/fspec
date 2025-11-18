# Bug: Duplicate Event Storm Entries Not Prevented

## Issue Summary

The `fspec add-domain-event` command (and likely other Event Storm commands) does not prevent duplicate entries. Running the same command multiple times creates multiple entries with different IDs, leading to data integrity issues and confusion.

## Steps to Reproduce

1. Create a work unit and start Event Storm:
   ```bash
   fspec create-story UI "Test Story"
   fspec update-work-unit-status UI-001 specifying
   fspec discover-event-storm UI-001
   ```

2. Add the same domain event multiple times:
   ```bash
   fspec add-domain-event UI-001 "TrackPlayed"
   fspec add-domain-event UI-001 "TrackPlayed"
   fspec add-domain-event UI-001 "TrackPlayed"
   ```

3. View the Event Storm:
   ```bash
   fspec show-event-storm UI-001
   ```

## Expected Behavior

**Option 1: Prevent Duplicates**
- Command should check if an event with the same text already exists
- If duplicate found, exit with error: "Event 'TrackPlayed' already exists (ID: 0)"
- Suggest using update command or different name

**Option 2: Warn on Duplicates**
- Allow duplicate but warn user: "⚠ Warning: Event 'TrackPlayed' already exists (ID: 0)"
- Ask for confirmation to create duplicate
- Log warning in output

**Option 3: Update Existing**
- If duplicate text found, update the existing entry instead of creating new one
- Provide flag `--force-duplicate` to explicitly allow duplicates

## Actual Behavior

Multiple entries are created with different IDs:

```json
[
  {
    "id": 0,
    "type": "event",
    "text": "TrackPlayed",
    "deleted": false,
    "createdAt": "2025-11-18T05:22:00.633Z"
  },
  {
    "id": 1,
    "type": "event",
    "text": "TrackPlayed",
    "deleted": false,
    "createdAt": "2025-11-18T05:22:10.439Z"
  },
  {
    "id": 2,
    "type": "event",
    "text": "TrackPlayed",
    "deleted": false,
    "createdAt": "2025-11-18T05:22:15.555Z"
  }
]
```

No warning, no error, no deduplication.

## Impact

### High Priority Issues:

1. **Data Quality**: Event Storm artifacts become polluted with duplicates
2. **Example Mapping Confusion**: Duplicate events generate duplicate examples
3. **Scenario Generation**: May create redundant or conflicting scenarios
4. **User Confusion**: Users don't know if duplicates were intentional
5. **Cleanup Required**: Manual JSON editing needed to remove duplicates

### Affected Commands:

Likely affects all Event Storm artifact commands:
- `fspec add-domain-event`
- `fspec add-command`
- `fspec add-policy`
- `fspec add-hotspot`
- `fspec add-aggregate` (foundation)
- `fspec add-bounded-context` (foundation)

## Real-World Evidence

From UI-005 Event Storm session:

```bash
$ fspec show-event-storm UI-005 | jq -r '.[] | select(.type=="event") | "\(.id): \(.text)"'
0: TrackPlayed
1: TrackPlayed      # Duplicate created accidentally
2: TrackPaused
3: TrackStopped
4: TrackEnded
5: TrackPaused      # Another duplicate
6: VolumeChanged
7: PlaylistCreated
...
```

16 event entries, but only 14 unique domain events. 2 duplicates created during session.

## Root Cause Analysis

**Hypothesis**: Commands append to array without checking for existing entries.

**Likely implementation**:
```typescript
// Current behavior (guessed)
function addDomainEvent(workUnitId: string, text: string) {
  const eventStorm = getEventStorm(workUnitId);
  const newEvent = {
    id: eventStorm.length,  // Sequential ID
    type: "event",
    text: text,
    deleted: false,
    createdAt: new Date().toISOString()
  };
  eventStorm.push(newEvent);  // No duplicate check!
  saveWorkUnit(workUnitId);
}
```

**Should be**:
```typescript
function addDomainEvent(workUnitId: string, text: string, options?: { allowDuplicate?: boolean }) {
  const eventStorm = getEventStorm(workUnitId);

  // Check for duplicates (case-insensitive, non-deleted only)
  const existingEvent = eventStorm.find(
    e => e.type === "event" &&
         e.text.toLowerCase() === text.toLowerCase() &&
         !e.deleted
  );

  if (existingEvent && !options?.allowDuplicate) {
    throw new Error(
      `Event '${text}' already exists (ID: ${existingEvent.id}). ` +
      `Use --force-duplicate to create anyway.`
    );
  }

  const newEvent = {
    id: getNextId(eventStorm),
    type: "event",
    text: text,
    deleted: false,
    createdAt: new Date().toISOString()
  };

  eventStorm.push(newEvent);
  saveWorkUnit(workUnitId);

  console.log(`✓ Added domain event "${text}" to ${workUnitId} (ID: ${newEvent.id})`);
}
```

## Suggested Fix

### Approach 1: Strict Validation (Recommended)

- **Prevent duplicates** by default
- Add `--force-duplicate` or `--allow-duplicate` flag for intentional duplicates
- Exit with code 1 and clear error message on duplicate detection
- Check case-insensitive to catch "TrackPlayed" vs "trackplayed"
- Only check non-deleted entries

### Approach 2: Interactive Confirmation

- Detect duplicate and prompt user:
  ```
  ⚠ Event 'TrackPlayed' already exists (ID: 0, created 2025-11-18T05:22:00.633Z)

  Options:
    [U] Update existing event
    [D] Create duplicate anyway
    [C] Cancel

  Choice: _
  ```

### Approach 3: Auto-Update

- If duplicate text found, update metadata (timestamp) but don't create new entry
- Log: `✓ Event "TrackPlayed" already exists, no changes made`

## Testing Strategy

### Unit Tests:

```typescript
describe('add-domain-event', () => {
  it('should prevent duplicate domain events by default', () => {
    addDomainEvent('TEST-001', 'EventA');
    expect(() => addDomainEvent('TEST-001', 'EventA')).toThrow(
      /already exists/
    );
  });

  it('should allow duplicates with --force-duplicate flag', () => {
    addDomainEvent('TEST-001', 'EventA');
    addDomainEvent('TEST-001', 'EventA', { allowDuplicate: true });
    const events = getEventStorm('TEST-001');
    expect(events.filter(e => e.text === 'EventA')).toHaveLength(2);
  });

  it('should be case-insensitive when checking duplicates', () => {
    addDomainEvent('TEST-001', 'EventA');
    expect(() => addDomainEvent('TEST-001', 'eventa')).toThrow();
  });

  it('should allow same text if original is deleted', () => {
    addDomainEvent('TEST-001', 'EventA');
    deleteDomainEvent('TEST-001', 0);
    addDomainEvent('TEST-001', 'EventA'); // Should succeed
  });
});
```

### Integration Tests:

```bash
# Test CLI behavior
fspec add-domain-event TEST-001 "EventA"
# Should succeed

fspec add-domain-event TEST-001 "EventA"
# Should fail with exit code 1 and error message

fspec add-domain-event TEST-001 "EventA" --force-duplicate
# Should succeed with warning
```

## Additional Context

- Tested on: macOS (Darwin 24.6.0)
- fspec version: Latest
- Node.js version: v22.20.0
- Work unit: UI-005 (Music Player Event Storm)

## Related Issues

- BUG-086: Event Storm commands exit with code 1 on success (separate issue)
- This bug is orthogonal - it's about data validation, not exit codes

## Priority

**High** - Affects data integrity and user experience during critical discovery phase. Event Storm is the foundation for Example Mapping and scenario generation, so data quality issues here cascade downstream.
