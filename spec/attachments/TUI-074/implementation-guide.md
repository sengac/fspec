# TUI-074: Create ProviderSettingsScreen Component

## Overview

Create an orchestrator component that wraps `ProviderSettingsPanel` (the presentation component), uses the existing `useProviderSettingsState` hook, and **moves ~300 lines of input handling** out of AgentView.tsx.

## Current State in AgentView.tsx

### Input Handling Code to Move (Lines ~6857-7155)

This is the main code to extract - approximately 300 lines of keyboard handling:

```typescript
if (showSettingsTab) {
  const currentItem = providerSettings.getCurrentItem();
  const currentProvider = providerSettings.getCurrentProvider();
  const currentProfile = providerSettings.getCurrentProfile();
  const { mode } = providerSettings;

  // Delete confirmation mode
  if (mode.type === 'delete-profile') {
    if (input === 'y' || input === 'Y') {
      void providerSettings.removeProfile(mode.providerId, mode.profileName).then(() => {
        providerSettings.setMode({ type: 'list' });
      });
      return;
    }
    if (key.escape || input === 'n' || input === 'N') {
      providerSettings.setMode({ type: 'list' });
      return;
    }
    return;
  }

  // API key editing mode
  if (mode.type === 'edit-api-key') {
    if (key.escape) {
      providerSettings.setMode({ type: 'list' });
      providerSettings.setEditingApiKey('');
      return;
    }
    if (key.return) {
      const apiKey = providerSettings.editingApiKey.trim();
      if (apiKey) {
        void providerSettings.saveApiKey(mode.providerId, apiKey).then(() => {
          providerSettings.setMode({ type: 'list' });
          providerSettings.setEditingApiKey('');
        });
      } else {
        providerSettings.setMode({ type: 'list' });
        providerSettings.setEditingApiKey('');
      }
      return;
    }
    if (key.backspace || key.delete) {
      providerSettings.setEditingApiKey(prev => prev.slice(0, -1));
      return;
    }
    // Character input
    const clean = input.split('').filter(ch => {
      const code = ch.charCodeAt(0);
      return code >= 32 && code <= 126;
    }).join('');
    if (clean) {
      providerSettings.setEditingApiKey(prev => prev + clean);
    }
    return;
  }

  // Profile form mode (create/edit)
  if (mode.type === 'create-profile' || mode.type === 'edit-profile') {
    if (key.escape) {
      providerSettings.setMode({ type: 'list' });
      return;
    }
    // Tab navigation between fields
    if (key.tab) {
      if (providerSettings.isEditingName) {
        providerSettings.setIsEditingName(false);
        providerSettings.setFormFieldIndex(0);
      } else if (key.shift) {
        if (providerSettings.formFieldIndex > 0) {
          providerSettings.setFormFieldIndex(prev => prev - 1);
        } else {
          providerSettings.setIsEditingName(true);
        }
      } else {
        if (providerSettings.formFieldIndex < 3) {
          providerSettings.setFormFieldIndex(prev => prev + 1);
        }
      }
      return;
    }
    // Enter to save
    if (key.return && !key.shift) {
      const values = providerSettings.formValues;
      const name = providerSettings.profileName.trim();
      if (values.baseUrl && values.apiKey && name) {
        const config = { ... };
        void providerSettings.saveProfileConfig(mode.providerId, name, config).then(() => {
          providerSettings.setMode({ type: 'list' });
        });
      }
      return;
    }
    // Field editing (backspace, character input)
    ...
  }

  // Filter mode
  if (providerSettings.isFilterMode) {
    if (key.escape) {
      providerSettings.setIsFilterMode(false);
      providerSettings.setFilter('');
      return;
    }
    if (key.return) {
      providerSettings.setIsFilterMode(false);
      return;
    }
    // Backspace and character input
    ...
  }

  // List mode - main navigation
  if (key.escape) {
    if (providerSettings.filter) {
      providerSettings.setFilter('');
      return;
    }
    setShowSettingsTab(false);
    return;
  }

  if (key.tab) {
    setShowSettingsTab(false);
    setShowModelSelector(true);
    // Switch to model selector
    return;
  }

  if (input === '/') {
    providerSettings.setIsFilterMode(true);
    return;
  }

  // Arrow navigation
  if (key.upArrow && providerSettings.selectedIndex > 0) { ... }
  if (key.downArrow && providerSettings.selectedIndex < providerSettings.navItems.length - 1) { ... }

  // Enter to expand/edit
  if (key.return && currentItem) { ... }

  // 'e' to edit API key or profile
  if ((input === 'e' || input === 'E') && currentItem) { ... }

  // 'n' to create new profile
  if ((input === 'n' || input === 'N') && currentItem) { ... }

  // 'd' to delete
  if ((input === 'd' || input === 'D') && currentItem) { ... }

  // 't' to test connection
  if ((input === 't' || input === 'T') && currentItem) { ... }

  // 'r' to refresh
  if (input === 'r' || input === 'R') { ... }

  return; // Consume all input when settings tab is open
}
```

### Rendering Code (Lines ~7598-7651)

```tsx
if (showSettingsTab) {
  const settingsVisibleHeightCalc = terminalHeight - 6;

  // Build effective panel mode by mapping hook mode types to panel mode types
  let effectiveMode: SettingsPanelMode;
  const hookMode = providerSettings.mode;
  
  if (hookMode.type === 'create-profile' || hookMode.type === 'edit-profile') {
    effectiveMode = { type: 'profile-form', ... };
  } else if (hookMode.type === 'delete-profile') {
    effectiveMode = { type: 'delete-confirm', ... };
  } else if (hookMode.type === 'edit-api-key') {
    effectiveMode = { type: 'edit-api-key', ... };
  } else {
    effectiveMode = { type: 'list' };
  }

  return (
    <ProviderSettingsPanel
      width={terminalWidth}
      height={terminalHeight}
      providers={providerSettings.providers}
      navItems={providerSettings.navItems}
      selectedIndex={providerSettings.selectedIndex}
      scrollOffset={providerSettings.scrollOffset}
      visibleHeight={settingsVisibleHeightCalc}
      mode={effectiveMode}
      filter={providerSettings.filter}
      isFilterMode={providerSettings.isFilterMode}
      testResult={providerSettings.testResult}
    />
  );
}
```

## Target File

Create: `src/tui/components/ProviderSettingsScreen.tsx`

## Component Interface

```tsx
export interface ProviderSettingsScreenProps {
  /** Terminal width for layout */
  width: number;
  /** Terminal height for layout */
  height: number;
  /** Called when screen should close */
  onClose: () => void;
  /** Called to switch to model selector */
  onSwitchToModels: () => void;
}

export function ProviderSettingsScreen({
  width,
  height,
  onClose,
  onSwitchToModels,
}: ProviderSettingsScreenProps): React.ReactElement {
  // Use the existing hook
  const providerSettings = useProviderSettingsState();
  
  // Calculate visible height
  const visibleHeight = height - 6;
  
  // Handle keyboard input - MOVE ALL 300 LINES HERE
  useInput((input, key) => {
    const currentItem = providerSettings.getCurrentItem();
    const currentProvider = providerSettings.getCurrentProvider();
    const currentProfile = providerSettings.getCurrentProfile();
    const { mode } = providerSettings;

    // Delete confirmation mode
    if (mode.type === 'delete-profile') {
      // ... handle y/n/escape
    }

    // API key editing mode
    if (mode.type === 'edit-api-key') {
      // ... handle escape, return, backspace, chars
    }

    // Profile form mode
    if (mode.type === 'create-profile' || mode.type === 'edit-profile') {
      // ... handle tab, return, backspace, chars
    }

    // Filter mode
    if (providerSettings.isFilterMode) {
      // ... handle escape, return, backspace, chars
    }

    // List mode navigation
    if (key.escape) {
      if (providerSettings.filter) {
        providerSettings.setFilter('');
        return;
      }
      onClose();
      return;
    }

    if (key.tab) {
      onSwitchToModels();
      return;
    }

    // ... rest of input handling
  }, { isActive: true });
  
  // Build effective panel mode
  const effectiveMode = buildEffectiveMode(providerSettings);
  
  // Render the presentation component
  return (
    <ProviderSettingsPanel
      width={width}
      height={height}
      providers={providerSettings.providers}
      navItems={providerSettings.navItems}
      selectedIndex={providerSettings.selectedIndex}
      scrollOffset={providerSettings.scrollOffset}
      visibleHeight={visibleHeight}
      mode={effectiveMode}
      filter={providerSettings.filter}
      isFilterMode={providerSettings.isFilterMode}
      testResult={providerSettings.testResult}
    />
  );
}
```

## Existing Components

### useProviderSettingsState.ts (Already exists)

Located at `src/tui/hooks/useProviderSettingsState.ts` - provides all state management.

### ProviderSettingsPanel.tsx (Already exists)

Located at `src/tui/components/ProviderSettingsPanel.tsx` - purely presentational, no input handling.

### ProviderSettingsView.tsx (EXISTS BUT NOT USED)

Located at `src/tui/components/ProviderSettingsView.tsx` - a standalone component with its own state and input handling. **This is NOT currently used by AgentView.**

## Key Decisions

1. **Use ProviderSettingsPanel** (not ProviderSettingsView) as the presentation layer
2. **Move input handling** from AgentView to ProviderSettingsScreen
3. **Keep useProviderSettingsState** as-is (no changes needed)

## Dependencies

- `useProviderSettingsState` hook (existing)
- `ProviderSettingsPanel` component (existing)
- Types from `ProviderSettingsPanel` exports

## Testing Considerations

- Test all keyboard modes: list, filter, api-key edit, profile form, delete confirm
- Test Tab switching to model selector
- Test Escape closing/cancelling
- Test profile CRUD operations
