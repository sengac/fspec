# TUI-046 AST Research: AgentView.tsx Exit Confirmation Integration Points

## Research Date
2025-01-17

## Purpose
Identify key integration points in AgentView.tsx for implementing the detach confirmation modal.

## AST Search Results

### 1. useState Hooks Location (for new state)
Pattern: `useState`
File: src/tui/components/AgentView.tsx

Key findings:
- useState imports at line 19
- State declarations start around line 595-700
- showSessionDeleteDialog state at line 691 - pattern to follow

```
src/tui/components/AgentView.tsx:691:const [showSessionDeleteDialog, setShowSessionDeleteDialog] = useState(false);
```

**Action**: Add `showExitConfirmation` state near line 691, following the same pattern.

### 2. ESC Key Handler Locations
Pattern: `key.escape`
File: src/tui/components/AgentView.tsx

```
src/tui/components/AgentView.tsx:3622:13:key.escape  (modal handlers)
src/tui/components/AgentView.tsx:3665:13:key.escape  (modal handlers)
src/tui/components/AgentView.tsx:3693:13:key.escape  (modal handlers)
src/tui/components/AgentView.tsx:3720:15:key.escape  (modal handlers)
src/tui/components/AgentView.tsx:3747:13:key.escape  (modal handlers)
src/tui/components/AgentView.tsx:3890:15:key.escape  (modal handlers)
src/tui/components/AgentView.tsx:3917:13:key.escape  (modal handlers)
src/tui/components/AgentView.tsx:4022:11:key.escape  (MAIN EXIT HANDLER - Priority 5)
```

**Action**: Modify line 4022 handler (Priority 5) to show modal instead of direct exit.

### 3. Current onExit() Call
Pattern: `onExit()`
File: src/tui/components/AgentView.tsx

```
src/tui/components/AgentView.tsx:4044:9:onExit()
```

**Action**: Replace direct `onExit()` call with modal show logic.

### 4. ThreeButtonDialog Import and Usage
Pattern: `ThreeButtonDialog`
File: src/tui/components/AgentView.tsx

```
src/tui/components/AgentView.tsx:64:10:ThreeButtonDialog  (import)
src/tui/components/AgentView.tsx:4825:12:ThreeButtonDialog (existing usage for delete)
```

**Action**: Already imported. Add second ThreeButtonDialog instance for exit confirmation.

### 5. Existing ThreeButtonDialog Pattern (TUI-040)
Pattern: `showSessionDeleteDialog`

```
src/tui/components/AgentView.tsx:691:const [showSessionDeleteDialog, setShowSessionDeleteDialog] = useState(false);
src/tui/components/AgentView.tsx:3661:if (showSessionDeleteDialog) { ... }
src/tui/components/AgentView.tsx:4824:{showSessionDeleteDialog && ( ... )}
```

**Action**: Follow this exact pattern for showExitConfirmation.

## Integration Plan

### State Addition (line ~692)
```typescript
const [showExitConfirmation, setShowExitConfirmation] = useState(false);
```

### ESC Handler Modification (line 4020-4046)
Current:
```typescript
// Priority 5: Exit view
onExit();
```

New:
```typescript
// Priority 5: Show exit confirmation if session exists
if (sessionRef.current) {
  setShowExitConfirmation(true);
} else {
  onExit();
}
```

### ThreeButtonDialog Render (after line 4824 block)
```tsx
{showExitConfirmation && (
  <ThreeButtonDialog
    message="Exit Session?"
    description={isLoading 
      ? "The agent is currently running. Choose how to exit."
      : "Choose how to exit the session."}
    options={['Detach', 'Close Session', 'Cancel']}
    defaultSelectedIndex={0}
    onSelect={handleExitChoice}
    onCancel={() => setShowExitConfirmation(false)}
  />
)}
```

## NAPI Functions Required
- `sessionDetach(sessionId)` - for Detach option
- `sessionManagerDestroy(sessionId)` - for Close Session option

Both already available in codelet-napi/index.d.ts:
```typescript
export declare function sessionAttach(sessionId: string, callback: ...): void
export declare function sessionDetach(sessionId: string): void
export declare function sessionManagerDestroy(sessionId: string): void
```

## Session ID Tracking
Need to track currentSessionId - currently AgentView uses sessionRef.current (CodeletSession object) but NAPI functions need the session ID string.

Option: Check if sessionManagerList() can identify the current session, or store ID when session is created.
