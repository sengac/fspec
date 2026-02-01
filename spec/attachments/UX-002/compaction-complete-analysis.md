# Complete Compaction UX Analysis - FULL RESEARCH

## ALL Sources of Compaction Messages - COMPREHENSIVE AUDIT

### 1. UI-Generated Messages (AgentView.tsx)
- **Line 3481-3484**: `setConversation` adds "[Compacting context...]" 
- **Line 3494-3500**: `setConversation` adds "[Context compacted: X→Y tokens...]"
- **Line 7456**: Retry success message (duplicate of 3498)

### 2. Rust-Generated Messages (Stream Messages) 
- **stream_loop.rs:190**: `"[Context compacted: {}→{} tokens, {:.0}% compression]"`
- **repl_loop.rs:107**: `"[Context compacted: {}→{} tokens, {:.0}% compression]"`
- These come through as status messages and are parsed by AgentView:3124

### 3. Message Processing (AgentView.tsx:3120-3130)
- Compaction messages from Rust are parsed for compactionReduction display
- They are NOT being filtered out of conversation - they show up as status messages
- Only some status messages are skipped, but compaction ones slip through

## COMPLETE PROBLEM ANALYSIS

### Issue 1: Double Message Problem
- UI adds "[Compacting context...]" immediately (line 3481)
- Rust ALSO sends "[Context compacted...]" via stream (line 190)
- BOTH end up in conversation history

### Issue 2: Input Area Dead Zone
- InputTransition shows only Text during compaction (line 321-324)
- MultiLineInput is completely hidden during compaction
- No keyboard interaction possible during compaction

### Issue 3: Incomplete State Flow
- ConversationInputArea has no isCompacting prop
- InputTransition has isCompacting but uses it wrong (Text instead of MultiLineInput)
- MultiLineInput has no compaction awareness

## REQUIRED ARCHITECTURE CHANGES - COMPLETE LIST

### 1. Remove ALL Conversation Pollution Sources
- Remove UI setConversation calls (lines 3481-3484, 3494-3500, 7456)
- Filter out Rust compaction messages in status message processing (line 3124 area)
- Ensure compaction messages ONLY show in input area

### 2. Fix Input Area Architecture 
- ConversationInputArea: Add isCompacting + compactionProgress props
- InputTransition: ALWAYS render MultiLineInput, pass compaction state through
- MultiLineInput: Accept isCompacting + compactionProgress, show in placeholder

### 3. Complete Prop Interface Changes
```typescript
// ConversationInputArea.tsx
interface ConversationInputAreaProps {
  // ... existing props
  isCompacting?: boolean;
  compactionProgress?: CompactionProgress | null;
}

// InputTransition.tsx - CHANGE BEHAVIOR
// Instead of: if (animationPhase === 'loading') return <Text>{currentDisplayText}</Text>
// Do: ALWAYS render MultiLineInput with compaction-aware placeholder

// MultiLineInput.tsx
interface MultiLineInputProps {
  // ... existing props  
  isCompacting?: boolean;
  compactionProgress?: CompactionProgress | null;
  statusMessage?: string; // For showing "Compacting: analyzing anchors... 15/32 turns"
}
```

### 4. Wiring Changes Required
- AgentView: Pass isCompacting to ConversationInputArea (line 7367 area)
- AgentView: Filter compaction messages in status processing (line 3124 area)
- ConversationInputArea: Pass compaction state to InputTransition
- InputTransition: Pass compaction state to MultiLineInput, don't replace with Text
- MultiLineInput: Show compaction status as placeholder text

## User Journey - COMPLETE FLOW

### Current (Broken) Flow:
1. User types `/compact`
2. UI adds "[Compacting context...]" to conversation
3. Input area becomes dead (Text only)
4. Rust sends "[Context compacted...]" which also appears in conversation  
5. User sees pollution in conversation AND dead input

### Fixed Flow Should Be:
1. User types `/compact`
2. Input placeholder changes to "Compacting: analyzing anchors... 15/32 turns"
3. Input area stays responsive (for navigation) but blocks typing
4. No messages appear in conversation
5. When done, input returns to normal placeholder immediately
6. Conversation stays clean

## Implementation Requirements

### Must Handle Both Message Sources:
- Remove UI setConversation calls
- Filter Rust stream messages  
- Route ALL compaction feedback to input area only

### Must Fix Component Architecture:
- Keep MultiLineInput active during compaction
- Show compaction status in placeholder, not separate Text component
- Maintain proper prop flow for state

### Must Ensure Clean Conversation:
- Zero compaction status messages in conversation history
- Only actual user/AI messages remain
- Success metrics only in transient UI areas (if at all)