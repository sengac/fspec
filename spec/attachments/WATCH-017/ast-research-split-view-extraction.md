# AST Research: Split View Extraction from AgentView.tsx

## Overview
Research conducted to understand the existing WATCH-010 split view code in AgentView.tsx for extraction into a separate SplitSessionView component.

## Current State Analysis

### WATCH-010 State Variables (lines 990-998)
```typescript
// WATCH-010: Watcher split view state
const [isWatcherSessionView, setIsWatcherSessionView] = useState(false);
const [activePane, setActivePane] = useState<'parent' | 'watcher'>('watcher');
const [parentSessionId, setParentSessionId] = useState<string | null>(null);
const [parentSessionName, setParentSessionName] = useState<string>('');
const [parentConversation, setParentConversation] = useState<ConversationLine[]>([]);
const [watcherRoleName, setWatcherRoleName] = useState<string>('');
const [isSplitViewSelectMode, setIsSplitViewSelectMode] = useState(false);
const [splitViewSelectedIndex, setSplitViewSelectedIndex] = useState(0);
```

### WATCH-010 useEffect (lines 1282-1394)
- Detects if current session is a watcher via `sessionGetParent()`
- Loads parent conversation via `sessionGetMergedOutput()`
- Subscribes to parent session updates via `sessionAttach()`
- Converts chunks to ConversationLine[] for display

### WATCH-010 Render Section (lines 6442-6613)
- Conditional render when `isWatcherSessionView` is true
- Two VirtualList components (left: parent, right: watcher)
- Header showing watcher role and parent session name
- Input area with MultiLineInput
- Keyboard hints

## Problem Analysis

The crash "t is not a function" occurs during render, not in useEffect. From logs:
1. useEffect completes successfully: "Split view setup complete"
2. Cleanup runs immediately: "Cleanup: detaching from parent session"

This indicates the component re-renders and the useEffect re-runs. The crash is in the render phase but the logging shows useEffect worked.

## Extraction Plan

### Step 1: Create minimal SplitSessionView
Start with just the static layout - no data loading, no NAPI calls:
- Header with placeholder text
- Two empty Box components for panes
- Input area

### Step 2: Add props interface
```typescript
interface SplitSessionViewProps {
  parentSessionId: string;
  childSessionId: string;
  parentSessionName: string;
  watcherRoleName: string;
  parentConversation: ConversationLine[];
  childConversation: ConversationLine[];
  onSubmit: (message: string) => void;
  isLoading: boolean;
  terminalWidth: number;
}
```

### Step 3: Build up incrementally
1. Render layout → test
2. Add header with props → test
3. Add VirtualList for parent pane → test
4. Add VirtualList for watcher pane → test
5. Add keyboard handling → test
6. Add input area → test

## Key Dependencies
- VirtualList component
- MultiLineInput component
- InputTransition component
- ConversationLine type
- NAPI functions: sessionGetParent, sessionGetMergedOutput, sessionAttach, sessionDetach, sessionGetRole, sessionManagerList
