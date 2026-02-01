# AST Research: Modal Dialog Patterns in AgentView.tsx

## Current Modal Dialog State Management Pattern

All modals follow this pattern in AgentView.tsx:

```typescript
// Dialog state declaration
const [showModalName, setShowModalName] = useState(false);

// Disable other interactions when any modal is active
const slashCommand = useSlashCommandInput({
  disabled: isResumeMode || isWatcherMode || isWatcherEditMode || 
            showModelSelector || showSettingsTab || 
            showThinkingLevelDialog || showAnchorViewer, // <-- Add each modal here
});

// Modal rendering (at end of component)
{showModalName && (
  <ModalComponent
    isVisible={showModalName}
    onClose={() => setShowModalName(false)}
    // other props...
  />
)}
```

## Existing Modal Dialogs

1. `showExitConfirmation` - Exit confirmation modal
2. `showThinkingLevelDialog` - Thinking level dialog (TUI-054)
3. `showAnchorViewer` - Anchor viewer dialog (TUI-056) - Already added!

## Integration Points Needed

1. Add to `disabled` prop logic in slashCommand ✓ (already done)
2. Handle `/anchors` command to open the dialog ✓ (already done)
3. Wire up real data instead of TODOs (needs implementation)