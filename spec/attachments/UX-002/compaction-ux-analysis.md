# Compaction UX Analysis - Current Issues

## Current Implementation Problems

### 1. Conversation Pollution
- `setConversation` adds `[Compacting context...]` status messages to conversation history
- Found at AgentView.tsx:3481-3484
- These status messages pollute the actual conversation between user and AI

### 2. Input Area Goes Completely Dead During Compaction
- `InputTransition.tsx` line 321-324: Returns only `<Text>` component during compaction
- `MultiLineInput` is NOT rendered during compaction (line 340+ only reached when animationPhase === 'complete')  
- User cannot press ESC, cannot type, cannot navigate - NO keyboard interaction possible
- This is architectural flaw: `suppressEnter` only works when `MultiLineInput` is actually rendered

### 3. Missing State Propagation
- `MultiLineInput.tsx` has no awareness of compaction state
- It only receives `suppressEnter` prop but during compaction it's not even rendered
- Should receive `isCompacting` prop to show appropriate placeholder and handle interactions

## Required Architecture Changes

### 1. Remove Conversation Pollution
- Remove `setConversation` calls for compaction status
- All compaction feedback should be in input area only

### 2. Keep MultiLineInput Active During Compaction  
- `InputTransition` should ALWAYS render `MultiLineInput`
- Pass `isCompacting` prop to `MultiLineInput`
- `MultiLineInput` should show compaction status in placeholder area
- `MultiLineInput` should handle ESC for cancellation during compaction
- Block typing but keep keyboard navigation active

### 3. Proper State Flow
```
AgentView (compaction state) 
→ ConversationInputArea (passes state)
→ InputTransition (passes state)  
→ MultiLineInput (receives isCompacting prop)
```

## Integration Points (WHO CALLS THIS?)

### Components that need updates:
1. **AgentView.tsx**: Remove setConversation calls for compaction, pass isCompacting to ConversationInputArea
2. **ConversationInputArea.tsx**: Add isCompacting prop and pass to InputTransition  
3. **InputTransition.tsx**: ALWAYS render MultiLineInput, pass isCompacting prop
4. **MultiLineInput.tsx**: Add isCompacting prop, show compaction status in placeholder, handle ESC

### Props Interface Changes:
```typescript
// ConversationInputArea.tsx
interface ConversationInputAreaProps {
  // ... existing props
  isCompacting?: boolean;
  compactionProgress?: CompactionProgress;
}

// MultiLineInput.tsx  
interface MultiLineInputProps {
  // ... existing props
  isCompacting?: boolean;
  compactionProgress?: CompactionProgress;
  onCancelCompaction?: () => void;
}
```

## User Experience Requirements

### What User Should See:
- Input area shows: "Analyzing anchors... 15/32 turns" as placeholder
- ESC key cancels compaction (if allowed)
- No status messages in conversation history
- Input appears disabled but still responsive to cancellation

### What User Should NOT See:
- `[Compacting context...]` in conversation
- Dead/unresponsive input area
- No way to cancel compaction