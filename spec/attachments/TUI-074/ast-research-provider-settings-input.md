# AST Research: Provider Settings Input Handling

## Summary

Analysis of code to extract from AgentView.tsx into ProviderSettingsScreen.tsx

## Key Files

### useProviderSettingsState.ts (EXISTING - NO CHANGES NEEDED)
- Location: `src/tui/hooks/useProviderSettingsState.ts`
- Line 92: `export function useProviderSettingsState(): UseProviderSettingsStateReturn`
- Provides all state management for provider settings
- Already exports all needed state and actions

### AgentView.tsx - Input Handling to Extract
- Location: `src/tui/components/AgentView.tsx`

#### Input Handling Block (Lines 6421-6724)
```
if (showSettingsTab) { ... }
```
- Line 6421: Main input handling block (~304 lines)
- Contains: delete confirmation, API key edit, profile form, filter mode, list navigation

#### Rendering Block (Line 7014)
```
if (showSettingsTab) { ... }
```
- Line 7014: Rendering block for ProviderSettingsPanel

## Mode Handling in AgentView (to extract)

1. **delete-profile mode** (lines 6428-6440)
   - 'y' confirms delete
   - 'n' or Escape cancels

2. **edit-api-key mode** (lines 6443-6474)
   - Escape cancels
   - Enter saves
   - Backspace/delete removes char
   - Printable chars append

3. **create-profile/edit-profile mode** (lines 6477-6561)
   - Tab navigates fields forward
   - Shift+Tab navigates backward
   - Escape cancels
   - Enter saves
   - Backspace/chars edit current field

4. **filter mode** (lines 6564-6586)
   - Escape clears and exits
   - Enter exits keeping filter
   - Backspace/chars edit filter

5. **list mode** (lines 6588-6724)
   - Escape closes (or clears filter first)
   - Tab switches to model selector
   - '/' enters filter mode
   - Up/Down arrows navigate
   - Enter expands provider or opens profile form
   - 'e' edits API key or profile
   - 'n' creates new profile
   - 'd' deletes profile or API key
   - 't' tests connection
   - 'r' refreshes

## Reference: ModelSelectorScreen Pattern

ModelSelectorScreen.tsx (TUI-073) provides the pattern to follow:
- Location: `src/tui/components/ModelSelectorScreen.tsx`
- Uses useModelSelectorState hook
- Handles all input via useInput
- Renders ModelSelectorView presentation component
- Props: width, height, currentModelId, onSelectModel, onClose, onSwitchToSettings

## Implementation Target

Create `src/tui/components/ProviderSettingsScreen.tsx`:
- Use useProviderSettingsState hook
- Handle all input via useInput (extract from AgentView)
- Render ProviderSettingsPanel presentation component
- Props: width, height, onClose, onSwitchToModels
