# AST Research: TUI-058 Integration Points

## Overview
This document analyzes the code integration points needed for default thinking level persistence.

## Pattern Reference: TUI-035 Model Persistence

The lastUsedModel persistence in AgentView.tsx serves as the template pattern:

### Loading Config (Line ~1834)
```typescript
// TUI-035: Load persisted model selection from config
let persistedModelString: string | null = null;
try {
  const config = await loadConfig();
  persistedModelString = config?.tui?.lastUsedModel || null;
  if (persistedModelString) {
    logger.debug(`Found persisted model selection: ${persistedModelString}`);
  }
} catch (err) {
  logger.warn('Failed to load config for persisted model, using default', { error: err });
}
```

### Saving Config (Line ~3986)
```typescript
const existingConfig = await loadConfig();
const updatedConfig = {
  ...existingConfig,
  tui: {
    ...existingConfig?.tui,
    lastUsedModel: modelString,
  },
};
await writeConfig('user', updatedConfig);
```

## Key Files to Modify

### 1. src/utils/defaultThinkingLevelConfig.ts (NEW FILE)
Create a dedicated config module for DRY/SOLID principles:
- `loadDefaultThinkingLevel(): Promise<JsThinkingLevel | null>`
- `saveDefaultThinkingLevel(level: JsThinkingLevel): Promise<void>`

### 2. src/tui/components/ThinkingLevelDialog.tsx
Current props interface (Line 51-58):
```typescript
export interface ThinkingLevelDialogProps {
  currentLevel: JsThinkingLevel;
  onSelect: (level: JsThinkingLevel) => void;
  onClose: () => void;
}
```

New props needed:
```typescript
export interface ThinkingLevelDialogProps {
  currentLevel: JsThinkingLevel;
  defaultLevel: JsThinkingLevel | null;  // NEW: to show (default) indicator
  onSelect: (level: JsThinkingLevel) => void;
  onSetDefault: (level: JsThinkingLevel) => void;  // NEW: for D key
  onClose: () => void;
}
```

### 3. src/tui/components/AgentView.tsx

#### ThinkingLevelDialog usage (Line 7555-7566):
```typescript
{showThinkingLevelDialog && currentSessionId && (
  <ThinkingLevelDialog
    currentLevel={rustSnapshot.baseThinkingLevel as JsThinkingLevel}
    onSelect={(level) => {
      getRustStateSource().setBaseThinkingLevel(currentSessionId, level);
      refreshRustState();
      setShowThinkingLevelDialog(false);
    }}
    onClose={() => setShowThinkingLevelDialog(false)}
  />
)}
```

Need to add:
- State: `defaultThinkingLevel: JsThinkingLevel | null`
- Load default on mount (similar to lastUsedModel at line ~1834)
- Pass `defaultLevel` and `onSetDefault` props
- Apply default after session creation (after line ~4580)

#### Session Creation Points:
1. `createNewSession` callback (Line ~4577)
2. `sessionManagerCreateWithId` direct call (Line ~2576)

After session is created, apply default:
```typescript
// After activateSession(result.sessionId):
if (defaultThinkingLevel !== null) {
  getRustStateSource().setBaseThinkingLevel(result.sessionId, defaultThinkingLevel);
}
```

## ThinkingLevelDialog Footer Update

Current (Line 135):
```typescript
<Text dimColor>↑↓ Navigate │ Enter Select │ Esc Close</Text>
```

New:
```typescript
<Text dimColor>↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close</Text>
```

## Input Handler Update

Current handler (Line 70-101) handles: escape, return, upArrow, downArrow

Need to add 'd' key handler:
```typescript
if (input.toLowerCase() === 'd') {
  onSetDefault(selectedIndex as JsThinkingLevel);
  // Show status message but don't close dialog
  return true;
}
```

## Option Display Update

Current option rendering (Line 113-129):
```typescript
{THINKING_LEVELS.map((option, index) => {
  const isSelected = index === selectedIndex;
  return (
    <Box key={option.level}>
      <Text ...>{isSelected ? '▸ ' : '  '}{option.label}</Text>
      <Text dimColor={!isSelected}>{' - '}{option.description}</Text>
    </Box>
  );
})}
```

Need to add default indicator:
```typescript
const isDefault = defaultLevel === index;
// In render:
<Text dimColor={!isSelected}>
  {' - '}{option.description}{isDefault ? ' (default)' : ''}
</Text>
```

## Config Key

Store at: `tui.defaultThinkingLevel` (number: 0-3)

Example config:
```json
{
  "tui": {
    "lastUsedModel": "anthropic/claude-sonnet-4-20250514",
    "defaultThinkingLevel": 2
  }
}
```

## Test File Patterns

Reference: `src/tui/__tests__/AgentView-model-persistence.test.tsx`

Key mock patterns needed:
- Mock `loadConfig` and `writeConfig` 
- Mock `getRustStateSource().setBaseThinkingLevel`
- Mock initial config state per test scenario
