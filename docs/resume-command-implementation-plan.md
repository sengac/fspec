# /resume Command Implementation Plan

## Overview

Implement a `/resume` command that displays a list of sessions ordered by most recently modified, allowing users to select and restore a session into the current context. The UI should be similar to the existing `/search` command.

## Codebase Analysis

### Current Session Persistence Architecture

**Data Storage Locations** (`~/.fspec/`):
- `sessions/*.json` - Individual session manifests
- `messages/messages.jsonl` - Content-addressed message store
- `history.jsonl` - Command history (JSONL)
- `blobs/` - Large content blob storage

**Key Types** (`codelet/napi/src/persistence/types.rs`):
```rust
pub struct SessionManifest {
    pub id: Uuid,
    pub name: String,
    pub project: PathBuf,
    pub provider: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<MessageRef>,
    pub forked_from: Option<ForkPoint>,
    pub merged_from: Vec<MergeRecord>,
    pub compaction: Option<CompactionState>,
    pub token_usage: TokenUsage,
}
```

**TypeScript Interface** (`codelet/napi/index.d.ts`):
```typescript
export interface NapiSessionManifest {
  id: string;
  name: string;
  project: string;
  provider: string;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
  forkedFrom?: NapiForkPoint;
  mergedFrom: Array<NapiMergeRecord>;
  compaction?: NapiCompactionState;
  tokenUsage: NapiTokenUsage;
}
```

### Existing NAPI Functions

From `codelet/napi/src/persistence/napi_bindings.rs`:
- `persistenceListSessions(project: string)` - Lists all sessions for a project
- `persistenceLoadSession(id: string)` - Loads a specific session by ID
- `persistenceGetSessionMessages(sessionId: string)` - Gets all messages for a session
- `persistenceResumeLastSession(project: string)` - Resumes the last session (current `/resume` behavior)

### Current /resume Command (Basic)

Location: `src/tui/components/AgentModal.tsx:472-490`

Current behavior just resumes the LAST session without showing a selection UI:
```typescript
if (userMessage === '/resume') {
  const { persistenceResumeLastSession } = await import('codelet-napi');
  const session = persistenceResumeLastSession(currentProjectRef.current);
  setCurrentSessionId(session.id);
  // Shows basic message, no UI
}
```

### /search Mode UI Pattern (Reference)

Location: `src/tui/components/AgentModal.tsx:1365-1414`

The search mode provides a template for the resume UI:
- Full-screen overlay with magenta border
- Search input field at top
- List of results with arrow key navigation
- Selection highlighting
- Enter to select, Escape to cancel

## Implementation Plan

### Phase 1: TypeScript UI (AgentModal.tsx)

#### 1.1 Add Resume Mode State Variables

```typescript
// NAPI-003: Resume mode state
const [isResumeMode, setIsResumeMode] = useState(false);
const [availableSessions, setAvailableSessions] = useState<SessionManifest[]>([]);
const [resumeSessionIndex, setResumeSessionIndex] = useState(0);
```

#### 1.2 Create handleResumeMode Function

```typescript
const handleResumeMode = useCallback(async () => {
  try {
    const { persistenceListSessions } = await import('codelet-napi');
    const sessions = persistenceListSessions(currentProjectRef.current);

    // Sort by updatedAt descending (most recent first)
    const sorted = sessions.sort((a: SessionManifest, b: SessionManifest) =>
      new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
    );

    if (sorted.length === 0) {
      setConversation(prev => [
        ...prev,
        { role: 'tool', content: 'No sessions found for this project' },
      ]);
      return;
    }

    setAvailableSessions(sorted);
    setResumeSessionIndex(0);
    setIsResumeMode(true);
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : 'Failed to list sessions';
    setConversation(prev => [
      ...prev,
      { role: 'tool', content: `Resume failed: ${errorMessage}` },
    ]);
  }
}, []);
```

#### 1.3 Update /resume Command Handler

Replace the current `/resume` handler (~line 472) to call `handleResumeMode()`:

```typescript
if (userMessage === '/resume') {
  setInputValue('');
  handleResumeMode();
  return;
}
```

#### 1.4 Add Resume Selection Handler

```typescript
const handleResumeSelect = useCallback(async () => {
  if (availableSessions.length === 0 || resumeSessionIndex >= availableSessions.length) {
    return;
  }

  const selectedSession = availableSessions[resumeSessionIndex];

  try {
    const { persistenceGetSessionMessages } = await import('codelet-napi');
    const messages = persistenceGetSessionMessages(selectedSession.id);

    // Convert stored messages to conversation format
    const restored: ConversationMessage[] = messages.map((m: NapiStoredMessage) => ({
      role: m.role === 'user' ? 'user' : 'assistant',
      content: m.content,
      isStreaming: false,
    }));

    // Update state
    setCurrentSessionId(selectedSession.id);
    setConversation(restored);
    setIsResumeMode(false);

    // Add confirmation message
    setConversation(prev => [
      ...prev,
      { role: 'tool', content: `Session resumed: "${selectedSession.name}" (${selectedSession.messageCount} messages)` },
    ]);
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : 'Failed to restore session';
    setConversation(prev => [
      ...prev,
      { role: 'tool', content: `Resume failed: ${errorMessage}` },
    ]);
    setIsResumeMode(false);
  }
}, [availableSessions, resumeSessionIndex]);

const handleResumeCancel = useCallback(() => {
  setIsResumeMode(false);
  setAvailableSessions([]);
  setResumeSessionIndex(0);
}, []);
```

#### 1.5 Add Keyboard Handling for Resume Mode

In the `useInput` hook, add handling for resume mode:

```typescript
// NAPI-003: Resume mode keyboard handling
if (isResumeMode) {
  if (key.escape) {
    handleResumeCancel();
    return;
  }
  if (key.return) {
    handleResumeSelect();
    return;
  }
  if (key.upArrow) {
    setResumeSessionIndex(prev => Math.max(0, prev - 1));
    return;
  }
  if (key.downArrow) {
    setResumeSessionIndex(prev => Math.min(availableSessions.length - 1, prev + 1));
    return;
  }
  return;
}
```

#### 1.6 Add Resume Mode Overlay UI

Add after the search mode overlay (~line 1414):

```tsx
// NAPI-003: Resume mode overlay (session selection)
if (isResumeMode) {
  return (
    <Box
      position="absolute"
      flexDirection="column"
      width={terminalWidth}
      height={terminalHeight}
    >
      <Box
        flexDirection="column"
        flexGrow={1}
        borderStyle="double"
        borderColor="blue"
        backgroundColor="black"
      >
        <Box
          flexDirection="column"
          padding={2}
          flexGrow={1}
        >
          <Box marginBottom={1}>
            <Text bold color="blue">
              Resume Session ({availableSessions.length} available)
            </Text>
          </Box>
          {availableSessions.length === 0 && (
            <Box>
              <Text dimColor>No sessions found for this project</Text>
            </Box>
          )}
          {availableSessions.slice(0, 15).map((session, idx) => {
            const isSelected = idx === resumeSessionIndex;
            const updatedAt = new Date(session.updatedAt);
            const timeAgo = formatTimeAgo(updatedAt);
            return (
              <Box key={session.id} flexDirection="column">
                <Text
                  backgroundColor={isSelected ? 'blue' : undefined}
                  color={isSelected ? 'black' : 'white'}
                >
                  {isSelected ? '> ' : '  '}
                  {session.name}
                </Text>
                <Text
                  backgroundColor={isSelected ? 'blue' : undefined}
                  color={isSelected ? 'black' : 'gray'}
                  dimColor={!isSelected}
                >
                  {'    '}
                  {session.messageCount} messages | {session.provider || 'unknown'} | {timeAgo}
                </Text>
              </Box>
            );
          })}
          <Box marginTop={1}>
            <Text dimColor>Enter Select | Arrow Navigate | Esc Cancel</Text>
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
```

#### 1.7 Add Helper Function for Time Formatting

```typescript
// NAPI-003: Format time relative to now
const formatTimeAgo = (date: Date): string => {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return 'just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
};
```

### Phase 2: Context Injection (Optional Enhancement)

The above implementation shows the conversation history visually, but the AI doesn't have this context loaded. For true session resumption with AI context:

#### 2.1 Rust-Side: Add Message Injection Function

In `codelet/napi/src/session.rs`, add a method to inject messages:

```rust
/// Load messages from a persisted session into the current context
/// This allows the AI to have full conversation history after resume
#[napi]
pub fn load_session_messages(&mut self, messages: Vec<Message>) -> Result<()> {
    // Clear current messages
    self.messages.clear();

    // Inject the loaded messages
    for msg in messages {
        self.messages.push(msg.into());
    }

    // Reinject context reminders at the end
    self.inject_context_reminders();

    Ok(())
}
```

#### 2.2 TypeScript-Side: Call loadSessionMessages on Resume

```typescript
const handleResumeSelect = useCallback(async () => {
  // ... existing code ...

  // If session ref exists, inject messages into Rust session
  if (sessionRef.current && restored.length > 0) {
    const rustMessages = restored.map(m => ({
      role: m.role === 'user' ? 'User' : 'Assistant',
      content: m.content,
    }));
    sessionRef.current.loadSessionMessages(rustMessages);
  }
}, [availableSessions, resumeSessionIndex]);
```

### Phase 3: Testing

#### 3.1 Unit Tests

Create `src/tui/__tests__/AgentModal-resume.test.tsx`:

```typescript
describe('AgentModal /resume command', () => {
  it('opens resume mode when /resume is entered', async () => {
    // Test that entering /resume opens the session selection UI
  });

  it('lists sessions sorted by updatedAt descending', async () => {
    // Test that sessions are sorted most recent first
  });

  it('navigates sessions with arrow keys', async () => {
    // Test up/down arrow navigation
  });

  it('selects session and restores conversation on Enter', async () => {
    // Test that selecting a session restores the conversation
  });

  it('cancels resume mode on Escape', async () => {
    // Test that Escape closes resume mode
  });

  it('shows "no sessions" message when none exist', async () => {
    // Test empty state handling
  });
});
```

### File Changes Summary

| File | Changes |
|------|---------|
| `src/tui/components/AgentModal.tsx` | Add resume mode state, handlers, UI overlay |
| `codelet/napi/src/session.rs` | (Optional) Add loadSessionMessages method |
| `codelet/napi/index.d.ts` | (Optional) Add loadSessionMessages type definition |
| `src/tui/__tests__/AgentModal-resume.test.tsx` | New test file |

### Dependencies

- **NAPI-002**: Session Persistence with Fork and Merge (provides the underlying persistence infrastructure)
- Uses existing `persistenceListSessions` and `persistenceGetSessionMessages` functions

### Acceptance Criteria

1. `/resume` command opens a full-screen session selection overlay
2. Sessions are listed sorted by most recently modified first
3. Each session shows: name, message count, provider, time since last update
4. Arrow keys navigate the session list
5. Enter selects and restores the session conversation
6. Escape cancels and returns to normal input mode
7. Restored session displays previous conversation messages
8. Session ID is updated to the selected session for future message persistence
9. Empty state shows helpful message when no sessions exist

### UI/UX Design

```
┌══════════════════════════════════════════════════════════════════════┐
║ Resume Session (5 available)                                          ║
║                                                                       ║
║ > Session Dec 23                                                      ║
║     12 messages | claude | 5m ago                                    ║
║   Feature implementation discussion                                   ║
║     8 messages | claude | 2h ago                                     ║
║   Bug investigation                                                   ║
║     23 messages | claude | 1d ago                                    ║
║   Code review feedback                                                ║
║     4 messages | gemini | 3d ago                                     ║
║   Architecture planning                                               ║
║     15 messages | claude | 1w ago                                    ║
║                                                                       ║
║ Enter Select | Arrow Navigate | Esc Cancel                           ║
╚══════════════════════════════════════════════════════════════════════╝
```

### Implementation Order

1. Add state variables and types
2. Implement `handleResumeMode` to load and sort sessions
3. Update `/resume` command handler
4. Add keyboard handling for resume mode
5. Implement `handleResumeSelect` to restore conversation
6. Add the resume mode overlay UI
7. Add helper functions (formatTimeAgo)
8. Write tests
9. (Optional) Implement Phase 2 for AI context injection
