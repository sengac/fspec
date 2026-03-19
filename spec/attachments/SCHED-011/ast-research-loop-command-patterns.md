# AST Research: Loop Command Integration Patterns

## Slash Command Dispatch Pattern (AgentView.tsx)

Guard pattern: `userMessage === '/cmd' || userMessage.startsWith('/cmd ')`
- setInputValue('') first
- Dynamic import of service module
- Service returns `{ success: boolean, message: string }`
- Display via `setConversation(prev => [...prev, { type: 'status', content: result.message }])`
- /schedule handler at lines 2806-2823 (last before catch-all)

## Session Gating Pattern

/loop requires session (session-scoped). Pattern from /thinking (lines 2751-2761):
- Check `currentSessionId` exists before executing
- Show error status message if no session

## Schedule Service Pattern (schedule-service.ts)

- Parse via `parseScheduleCommand(input)` → `ParsedScheduleCommand`
- Route via `switch (parsed.subcommand)`
- Validate required fields
- Delegate to CRUD functions
- Format ✓/✗ response messages
- Catch errors → `{ success: false, message }`

## /loop Differences from /schedule

| Aspect | /schedule | /loop |
|--------|-----------|-------|
| Storage | schedules.json via LockedFileManager | In-memory Map |
| Scope | Global (cwd-based) | Session-scoped |
| requiresSession | false | true |
| Interval syntax | --cron flag | Natural language (5m, every 2h) |
| Persistence | Survives restart | Dies with session |
| Overlap default | configurable | skip (always) |

## Key Files to Create

1. `src/tui/utils/loopCommandParser.ts` — interval parsing, /loop syntax
2. `src/tui/services/loop-service.ts` — in-memory schedule management
3. Register in `src/tui/utils/slashCommands.ts`
4. Dispatch in `src/tui/components/AgentView.tsx`
