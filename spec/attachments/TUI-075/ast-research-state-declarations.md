# AST Research: AgentView State Declarations

## Research Query
Pattern: `const [$NAME, $SETTER] = useState($INIT)`
File: src/tui/components/AgentView.tsx
Language: TSX

## Findings

### Screen Visibility States (Lines 985-986)
These are the key states for TUI-075:
```
985: const [showModelSelector, setShowModelSelector] = useState(false);
986: const [showSettingsTab, setShowSettingsTab] = useState(false);
```

### Related State (Line 984)
```
984: const [modelsInitialized, setModelsInitialized] = useState(false);
```
This is used by the model loading logic and may need review.

### Total useState Declarations: 42

All state is properly typed via TypeScript inference from initial values.

## Key Integration Points

1. **showModelSelector** - Controls ModelSelectorScreen visibility
2. **showSettingsTab** - Controls ProviderSettingsScreen visibility
3. **modelsInitialized** - May be orphaned now that useModelSelectorState manages model loading

## Verification Notes

- No undefined state setter references found in current code
- Screen toggle states are properly declared
- Integration callbacks use these setters appropriately
