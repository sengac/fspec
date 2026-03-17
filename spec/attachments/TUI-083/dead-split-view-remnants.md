# Dead Split-View & Cross-Pane Correlation Remnants

Remnants from the removed `SplitSessionView` / `correlationMapping` system (WATCH-011).
These are all dead code — no longer functional after TUI-080 removed the split screen.

---

## Production Code

### 1. `src/tui/types/conversation.ts`
- **Lines 25-27**: `correlationId` field + `WATCH-011` comment on `ConversationMessage`
- **Lines 40-42**: `correlationId` field + `WATCH-011` comment on `ConversationLine`

### 2. `src/tui/utils/conversationUtils.ts`
- **Lines 81-82**: `// WATCH-011: Propagate correlation fields` + `correlationId` extraction
- **Lines 106, 117, 133**: `correlationId` propagated into every output line

### 3. `src/tui/utils/thinkingBlockManager.ts`
- **Line 46-47**: `correlationId` field in `AppendThinkingOptions` interface
- **Lines 172-173**: Conditional correlationId assignment on active message
- **Line 193**: `correlationId` in new message object
- **Lines 269-270**: Same pattern in append path
- **Line 285**: `correlationId` in fallback message object

### 4. `src/tui/utils/chunkProcessor.ts`
- **Line 229**: `const correlationId = chunk.correlationId`
- **Lines 236, 241, 265, 271, 313, 344**: `correlationId` threaded through every message creation path

### 5. `src/tui/components/AgentView.tsx`
- **Lines 204-205**: `// WATCH-011: Correlation IDs for cross-pane selection highlighting` + `correlationId` field in message interface
- **Lines 286-287**: `// WATCH-011: Extract correlation fields from chunk` + extraction
- **Lines 294, 306, 314, 323, 341, 347, 390, 457, 495, 503**: `correlationId` propagated through all message/chunk handling branches

### 6. `src/tui/hooks/useLazyConversationLines.ts`
- **Lines 81, 87**: `isSupervisorView` parameter (default `false`, never passed as `true` anymore)
- **Lines 93, 103, 108, 121, 124**: `prevSupervisorViewRef` tracking and `supervisorViewChanged` invalidation logic
- **Line 151**: `isSupervisorView` in dependency array

---

## Test Files (dead references)

### 7. `src/__tests__/unified-view-navigation.test.ts`
- **Line 154**: `// @step Given I am viewing a watcher in SplitSessionView`

### 8. `src/tui/__tests__/watcher-session-header-indicator.test.tsx`
- Entire file tests watcher-specific session header indicator behaviour (split-view era)

### 9. `src/tui/utils/__tests__/thinkingBlockManager.test.ts`
- **Lines 319-353**: Tests for `correlationId` propagation in `appendThinking`

### 10. `src/tui/components/__tests__/remove-dead-split-view.test.ts`
- This test verified TUI-080 removal — can be removed now that the cleanup is long done

---

## Cleanup Strategy

1. Remove `correlationId` from `ConversationMessage` and `ConversationLine` interfaces
2. Remove all `correlationId` propagation in `conversationUtils.ts`, `thinkingBlockManager.ts`, `chunkProcessor.ts`, `AgentView.tsx`
3. Remove `correlationId` from `AppendThinkingOptions`
4. Remove `isSupervisorView` parameter from `useLazyConversationLines.ts`
5. Remove/update dead test references
6. Run full test suite to confirm nothing depends on these fields
