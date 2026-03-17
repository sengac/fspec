# TUI-081: Display Active Role in AgentView Below SessionHeader

## Summary

When a session has an active role (set via `/role` or `set_role`), the role text should be displayed as a pinned bar immediately below the SessionHeader border, above the conversation area.

## Current SessionHeader Layout

```
#1 (AUTH-001: implementing): claude-sonnet-4 [R] [V] [200k]  1234↓ 567↑ [45%]
───────────────────────────────────────────────────────────────────────────────
```

The SessionHeader component (`src/tui/components/SessionHeader.tsx`) renders:
1. A `height={1}` row with left content (session/model info) and right content (tokens/context)
2. A single-line bottom border (`borderStyle="single"`, `borderBottom` only)

## Proposed Layout (with active role)

```
#1 (AUTH-001: implementing): claude-sonnet-4 [R] [V] [200k]  1234↓ 567↑ [45%]
───────────────────────────────────────────────────────────────────────────────
📎 Role: You are a security reviewer. Analyze code for vulnerabilities...
```

When no role is set, the role bar is hidden (zero height).

## Design Decisions

1. **Location:** Directly below the SessionHeader border, above conversation content
2. **Styling:** Dim/muted text with a 📎 prefix (or just `Role:` in cyan), single line with truncation
3. **Height:** 1 line when role is active, 0 when no role
4. **Truncation:** Long roles truncated with `textWrap="truncate-end"`
5. **Reactivity:** Updates when `/role` dialog submits, when `set_role` is called from AgentManager, or when switching sessions

## Implementation Approach

### Option A: Extend SessionHeader Component

Add an optional `roleText` prop to `SessionHeaderProps` and render an additional line below the border:

```tsx
export interface SessionHeaderProps {
  // ... existing props
  roleText?: string | null;
}
```

Render conditionally:

```tsx
{/* Bottom border separator */}
<Box width="100%" borderStyle="single" borderBottom borderTop={false} borderLeft={false} borderRight={false} />
{/* Role display (only when role is active) */}
{roleText && (
  <Box height={1} width="100%">
    <Text wrap="truncate-end" dimColor>
      {chalk.cyan('📎 Role:')} {roleText}
    </Text>
  </Box>
)}
```

### Option B: Separate RoleBanner Component (Recommended)

Create a new lightweight `RoleBanner` component rendered in AgentView between SessionHeader and conversation:

```tsx
// src/tui/components/RoleBanner.tsx
const RoleBanner: React.FC<{ sessionId: string | null }> = ({ sessionId }) => {
  // Read role from NAPI
  const role = sessionId ? sessionGetRole(sessionId)?.name : null;
  if (!role) return null;
  
  return (
    <Box height={1} width="100%">
      <Text wrap="truncate-end" dimColor>
        {chalk.cyan('Role:')} {role}
      </Text>
    </Box>
  );
};
```

Then in AgentView:

```tsx
<SessionHeader {...headerProps} />
<RoleBanner sessionId={currentSessionId} />
{/* conversation area */}
```

**Recommended:** Option B — keeps SessionHeader focused, new component is independently testable, and role reading logic is isolated.

## Reactivity Considerations

The role is stored in Rust (`RwLock<Option<String>>`). To update the TUI when the role changes:

1. **After `/role` dialog submit:** The AgentView already re-renders after `setShowRoleDialog(false)`. The RoleBanner would re-read the role from NAPI.
2. **After `set_role` from AgentManager (another session sets our role):** This won't automatically trigger a re-render. Options:
   - Poll `sessionGetRole` on a timer (simple but wasteful)
   - Add a Rust→JS notification channel for role changes
   - Piggyback on existing `refreshRustState` polling (already happens for token updates)
3. **Session switch:** When user switches sessions (Shift+Arrow), the `currentSessionId` changes, triggering re-render of RoleBanner.

Simplest approach: read role in the same `refreshRustState` polling cycle that already runs in AgentView.

## Files to Create/Modify

1. **CREATE:** `src/tui/components/RoleBanner.tsx` — New component
2. **MODIFY:** `src/tui/components/AgentView.tsx` — Render RoleBanner between SessionHeader and conversation area
3. **MODIFY:** May need to add role to the state that's refreshed from Rust on polling cycles

## Current SessionHeader Props Reference

```typescript
export interface SessionHeaderProps {
  modelId: string;
  hasReasoning?: boolean;
  hasVision?: boolean;
  contextWindow?: number;
  isDebugEnabled?: boolean;
  isSelectMode?: boolean;
  thinkingLevel?: JsThinkingLevel | null;
  baseThinkingLevel?: JsThinkingLevel;
  isLoading?: boolean;
  tokensPerSecond?: number | null;
  tokenUsage?: TokenTracker;
  rustTokens?: TokenTracker;
  contextFillPercentage?: number;
  compactionReduction?: number | null;
  supervisorInfo?: SupervisorHeaderInfo;
  sessionNumber?: number;
  isIsolated?: boolean;
}
```

Note: No `role` prop exists currently. The component uses Zustand for work unit info (`useCurrentWorkUnitId`, `useCurrentWorkUnitStatus`).
