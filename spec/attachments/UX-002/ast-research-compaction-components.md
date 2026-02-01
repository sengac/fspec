# AST Research - Compaction UX Components Analysis

## Research Query: Component Architecture for Compaction State Flow

### Components Analyzed

#### 1. MultiLineInput Component Analysis
```typescript
// File: src/tui/components/MultiLineInput.tsx
// Current interface - needs isCompacting props
export interface MultiLineInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  isActive?: boolean;
  maxVisibleLines?: number;
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
  suppressEnter?: boolean;
}
// MISSING: isCompacting?: boolean; compactionProgress?: CompactionProgress | null;
```

#### 2. InputTransition Component Analysis
```typescript
// File: src/tui/components/InputTransition.tsx
// Lines 321-324: Problem area
if (animationPhase === 'loading') {
  return <Text dimColor>{currentDisplayText}</Text>; // ← REPLACES MultiLineInput!
}
// Should ALWAYS render MultiLineInput with compaction-aware placeholder
```

#### 3. ConversationInputArea Component Analysis
```typescript
// File: src/tui/components/ConversationInputArea.tsx
// Current interface - missing compaction props
export interface ConversationInputAreaProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: (value: string) => void;
  isLoading: boolean;
  placeholder?: string;
  isActive?: boolean;
  skipAnimation?: boolean;
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
  maxVisibleLines?: number;
  promptChar?: string;
  promptColor?: string;
  isPaused?: boolean;
  pauseInfo?: PauseInfo;
  suppressEnter?: boolean;
}
// MISSING: isCompacting?: boolean; compactionProgress?: CompactionProgress | null;
```

#### 4. AgentView Message Pollution Sources
```typescript
// File: src/tui/components/AgentView.tsx
// Line 3481-3484: UI-generated compaction start message
setConversation(prev => [
  ...prev,
  { type: 'status', content: '[Compacting context...]' },
]);

// Line 3494-3500: UI-generated compaction success message  
setConversation(prev => [
  ...prev,
  {
    type: 'status',
    content: `[Context compacted: ${result.originalTokens}→${result.compactedTokens} tokens...]`,
  },
]);

// Line 7456: Duplicate retry success message
content: `[Context compacted: ${result.originalTokens}→${result.compactedTokens} tokens...]`,

// Line 3120-3130: Status message processing (where Rust messages appear)
const compactionMatch = statusMessage.match(/Context compacted:.*?(\d+)% compression/);
if (compactionMatch) {
  const reductionPct = parseInt(compactionMatch[1], 10);
  setCompactionReduction(reductionPct);
} 
// These Rust messages are NOT being filtered out of conversation!
```

### Rust Message Sources
```rust
// File: codelet/cli/src/interactive/stream_loop.rs:190
println!(
    "[Context compacted: {}→{} tokens, {:.0}% compression]",
    metrics.original_tokens, metrics.compacted_tokens, compression_ratio
);

// File: codelet/cli/src/interactive/repl_loop.rs:107
println!(
    "[Context compacted: {}→{} tokens, {:.0}% compression]",
    result.original_tokens, result.compacted_tokens, compression_ratio
);
```

## Integration Points Analysis

### Current Component Chain:
```
AgentView 
  → ConversationInputArea (missing compaction props)
    → InputTransition (has compaction props but wrong usage)
      → MultiLineInput (missing compaction props) | Text (wrong component)
```

### Required Component Chain:
```
AgentView (filter messages + pass compaction state)
  → ConversationInputArea (accept + pass compaction props)
    → InputTransition (accept + pass compaction props, ALWAYS render MultiLineInput)
      → MultiLineInput (accept compaction props, show in placeholder)
```

### Props Flow Requirements:
```typescript
// AgentView.tsx needs to pass:
isCompacting={compaction.progressState.isActive || rustSnapshot.isCompacting}
compactionProgress={compaction.progressState.isActive 
  ? { phase: compaction.progressState.message, current: X, total: Y }
  : rustSnapshot.compactionProgress}

// To ConversationInputArea, which passes to InputTransition, which passes to MultiLineInput
```

## Implementation Plan

### 1. MultiLineInput Changes
- Add isCompacting and compactionProgress to props interface
- Modify placeholder logic to show compaction status when isCompacting=true
- Format: "Compacting: analyzing anchors... 15/32 turns" 

### 2. InputTransition Changes  
- Remove conditional Text rendering on line 321-324
- ALWAYS render MultiLineInput, pass compaction props through
- Let MultiLineInput handle compaction display internally

### 3. ConversationInputArea Changes
- Add compaction props to interface
- Pass compaction props to InputTransition

### 4. AgentView Changes
- Remove setConversation calls for compaction (lines 3481-3484, 3494-3500, 7456)
- Filter Rust compaction messages in status processing (line 3124 area)
- Pass compaction state to ConversationInputArea

## Testing Requirements

### Component Tests Needed:
1. MultiLineInput shows compaction status in placeholder when isCompacting=true
2. InputTransition always renders MultiLineInput during compaction (not Text)
3. ConversationInputArea passes compaction props correctly
4. AgentView filters out compaction status messages from conversation
5. Integration test: end-to-end compaction state flow

### Scenario Coverage:
- Input placeholder shows "Compacting: analyzing anchors... X/Y turns"
- Conversation history remains clean (no compaction messages)
- Input area stays interactive but blocks typing
- State transitions work correctly when compaction completes