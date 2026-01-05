# AST Research: SafeTextInput Component

## Search Pattern
`const SafeTextInput` in `src/tui/components/AgentView.tsx`

## Results
src/tui/components/AgentView.tsx:203:1:const SafeTextInput: React.FC<

## Analysis
- SafeTextInput is defined at line 203 in AgentView.tsx
- It uses useInput hook for handling keyboard events
- The component is a functional component with React.FC type
- It needs to be replaced with MultiLineInput component

## Integration Points
- AgentView.tsx line ~4551 uses SafeTextInput
- Props passed: value, onChange, onSubmit, placeholder, isActive, onHistoryPrev, onHistoryNext

## Key Interface: SafeTextInputCallbacks (line 197)
```typescript
interface SafeTextInputCallbacks {
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
}
```

