# AST Research: Watcher Session Header Components

## Research Summary

### 1. SplitSessionViewProps Interface Location
- File: `src/tui/components/SplitSessionView.tsx`
- Line: 34
- Current props need extension to include model/token display data

### 2. Utility Functions to Reuse

#### getContextFillColor
- File: `src/tui/components/AgentView.tsx`
- Line: 1220
- Purpose: Returns color based on context fill percentage (green/yellow/red)
- Needs to be extracted or imported by SplitSessionView

#### formatContextWindow
- File: `src/tui/components/AgentView.tsx`  
- Line: 261
- Purpose: Formats context window numbers (200000 → "200k")
- Needs to be extracted or imported by SplitSessionView

### 3. Current Header Implementation
- SplitSessionView header (lines 258-271) shows minimal info:
  - Watcher role name
  - Parent session name
  - Loading indicator
- Missing: model capabilities, context window, token usage, context fill %

### 4. AgentView Header Pattern (lines 6488-6554)
- Shows: Model name, [R] reasoning, [V] vision, [context window], tokens↓↑, [fill%]
- Uses: displayModelId, displayReasoning, displayHasVision, displayContextWindow
- Uses: tokenUsage, rustTokens, contextFillPercentage

### 5. Required Changes

1. **Extract utilities** from AgentView (or export them):
   - `getContextFillColor()`
   - `formatContextWindow()`

2. **Extend SplitSessionViewProps**:
   ```typescript
   interface SplitSessionViewProps {
     // existing props...
     displayModelId?: string;
     displayReasoning?: boolean;
     displayHasVision?: boolean;
     displayContextWindow?: number;
     tokenUsage?: { inputTokens: number; outputTokens: number };
     rustTokens?: { inputTokens: number; outputTokens: number };
     contextFillPercentage?: number;
     isTurnSelectMode?: boolean;
   }
   ```

3. **Update SplitSessionView header** to match AgentView pattern

4. **Pass additional props** from AgentView to SplitSessionView
