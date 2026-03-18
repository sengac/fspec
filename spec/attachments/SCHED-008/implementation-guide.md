# SCHED-008: Schedule TUI Slash Commands — Implementation Guide

## Overview

Add `/schedule` slash commands to the TUI for managing schedules: add (agent or shell), list, pause, resume, remove. Integrate with the existing slash command infrastructure.

## Existing Slash Command Architecture

### Three-Layer System

1. **Registry** — `src/tui/utils/slashCommands.ts` defines `SLASH_COMMANDS[]` array
2. **Hook** — `src/tui/hooks/useSlashCommandInput.ts` manages palette visibility, navigation, selection
3. **Dispatch** — `AgentView.tsx`'s `handleSubmitWithCommand()` uses if-chain to route commands

### Registration Pattern

Add to `SLASH_COMMANDS` in `src/tui/utils/slashCommands.ts`:

```typescript
export const SLASH_COMMANDS: SlashCommand[] = [
  // ... existing commands ...
  {
    name: 'schedule',
    description: 'Manage scheduled jobs (add, list, pause, resume, remove)',
    syntax: 'add|list|pause|resume|remove [options]',
    requiresSession: false, // Schedule management doesn't require an active AI session
  },
];
```

### Dispatch Pattern

Add to `handleSubmitWithCommand` in `AgentView.tsx`:

```typescript
if (userMessage === '/schedule' || userMessage.startsWith('/schedule ')) {
  await handleScheduleCommand(userMessage);
  return;
}
```

## Subcommand Parsing

`/schedule` has subcommands with distinct argument shapes:

### `/schedule add` — Agent type
```
/schedule add nightly-review --cron "0 2 * * *" --tz Australia/Brisbane --role "Code reviewer" --prompt "Review all open PRs" [--overlap skip|queue]
```

### `/schedule add` — Shell type
```
/schedule add daily-sync --cron "0 9 * * 1-5" --tz UTC --command "npm run sync" [--overlap skip|queue]
```

### `/schedule list`
```
/schedule list
```

### `/schedule pause|resume`
```
/schedule pause nightly-review
/schedule resume nightly-review
```

### `/schedule remove`
```
/schedule remove daily-sync
```

### Argument Parser

Create a dedicated parser in `src/tui/utils/scheduleCommandParser.ts`:

```typescript
interface ParsedScheduleCommand {
  subcommand: 'add' | 'list' | 'pause' | 'resume' | 'remove';
  name?: string;
  cron?: string;
  timezone?: string;
  role?: string;
  prompt?: string;
  command?: string;
  overlapPolicy?: 'skip' | 'queue';
}

export function parseScheduleCommand(input: string): ParsedScheduleCommand {
  // Parse the slash command string into structured arguments
  // Handle quoted strings for cron, role, prompt, command
}
```

## UI Rendering

### `/schedule list` Output

Render as a formatted table in the TUI output area (as a `UserNotification` StreamChunk):

```
┌─────────────────┬───────────────┬──────────────────┬───────┬──────────────────────┬──────────────────────┬────────┐
│ Name            │ Cron          │ Timezone         │ Type  │ Last Run             │ Next Run             │ Status │
├─────────────────┼───────────────┼──────────────────┼───────┼──────────────────────┼──────────────────────┼────────┤
│ nightly-review  │ 0 2 * * *     │ Australia/Bris.. │ agent │ 2026-03-17 02:00 AEST│ 2026-03-18 02:00 AEST│ active │
│ daily-sync      │ 0 9 * * 1-5   │ UTC              │ shell │ never                │ 2026-03-18 09:00 UTC │ active │
└─────────────────┴───────────────┴──────────────────┴───────┴──────────────────────┴──────────────────────┴────────┘
```

### Confirmation Messages

All mutating commands display confirmation via `UserNotification`:

```
✓ Schedule "nightly-review" added (agent, 0 2 * * *, Australia/Brisbane)
✓ Schedule "daily-sync" removed
✓ Schedule "nightly-review" paused
✓ Schedule "nightly-review" resumed
```

### Error Messages

```
✗ Schedule "nightly-review" already exists
✗ Schedule "nonexistent" not found
✗ Invalid cron expression: "not a cron"
✗ Invalid timezone: "Fake/Timezone"
```

## NAPI Bridge

The slash commands need to call into the Rust/TypeScript layer for persistence. Since `spec/schedules.json` is managed by the TypeScript `LockedFileManager` (SCHED-002), the slash commands can call TypeScript functions directly:

```typescript
// src/tui/services/schedule-service.ts
import { fileManager } from '../../utils/file-manager';

export async function addSchedule(schedule: ScheduleEntry, cwd: string): Promise<void> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'schedules.json');
  
  await fileManager.transaction(filePath, (data: SchedulesFile) => {
    if (data.schedules[schedule.name]) {
      throw new Error(`Schedule "${schedule.name}" already exists`);
    }
    data.schedules[schedule.name] = schedule;
  });
}
```

### "Next Run" Calculation

The list command needs to calculate the next trigger time. Use a JavaScript cron library (e.g., `cron-parser`) for this:

```typescript
import parser from 'cron-parser';

function getNextRun(cron: string, timezone: string): Date {
  const interval = parser.parseExpression(cron, { tz: timezone });
  return interval.next().toDate();
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/tui/utils/slashCommands.ts` | Modify | Add `/schedule` to registry |
| `src/tui/utils/scheduleCommandParser.ts` | Create | Parse /schedule subcommands |
| `src/tui/services/schedule-service.ts` | Create | CRUD operations on schedules.json |
| `src/tui/components/AgentView.tsx` | Modify | Add `/schedule` dispatch in handleSubmitWithCommand |

## Key Constraints

- `/schedule` does NOT require an active AI session — it's project-level management
- Quoted strings must be handled correctly (cron expressions, prompts with spaces)
- Validation happens at the command layer — invalid cron/timezone is rejected before write
- The scheduler (Rust tokio task) picks up changes on its next 30-second tick automatically — no explicit notification needed
- All output goes through `UserNotification` StreamChunks for TUI rendering
