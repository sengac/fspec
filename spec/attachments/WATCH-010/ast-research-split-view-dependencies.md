# AST Research: Watcher Split View Dependencies

## Overview
Research conducted using `fspec research --tool=ast` to understand code patterns for implementing the watcher split view in AgentView.tsx.

## NAPI Bindings Available

### sessionGetParent (codelet/napi/index.d.ts:919)
```typescript
export declare function sessionGetParent(sessionId: string): string | null
```
- Returns parent session ID if session is a watcher, null otherwise
- Used to detect if current session should render in split-view mode

### sessionGetMergedOutput (5 usages in AgentView.tsx)
```
src/tui/components/AgentView.tsx:68:3:sessionGetMergedOutput (import)
src/tui/components/AgentView.tsx:3531:20:sessionGetMergedOutput
src/tui/components/AgentView.tsx:3652:30:sessionGetMergedOutput  
src/tui/components/AgentView.tsx:3920:28:sessionGetMergedOutput
src/tui/components/AgentView.tsx:4037:30:sessionGetMergedOutput
```
- Used to get buffered output for a session
- Will be used to load parent session conversation into left pane

## VirtualList Component Usage
```
src/tui/components/AgentView.tsx:28:10:VirtualList (import)
src/tui/components/AgentView.tsx:6336:10:VirtualList (usage in render)
```
- Existing VirtualList component handles scrollable conversation display
- Split view will need two VirtualList instances (one per pane)

## Existing Watcher State Variables (lines 978-987)
From earlier grep search:
```typescript
// WATCH-008: Watcher management overlay state
const [isWatcherMode, setIsWatcherMode] = useState(false);
const [watcherList, setWatcherList] = useState<WatcherInfo[]>([]);
const [watcherIndex, setWatcherIndex] = useState(0);
const [watcherScrollOffset, setWatcherScrollOffset] = useState(0);
const [showWatcherDeleteDialog, setShowWatcherDeleteDialog] = useState(false);
const [isWatcherEditMode, setIsWatcherEditMode] = useState(false);
const [watcherEditValue, setWatcherEditValue] = useState('');
// WATCH-009: Watcher creation view state
const [isWatcherCreateMode, setIsWatcherCreateMode] = useState(false);
```

## New State Variables Needed for Split View
```typescript
// WATCH-010: Split view state
const [activePane, setActivePane] = useState<'parent' | 'watcher'>('watcher');
const [parentSessionId, setParentSessionId] = useState<string | null>(null);
const [parentSessionName, setParentSessionName] = useState<string>('');
const [parentConversation, setParentConversation] = useState<ConversationLine[]>([]);
```

## Keyboard Handling Pattern
The useInput hook in AgentView handles various modes (isWatcherMode, isWatcherEditMode, etc.).
Split view keyboard handling should be added as another mode condition.

## Rendering Pattern
AgentView uses conditional rendering based on current mode:
- `if (isWatcherCreateMode)` - renders WatcherCreateView
- `if (isWatcherMode)` - renders watcher management overlay
- Default - renders normal AgentView

Split view rendering should check:
```typescript
const isWatcherSession = parentSessionId !== null;
// Then conditionally render split view or single pane
```

## Integration Points
1. Import `sessionGetParent` from codelet-napi (not yet imported)
2. Check for parent session when session changes (in useEffect)
3. Load parent conversation using `sessionGetMergedOutput(parentSessionId)`
4. Subscribe to parent session updates via `sessionAttach`
