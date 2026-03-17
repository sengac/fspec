# AST Research: RoleDialog Component for TUI-082

## Research Summary

Analyzed RoleDialog.tsx to understand the current button layout and focus cycling for adding a Remove button.

## Key Findings

### RoleDialog.tsx Structure
- **Location**: `src/components/RoleDialog.tsx`
- **FocusArea type**: `'textarea' | 'ok' | 'cancel'` — needs `'remove'` added
- **Tab cycle**: `textarea → ok → cancel → textarea` — needs `remove` inserted between `ok` and `cancel`
- **Arrow navigation**: Left/right between `ok` and `cancel` — needs to include `remove`
- **Submit handler**: `onSubmit(value)` called when OK pressed — Remove will call `onSubmit('')`

### Button Row (lines 254-273)
```tsx
<Box marginTop={1} justifyContent="center">
  <Box marginX={1}>
    <Text backgroundColor={focus === 'ok' ? 'blue' : undefined} ...> OK </Text>
  </Box>
  <Box marginX={1}>
    <Text backgroundColor={focus === 'cancel' ? 'blue' : undefined} ...> Cancel </Text>
  </Box>
</Box>
```

### AgentView Integration (line 5334-5361)
- RoleDialog receives `initialRole` from `sessionGetRole(currentSessionId)?.name`
- `onSubmit` calls `sessionSetRole(id, role.trim(), null, null)` for non-empty
- `onSubmit` calls `sessionSetRole(id, '', null, null)` for empty (BUG-121 fix enables this)
- Remove button just needs to call `onSubmit('')` — existing plumbing handles the rest

### Changes Required
1. Add `showRemove` prop to RoleDialogProps (derived from `initialRole` being non-empty)
2. Add `'remove'` to FocusArea union type
3. Update Tab cycle in useInputCompat handler
4. Update left/right arrow navigation for button row
5. Add Remove button JSX between OK and Cancel (conditionally rendered)
6. Remove button styled with red background when focused, red text when not
7. Enter on Remove calls `onSubmit('')` and dialog closes
