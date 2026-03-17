# AST Research: AgentView Structure for RoleBanner Insertion

## Research Summary

Analyzed `src/tui/components/AgentView.tsx` to find exact insertion point for RoleBanner.

## Findings

### JSX Layout (line 4976+)

```
<Box flexDirection="column" flexGrow={1}>
  <SessionHeader ... />                    ← line 4996 end
                                           ← ⭐ INSERT RoleBanner HERE (line 4997)
  <Box flexGrow={1} flexBasis={0}>         ← line 4999
    <VirtualList ... />
  </Box>
  ...
</Box>
```

### How Role is Read (NAPI Binding)

- `sessionGetRole(currentSessionId)` returns `{ name: string } | null`
- Imported from `@sengac/codelet-napi` (line 63-93 import block)
- Already used at line 5320 for `/role` dialog initialization

### Current Role State Pattern

- `currentSessionId` — the active session ID (useState)
- `sessionGetRole(id)?.name ?? ''` — read pattern already established
- `showRoleDialog` — boolean for dialog visibility (useState at line 1082)
- After `/role` submit: `setShowRoleDialog(false)` triggers re-render which will pick up new role

### RoleBanner Integration Approach

1. Create `src/tui/components/RoleBanner.tsx` as pure functional component
2. Props: `{ roleText: string | null }` 
3. Read role in AgentView and pass as prop (avoids extra NAPI call in component)
4. Insert between `</SessionHeader>` and conversation `<Box>`
5. Role text can be read alongside existing `sessionGetRole` call
