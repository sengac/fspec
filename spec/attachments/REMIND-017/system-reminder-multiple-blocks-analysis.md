# REMIND-017: Multiple System-Reminder Blocks Analysis

## Problem

Commands that collect multiple system reminders emit them as **separate consecutive `<system-reminder>` blocks** instead of consolidating into a single block. This was fixed for `update-work-unit-status` in VAL-004 but never propagated to the other affected commands.

## Reference Implementation (VAL-004 fix in `update-work-unit-status`)

```typescript
// src/commands/update-work-unit-status.ts lines 808-819
const reminders: string[] = [];
// ... collect individually-wrapped reminders ...

if (reminders.length > 0) {
  const unwrappedContents = reminders.map(r =>
    r.replace(/<system-reminder>\n?/g, '')
      .replace(/<\/system-reminder>\n?/g, '')
      .trim()
  );
  systemReminder = wrapInSystemReminder(unwrappedContents.join('\n\n'));
}
```

This strips individual `<system-reminder>` wrappers, joins content with double newlines, and re-wraps once.

## Affected Commands

### 1. `show-work-unit` — up to 5 separate blocks

**File:** `src/commands/show-work-unit.ts`  
**Collection:** line 214 — `const systemReminders: string[] = [];`  
**Emission:** lines 447-452

```typescript
for (const reminder of result.systemReminders) {
  output.log(reminder);   // Each individually wrapped
  output.log('');
}
```

**Reminders collected:**
- Missing estimate reminder
- Empty Example Mapping reminder
- Long duration reminder (>24h in current state)
- Large estimate reminder (>13 points)
- Active/deleted item count hint

### 2. `add-tag-to-feature` — 1+ blocks per unregistered tag

**File:** `src/commands/add-tag-to-feature.ts`  
**Collection:** line 211 — `const systemReminders: string[] = [];`  
**Emission:** lines 313-316

```typescript
for (const reminder of result.systemReminders) {
  output.log('\n' + reminder);  // Each individually wrapped
}
```

**Reminders collected:**
- Unregistered tag reminders (one per tag)
- Missing required tags reminder

### 3. `generate-scenarios` — up to 3 separate blocks

**File:** `src/commands/generate-scenarios.ts`  
**Collection:** line 545 — `const systemReminders: string[] = [];`  
**Emission:** lines 656-659

```typescript
for (const reminder of result.systemReminders) {
  output.log('\n' + reminder);  // Each individually wrapped
}
```

**Reminders collected:**
- Scenario generation guidance
- Post-generation reminder
- Prefill detection reminder

### 4. TUI path (`globalSessionStreamManager.ts`) — re-wraps each entry separately

**File:** `src/tui/services/globalSessionStreamManager.ts`  
**Emission:** lines 366-369

```typescript
systemReminder = parsed.systemReminders
  .map(r => `<system-reminder>\n${r}\n</system-reminder>`)
  .join('\n');
```

This takes the already-unwrapped array from `fspec-callback.ts` `parseSystemReminders()` and wraps each one individually again, producing multiple consecutive blocks.

**Note:** `fspec-callback.ts` `parseSystemReminders()` (line 1257) correctly strips tags during parsing, returning bare content strings. The TUI path then incorrectly re-wraps each one separately.

## What Currently Works (Not Affected)

- **`update-work-unit-status`** — already consolidates (VAL-004 fix)
- **`fspec-callback.ts` parseSystemReminders** — correctly strips tags on parse
- **Single-reminder commands** (create-story, create-bug, create-task, etc.) — only ever emit one block

## Fix Approach

### For commands (show-work-unit, add-tag-to-feature, generate-scenarios)

Apply the same consolidation pattern at the point where `systemReminders: string[]` is returned:

```typescript
// Before returning result, consolidate
let consolidatedReminder: string | undefined;
if (systemReminders.length > 0) {
  const unwrappedContents = systemReminders.map(r =>
    r.replace(/<system-reminder>\n?/g, '').replace(/<\/system-reminder>\n?/g, '').trim()
  );
  consolidatedReminder = wrapInSystemReminder(unwrappedContents.join('\n\n'));
}
return { ...result, systemReminder: consolidatedReminder };
```

### For TUI path (globalSessionStreamManager.ts)

Since `parsed.systemReminders` is already unwrapped text, just join and wrap once:

```typescript
systemReminder = `<system-reminder>\n${parsed.systemReminders.join('\n\n')}\n</system-reminder>`;
```

## Existing Test Coverage

- `src/commands/__tests__/system-reminder-consolidation.test.ts` — verifies consolidation for `update-work-unit-status` only
- Feature: `spec/features/multiple-consecutive-system-reminder-blocks-in-update-work-unit-status.feature`

## Evidence

Running `fspec show-work-unit VAL-004` right now emits two separate blocks:
```
<system-reminder>
Work unit VAL-004 has no estimate...
</system-reminder>
<system-reminder>
Work unit VAL-004 has been in done status for 3224 hours...
</system-reminder>
```
