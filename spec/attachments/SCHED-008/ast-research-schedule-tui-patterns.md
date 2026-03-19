# SCHED-008: AST Research — Schedule TUI Slash Command Patterns

## Slash Command Registration Pattern (slashCommands.ts)

```typescript
export interface SlashCommand {
  name: string;           // without leading "/"
  description: string;
  syntax?: string;
  aliases?: string[];
  requiresSession?: boolean;  // default: true
}

export const SLASH_COMMANDS: SlashCommand[] = [
  { name: 'model',    description: 'Select AI model',          requiresSession: false },
  { name: 'provider', description: 'Configure API providers',  requiresSession: false },
  // ... 12 commands total
];
```

## Dispatch Pattern (AgentView.tsx handleSubmitWithCommand)

Lines 2549–2816. Linear if/else chain:
```typescript
const handleSubmitWithCommand = useCallback(async (commandText: string) => {
  const userMessage = commandText.trim();
  setInputValue('');
  
  if (userMessage === '/model') { setShowModelSelector(true); return; }
  if (userMessage === '/debug' || userMessage.startsWith('/debug ')) { ... return; }
  if (userMessage === '/thinking' || userMessage.startsWith('/thinking ')) { ... return; }
  // ... etc
  
  // fallthrough: just clears input
  setInputValue('');
}, [...]);
```

## Status Message Pattern

Success/error display in slash commands:
```typescript
setConversation(prev => [
  ...prev,
  { type: 'status', content: '✓ Some success message' },
]);
```

## Existing Schedule Commands (src/commands/schedule/)

Pure functions that can be called directly from TUI:
- `addSchedule(options)` → validates + writes to schedules.json via fileManager.transaction
- `listSchedules(options)` → reads from schedules.json via fileManager.readJSON
- `pauseSchedule(options)` → status update via fileManager.transaction
- `resumeSchedule(options)` → status update via fileManager.transaction
- `removeSchedule(options)` → delete via fileManager.transaction

All accept `cwd?: string` parameter.

## Schedule Types (src/types/schedule.ts)

```typescript
type JobType = 'agent' | 'shell';
type OverlapPolicy = 'skip' | 'queue';
type ScheduleStatus = 'active' | 'paused';

interface ScheduleEntryBase {
  name: string; cron: string; timezone: string;
  jobType: JobType; overlapPolicy: OverlapPolicy;
  status: ScheduleStatus; lastRunAt: string | null;
  lastRunStatus: 'completed' | 'failed' | 'skipped' | null;
  createdAt: string;
}

interface AgentScheduleEntry extends ScheduleEntryBase {
  jobType: 'agent'; role: string; prompt: string;
}
interface ShellScheduleEntry extends ScheduleEntryBase {
  jobType: 'shell'; command: string;
}
type ScheduleEntry = AgentScheduleEntry | ShellScheduleEntry;
```

## LockedFileManager Pattern

```typescript
import { fileManager } from '../../utils/file-manager';

// Read-only
const data = await fileManager.readJSON<SchedulesData>(filePath, defaultData);

// Read-modify-write (atomic)
await fileManager.transaction<SchedulesData>(filePath, (data) => {
  data.schedules[name] = newEntry;  // mutation-based API
});
```

## Key Integration Points

1. Add to SLASH_COMMANDS array in `src/tui/utils/slashCommands.ts`
2. Add dispatch branch in `handleSubmitWithCommand` in `AgentView.tsx`  
3. Reuse existing pure functions from `src/commands/schedule/`
4. Display via `setConversation(prev => [...prev, { type: 'status', content }])`
