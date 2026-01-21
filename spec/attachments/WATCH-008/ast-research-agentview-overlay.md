# AST Research: AgentView Overlay Patterns for WATCH-008

## Research Goal
Understand existing overlay patterns in AgentView.tsx to implement Watcher Management overlay.

## Findings

### 1. NAPI Watcher Functions Available

From `codelet/napi/index.d.ts`:

```typescript
// Create watcher session for a parent
export declare function sessionCreateWatcher(
  parentId: string, 
  model: string, 
  project: string, 
  name: string
): Promise<string>

// Get parent session ID for a watcher (null if not a watcher)
export declare function sessionGetParent(sessionId: string): string | null

// Get all watcher session IDs for a parent
export declare function sessionGetWatchers(sessionId: string): Array<string>

// Inject watcher message into parent session
export declare function watcherInject(watcherId: string, message: string): void

// Role management
export declare function sessionSetRole(
  sessionId: string, 
  roleName: string, 
  roleDescription: string | undefined | null, 
  authority: string
): void

export declare function sessionGetRole(sessionId: string): SessionRoleInfo | null

export interface SessionRoleInfo {
  name: string;
  description: string | null;
  authority: string; // "peer" or "supervisor"
}
```

### 2. Existing Overlay Pattern (isResumeMode)

State variables (around line 950):
```typescript
const [isResumeMode, setIsResumeMode] = useState(false);
const [availableSessions, setAvailableSessions] = useState<MergedSession[]>([]);
const [resumeSessionIndex, setResumeSessionIndex] = useState(0);
const [resumeScrollOffset, setResumeScrollOffset] = useState(0);
```

Command handler (line 1697):
```typescript
if (userMessage === '/resume') {
  setInputValue('');
  void handleResumeMode();
  return;
}
```

Overlay render (line 5642-5776):
```typescript
if (isResumeMode) {
  return (
    <Box position="absolute" flexDirection="column" width={terminalWidth} height={terminalHeight}>
      <Box flexDirection="column" flexGrow={1} backgroundColor="black">
        {/* Header */}
        <Box marginBottom={1}>
          <Text bold color="blue">Resume Session ({count} available)</Text>
        </Box>
        {/* Scrollable list */}
        <Box flexDirection="row" flexGrow={1}>
          {/* List items */}
          {/* Scrollbar */}
        </Box>
        {/* Key hints */}
        <Box marginTop={1}>
          <Text dimColor>Enter Select | ↑↓ Navigate | D Delete | Esc Cancel</Text>
        </Box>
      </Box>
    </Box>
  );
}
```

### 3. Key Handling Pattern

Located in useInput hook (around line 4930):
```typescript
useInput((input, key) => {
  // Priority handling for modals/overlays
  if (isResumeMode) {
    if (key.escape) {
      setIsResumeMode(false);
      return;
    }
    if (key.upArrow) {
      setResumeSessionIndex(prev => Math.max(0, prev - 1));
      return;
    }
    // ... etc
  }
});
```

### 4. Confirmation Dialog Pattern

Uses ThreeButtonDialog component:
```typescript
{showSessionDeleteDialog && (
  <ThreeButtonDialog
    message={`Delete session "${sessionName}"?`}
    options={['Delete This Session', 'Delete ALL Sessions', 'Cancel']}
    onSelect={handleSessionDeleteSelect}
    onCancel={handleSessionDeleteCancel}
  />
)}
```

### 5. Integration Points

| Line | File | Description |
|------|------|-------------|
| ~950 | AgentView.tsx | Add isWatcherMode, watcherList, watcherIndex state |
| ~1696 | AgentView.tsx | Add /watcher command handler |
| ~4930 | AgentView.tsx | Add watcher mode key handling |
| ~5776 | AgentView.tsx | Add watcher overlay render block |
| imports | AgentView.tsx | Import sessionGetWatchers, sessionGetRole |

## Conclusion

The pattern is clear:
1. State variables for mode flag, list data, selection index, scroll offset
2. Command handler sets mode flag and loads data
3. Render block checks mode flag and returns overlay JSX
4. useInput handles keyboard navigation specific to mode
5. Escape closes overlay, Enter selects item
