# AST Research: CTX-009 Badge Threshold Mismatch

## Research Summary

Investigated the SessionHeader and AgentView components to confirm data flow for badge display.

## Findings

### SessionHeader.tsx — Props Interface (line 40)

```typescript
export interface SessionHeaderProps {
  modelId: string;
  hasReasoning?: boolean;
  hasVision?: boolean;
  contextWindow?: number;        // ← Currently used for badge display
  // ... other props
  contextFillPercentage?: number; // ← This uses compaction threshold as denominator
}
```

**Key observation:** No `compactionThreshold` prop exists yet. The badge on line 160-162 uses `contextWindow` directly:
```typescript
if (contextWindow > 0) {
  leftContent += chalk.dim(` [${formatContextWindow(contextWindow)}]`);
}
```

### AgentView.tsx — rustModelInfo (line 1164)

The `rustModelInfo` useMemo extracts `modelId`, `reasoning`, `hasVision`, `contextWindow` from `rustSnapshot.model` but does NOT extract `compactionThreshold`.

The `createModelInfo` helper (line 1166) returns `{ modelId, reasoning, hasVision, contextWindow }` — no threshold field.

### SessionHeader usage (line 5251-5268)

```tsx
<SessionHeader
  contextWindow={displayContextWindow}  // ← from rustModelInfo.contextWindow
  contextFillPercentage={contextFillPercentage}  // ← computed against threshold
  // ... no compactionThreshold prop
/>
```

## Fix Plan

1. Add `compactionThreshold?: number` to `SessionHeaderProps`
2. Change badge line to: `const badgeValue = compactionThreshold ?? contextWindow;`
3. In `rustModelInfo`, extract `compactionThreshold` from `rustSnapshot.model.compactionThreshold`
4. Pass `compactionThreshold` prop to `<SessionHeader>`
