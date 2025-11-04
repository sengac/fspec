# BUG-064: Duplicate Work Unit IDs in State Arrays

## Summary

The `update-work-unit-status` command creates duplicate entries in the `states` arrays when:
1. Moving a work unit backward through workflow states (e.g., `testing` → `specifying`)
2. Moving a work unit to the same state it's already in

## Root Cause

The state management logic in `src/commands/update-work-unit-status.ts` fails to properly remove the work unit ID from the previous state array before adding it to the new state array.

## Reproduction Steps

1. Create a work unit or use existing one (e.g., TUI-016)
2. Move it to `testing` state: `fspec update-work-unit-status TUI-016 testing`
3. Move it backward to `specifying`: `fspec update-work-unit-status TUI-016 specifying`
4. Move it forward to `testing` again: `fspec update-work-unit-status TUI-016 testing`
5. Check `spec/work-units.json` - the `states.testing` array will contain duplicate `"TUI-016"` entries

## Observed Behavior

```json
{
  "states": {
    "testing": [
      "TUI-016",
      "TUI-016"
    ]
  }
}
```

## Expected Behavior

```json
{
  "states": {
    "testing": [
      "TUI-016"
    ]
  }
}
```

## Evidence from TUI-016

State history shows backward movement:
```json
"stateHistory": [
  {
    "state": "specifying",
    "timestamp": "2025-11-03T21:50:52.207Z"
  },
  {
    "state": "testing",
    "timestamp": "2025-11-03T22:24:55.201Z"
  },
  {
    "state": "specifying",
    "timestamp": "2025-11-03T22:31:54.206Z"
  },
  {
    "state": "testing",
    "timestamp": "2025-11-03T23:34:49.241Z"
  }
]
```

After these transitions, `states.testing` contained `["TUI-016", "TUI-016"]`.

## Impact

- **Kanban board corruption**: Work units appear multiple times in the same column
- **Query inconsistencies**: Commands like `fspec list-work-units --status=testing` may return duplicates
- **Data integrity**: Violates the invariant that each work unit appears exactly once across all state arrays

## Suspected Code Location

`src/commands/update-work-unit-status.ts` - likely in the section that manages the `states` object updates.

The removal logic should:
1. Remove work unit ID from ALL state arrays (not just the current state)
2. Add work unit ID to the new state array
3. Deduplicate to prevent duplicates even if logic fails

## Proposed Fix

Add deduplication or proper removal logic:

```typescript
// Pseudo-code for fix
// 1. Remove from ALL states first
for (const state of Object.keys(workUnitsData.states)) {
  workUnitsData.states[state] = workUnitsData.states[state].filter(
    id => id !== workUnitId
  );
}

// 2. Add to new state
if (!workUnitsData.states[newStatus].includes(workUnitId)) {
  workUnitsData.states[newStatus].push(workUnitId);
}
```

## Test Cases Needed

1. Moving backward (testing → specifying → testing) should not create duplicates
2. Moving to same state (testing → testing) should not create duplicates
3. Moving forward normally should work as before
4. After any state transition, each work unit should appear in exactly one state array

## Workaround

Manually edit `spec/work-units.json` to remove duplicates from state arrays.
