# TUI-047 AST Research: Resume View Integration Points

## Research Date
2025-01-17

## Purpose
Identify key integration points for attaching to detached sessions from /resume view.

## AST Search Results

### 1. handleResumeMode Location
Pattern: `handleResumeMode`
```
src/tui/components/AgentView.tsx:1352:12:handleResumeMode  (call site in /resume command)
src/tui/components/AgentView.tsx:2964:9:handleResumeMode   (function definition)
```

**Action**: Modify to also call sessionManagerList() and merge results.

### 2. handleResumeSelect Location  
Pattern: `handleResumeSelect`
```
src/tui/components/AgentView.tsx:2989:9:handleResumeSelect   (function definition)
src/tui/components/AgentView.tsx:3712:16:handleResumeSelect  (Enter key handler call site)
```

**Action**: Add branch for background sessions that calls sessionAttach() instead of persistenceLoadSession().

### 3. handleSessionDeleteSelect Location
Pattern: `handleSessionDeleteSelect`
```
src/tui/components/AgentView.tsx:3374:9:handleSessionDeleteSelect   (function definition)
src/tui/components/AgentView.tsx:4879:23:handleSessionDeleteSelect  (ThreeButtonDialog onSelect)
```

**Action**: Add branch to call sessionManagerDestroy() for background sessions.

### 4. NAPI Functions (NOT in AgentView yet)
Pattern: `sessionManagerList` - No matches (needs to be imported)
Pattern: `sessionAttach` - No matches (needs to be imported)

These need to be dynamically imported from @sengac/codelet-napi.

## Available NAPI Functions (from codelet/napi/index.d.ts)

```typescript
// List background sessions
sessionManagerList(): Array<SessionInfo>

interface SessionInfo {
  id: string
  name: string
  status: string  // "running" or "idle"
  project: string
  messageCount: number
}

// Attach to session for live streaming
sessionAttach(sessionId: string, callback: ((err: Error | null, arg: StreamChunk) => any)): void

// Get buffered output
sessionGetBufferedOutput(sessionId: string, limit: number): Array<StreamChunk>

// Get session status
sessionGetStatus(sessionId: string): string
```

## Integration Plan

### 1. MergedSession Interface (new)
```typescript
interface MergedSession extends SessionManifest {
  isBackgroundSession: boolean;
  backgroundStatus: 'running' | 'idle' | null;
}
```

### 2. mergeSessionLists() Helper (new)
```typescript
function mergeSessionLists(
  persisted: NapiSessionManifest[],
  background: SessionInfo[]
): MergedSession[]
```

### 3. getStatusIcon() Helper (new)
```typescript
function getStatusIcon(session: MergedSession): string {
  if (session.isBackgroundSession) {
    return session.backgroundStatus === 'running' ? '🔄' : '⏸️';
  }
  return '💾';
}
```

### 4. handleResumeMode() Modification (line 2964)
- Add import for sessionManagerList
- Call both persistenceListSessions() and sessionManagerList()
- Use mergeSessionLists() to combine
- Store merged list in state

### 5. handleResumeSelect() Modification (line 2989)
- Check if selected session isBackgroundSession && status === 'running'
- If yes: call attachToBackgroundSession()
- If no: existing persistenceLoadSession() code path

### 6. attachToBackgroundSession() Helper (new)
```typescript
async function attachToBackgroundSession(sessionId: string) {
  // 1. Get buffered output
  const buffered = sessionGetBufferedOutput(sessionId, 10000);
  // 2. Hydrate conversation from buffer
  for (const chunk of buffered) handleStreamChunk(chunk);
  // 3. Attach for live streaming
  sessionAttach(sessionId, (err, chunk) => handleStreamChunk(chunk));
}
```

### 7. handleSessionDeleteSelect() Modification (line 3374)
- Check if selected session isBackgroundSession
- If yes: call sessionManagerDestroy() instead of persistenceDeleteSession()

## Resume List Rendering Update

Current (line ~4757):
```tsx
<Text>{session.name}</Text>
```

New:
```tsx
<Text>{getStatusIcon(session)} {session.name}</Text>
```
