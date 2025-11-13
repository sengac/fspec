# System-Reminder Duplication Analysis

## Executive Summary

Found **one primary location** where `<system-reminder>` tags create multiple consecutive blocks instead of a single unified block.

**Location**: `src/commands/update-work-unit-status.ts` (lines 626-684)

## The Problem

When multiple reminders are applicable (e.g., transitioning to 'done' status), the current implementation creates:

```xml
<system-reminder>
Work unit VAL-003 is now in DONE status.
</system-reminder>

<system-reminder>
QUALITY CHECK OPPORTUNITY
...
</system-reminder>
```

**Expected behavior**: All content should be in a single `<system-reminder>` block:

```xml
<system-reminder>
Work unit VAL-003 is now in DONE status.

QUALITY CHECK OPPORTUNITY
...
</system-reminder>
```

## Root Cause Analysis

### File: `src/commands/update-work-unit-status.ts`

**Lines 626-684** collect multiple reminders that are ALREADY wrapped in `<system-reminder>` tags:

```typescript
const reminders: string[] = [];

// Line 633: Get status change reminder (already wrapped)
const statusReminder = await getStatusChangeReminder(
  workUnit,
  targetStatus,
  options.skipTemporalValidation || false
);
if (statusReminder) {
  reminders.push(statusReminder);  // Line 637
}

// Line 640: Get virtual hooks reminder (already wrapped)
const virtualHooksReminder = getVirtualHooksReminder(options.workUnitId);
if (virtualHooksReminder) {
  reminders.push(virtualHooksReminder);  // Line 644
}

// Line 647: Get cleanup reminder (already wrapped)
const cleanupReminder = getVirtualHooksCleanupReminder(
  workUnit,
  targetStatus,
  virtualHooks
);
if (cleanupReminder) {
  reminders.push(cleanupReminder);  // Line 656
}

// Lines 661-678: Create review reminder inline (wrapped)
if (targetStatus === 'done' && workUnit.type === 'story') {
  const reviewReminder = `<system-reminder>
QUALITY CHECK OPPORTUNITY
...
</system-reminder>`;
  reminders.push(reviewReminder);  // Line 678
}

// Line 684: Join creates CONSECUTIVE blocks (the bug!)
const systemReminder = reminders.length > 0 ? reminders.join('\n\n') : undefined;
```

### Why This Happens

1. **Helper functions return wrapped content**:
   - `getStatusChangeReminder()` returns content wrapped in `<system-reminder>` tags
   - `getVirtualHooksReminder()` returns content wrapped in `<system-reminder>` tags
   - `getVirtualHooksCleanupReminder()` returns content wrapped in `<system-reminder>` tags

2. **Inline reminder is also wrapped**:
   - The review reminder (lines 661-678) is created with tags already included

3. **Join operation preserves all wrappers**:
   - `reminders.join('\n\n')` concatenates all the already-wrapped blocks
   - This creates: `</system-reminder>\n\n<system-reminder>` between blocks

## Helper Functions Involved

### `getStatusChangeReminder()` (lines 553-623)
Returns wrapped content like:
```typescript
return wrapInSystemReminder(`Work unit ${workUnit.id} is now in ${status.toUpperCase()} status.`);
```

### `getVirtualHooksReminder()` (lines 546-550)
Uses utility function that returns wrapped content.

### `getVirtualHooksCleanupReminder()` (lines 510-543)
Returns wrapped content:
```typescript
return wrapInSystemReminder(message);
```

## Proposed Solution

**Option 1: Collect unwrapped content, wrap once** (Recommended)

```typescript
// Change helper functions to return unwrapped content
const reminderContents: string[] = [];

const statusContent = await getStatusChangeReminderContent(...);  // New unwrapped version
if (statusContent) {
  reminderContents.push(statusContent);
}

// ... collect other unwrapped content ...

// Wrap ONCE at the end
const systemReminder = reminderContents.length > 0
  ? wrapInSystemReminder(reminderContents.join('\n\n'))
  : undefined;
```

**Option 2: Strip wrappers and re-wrap**

```typescript
// Strip <system-reminder> tags from each reminder
const unwrappedContents = reminders.map(r =>
  r.replace(/<system-reminder>\n?/g, '')
     .replace(/<\/system-reminder>\n?/g, '')
     .trim()
);

// Wrap once
const systemReminder = unwrappedContents.length > 0
  ? wrapInSystemReminder(unwrappedContents.join('\n\n'))
  : undefined;
```

## Impact Assessment

### Files That Need Changes

1. **`src/commands/update-work-unit-status.ts`** (Primary fix)
   - Modify lines 626-684 to collect unwrapped content
   - May need to refactor helper functions

2. **Helper functions** (if using Option 1):
   - `getStatusChangeReminder()` → `getStatusChangeReminderContent()`
   - `getVirtualHooksReminder()` → `getVirtualHooksReminderContent()`
   - `getVirtualHooksCleanupReminder()` → `getVirtualHooksCleanupReminderContent()`

### Testing Requirements

1. Test single reminder (existing behavior should work)
2. Test multiple reminders (should combine into single block)
3. Test transitions that trigger all reminders:
   - Story transitioning to 'done' (status + review reminders)
   - Work unit with virtual hooks (status + hooks reminders)
   - Work unit in 'done' status with virtual hooks (status + cleanup reminders)

## Other Locations Reviewed (No Issues Found)

These files manually construct `<system-reminder>` tags but do NOT cause duplication:

1. `src/commands/create-story.ts:142` - Standalone reminder
2. `src/commands/create-bug.ts:142` - Standalone reminder
3. `src/commands/create-task.ts:142` - Standalone reminder
4. `src/utils/prefill-detection.ts:115-140` - Standalone reminder
5. `src/utils/foundation-check.ts:73-87` - Standalone reminder
6. `src/utils/git-checkpoint.ts:351,455,970` - Standalone reminders
7. `src/utils/step-validation.ts:236-306` - Standalone reminder
8. `src/utils/agentRuntimeConfig.ts:101` - Utility function
9. `src/utils/projectManagementTemplate.ts:40` - Utility function
10. `src/utils/system-reminder.ts:27` - Core wrapper utility (correct abstraction)

## Verification

To verify the bug exists, run:

```bash
# Transition a story work unit to 'done' status
fspec update-work-unit-status STORY-XXX done
```

Check the output for multiple consecutive `<system-reminder>` blocks.

## Priority

**Medium** - This affects user experience when AI agents receive reminders, but the information is still delivered (just in a less optimal format).
