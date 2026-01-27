# ThinkingLevelDialog Design Document

## Overview

This document provides a comprehensive analysis of the fspec codebase to support the implementation of a `ThinkingLevelDialog` component. The dialog allows users to set a base thinking level via the `/thinking` command, which serves as a floor for all requests (can be overridden by higher levels detected in prompt text like "ultrathink").

## Command Syntax

```
/thinking              # Opens the dialog for interactive selection
/thinking <level>      # Sets level directly without dialog
```

Valid levels (case insensitive):
- `off` - No extended thinking
- `low` - ~4K tokens
- `med` or `medium` - ~10K tokens
- `high` - ~32K tokens

**Requires active session** - Shows error message if no session is active.

## Current Thinking Level System

### JsThinkingLevel Enum (codelet-napi)

```rust
// codelet/napi/src/thinking_config.rs
pub enum JsThinkingLevel {
    Off,      // 0 - No thinking/reasoning
    Low,      // 1 - Minimal thinking (~4K tokens)
    Medium,   // 2 - Balanced thinking (~10K tokens)  
    High,     // 3 - Maximum thinking (~32K tokens)
}
```

### Detection Keywords (src/utils/thinkingLevel.ts)

```typescript
// Priority: disable > conversational exclusion > high > medium > low > off

// DISABLE keywords → Off
const DISABLE_KEYWORDS = [
  'quickly', 'brief', 'briefly', 'fast', 'nothink', 'no thinking',
  "don't think hard", "don't overthink"
];

// HIGH patterns → High (~32K tokens)
const HIGH_PATTERNS = [
  /\bultrathink\b/i,
  /\bthink\s+harder\b/i,
  /\bthink\s+intensely\b/i,
  /\bthink\s+very\s+hard\b/i,
  /\bthink\s+super\s+hard\b/i,
  /\bthink\s+really\s+hard\b/i,
  /\bthink\s+longer\b/i,
];

// MEDIUM patterns → Medium (~10K tokens)
const MEDIUM_PATTERNS = [
  /\bmegathink\b/i,
  /\bthink\s+hard\b/i,
  /\bthink\s+deeply\b/i,
  /\bthink\s+more\b/i,
  /\bthink\s+a\s+lot\b/i,
];

// LOW patterns → Low (~4K tokens)
const LOW_PATTERNS = [
  /\bthink\s+about\b/i,
  /\bthink\s+through\b/i,
  /\bthink\s+carefully\b/i,
  /^think\b/i,
  /[:.]\s*think\b/i,
];
```

### Current Usage in AgentView

```typescript
// AgentView.tsx lines 2585-2597
const thinkingLevel = detectThinkingLevel(userMessage);
setDetectedThinkingLevel(thinkingLevel);

// Get thinking config JSON if level is not Off
let thinkingConfig: string | null = null;
if (thinkingLevel !== JsThinkingLevel.Off) {
  thinkingConfig = getThinkingConfig(currentProvider, thinkingLevel);
  const label = getThinkingLevelLabel(thinkingLevel);
  if (label) {
    logger.debug(`Thinking level detected: ${label}`);
  }
}
```

### SessionHeader Display

The `SessionHeader` component displays thinking level during streaming:

```typescript
// SessionHeader.tsx lines 80-93
const getThinkingLevelLabel = (level: JsThinkingLevel): string => {
  switch (level) {
    case JsThinkingLevel.Off:
      return '';
    case JsThinkingLevel.Low:
      return '[T:Low]';
    case JsThinkingLevel.Medium:
      return '[T:Med]';
    case JsThinkingLevel.High:
      return '[T:High]';
    default:
      return '';
  }
};
```

## Dialog Implementation Patterns

### Pattern 1: SlashCommandPalette (Inline Popup)

**File:** `src/tui/components/SlashCommandPalette.tsx`

Features:
- Full-screen overlay with centered dialog
- Round border with cyan color
- Fixed-width calculation from all commands
- Up/down navigation with wrap-around
- Scroll indicators for long lists
- Footer with keyboard hints

```tsx
<Box
  position="absolute"
  width="100%"
  height="100%"
  justifyContent="center"
  alignItems="center"
  flexDirection="column"
>
  <Box
    flexDirection="column"
    borderStyle="round"
    borderColor="cyan"
    paddingX={1}
    backgroundColor="black"
    width={dialogWidth + 4}
  >
    {/* Header, Separator, Items, Footer */}
  </Box>
</Box>
```

### Pattern 2: AttachmentDialog (Modal with useInputCompat)

**File:** `src/tui/components/AttachmentDialog.tsx`

Features:
- Uses base `Dialog` component
- CRITICAL priority for input handling
- useState for selectedIndex and scrollOffset
- Auto-scroll on navigation
- Consumes all input when open

```tsx
useInputCompat({
  id: 'attachment-dialog',
  priority: InputPriority.CRITICAL,
  description: 'Attachment selection dialog',
  handler: (input, key) => {
    if (key.escape) { onClose(); return true; }
    if (key.return) { onSelect(items[selectedIndex]); onClose(); return true; }
    if (key.upArrow) { /* navigate up */ return true; }
    if (key.downArrow) { /* navigate down */ return true; }
    return true; // Consume all input
  },
});
```

### Pattern 3: Base Dialog Component

**File:** `src/components/Dialog.tsx`

Provides:
- Centered modal overlay
- Border styling
- ESC key handling via useInputCompat
- isActive prop for input capture control

## Slash Command System

### Command Registry (src/tui/utils/slashCommands.ts)

```typescript
export interface SlashCommand {
  name: string;
  description: string;
  syntax?: string;
  aliases?: string[];
  requiresSession?: boolean;
}

export const SLASH_COMMANDS: SlashCommand[] = [
  { name: 'model', description: 'Select AI model', requiresSession: false },
  { name: 'provider', description: 'Configure API providers', requiresSession: false },
  { name: 'debug', description: 'Toggle debug capture mode' },
  { name: 'clear', description: 'Clear conversation history' },
  { name: 'compact', description: 'Compact context window' },
  // ... more commands
];
```

### Command Execution (AgentView.tsx handleSubmitWithCommand)

Commands are handled inline with pattern matching:

```typescript
// Handle /model command
if (userMessage === '/model') {
  setInputValue('');
  setShowModelSelector(true);
  // ... initialization
  return;
}

// Handle /debug command
if (userMessage === '/debug') {
  setInputValue('');
  // ... toggle debug
  return;
}
```

### useSlashCommandInput Hook

**File:** `src/tui/hooks/useSlashCommandInput.ts`

Features:
- Visibility and filter state management
- Three-tier command filtering (prefix → substring → description)
- Keyboard navigation (up/down/tab/enter/escape)
- Disabled state when other modes are active

## Implementation Plan

### 1. Add `/thinking` to SLASH_COMMANDS

```typescript
// src/tui/utils/slashCommands.ts
{
  name: 'thinking',
  description: 'Set base thinking level (Off/Low/Med/High)',
  requiresSession: false,
},
```

### 2. Create ThinkingLevelDialog Component

**File:** `src/tui/components/ThinkingLevelDialog.tsx`

```typescript
interface ThinkingLevelDialogProps {
  currentLevel: JsThinkingLevel;
  onSelect: (level: JsThinkingLevel) => void;
  onClose: () => void;
}

const THINKING_LEVELS = [
  { level: JsThinkingLevel.Off, name: 'Off', description: 'No extended thinking' },
  { level: JsThinkingLevel.Low, name: 'Low', description: '~4K tokens, quick analysis' },
  { level: JsThinkingLevel.Medium, name: 'Medium', description: '~10K tokens, balanced' },
  { level: JsThinkingLevel.High, name: 'High', description: '~32K tokens, deep reasoning' },
];
```

### 3. Add State to AgentView

```typescript
// New state for base thinking level
const [baseThinkingLevel, setBaseThinkingLevel] = useState<JsThinkingLevel>(JsThinkingLevel.Off);
const [showThinkingLevelDialog, setShowThinkingLevelDialog] = useState(false);
```

### 4. Handle /thinking Command

```typescript
// In handleSubmitWithCommand
if (userMessage === '/thinking' || userMessage.startsWith('/thinking ')) {
  setInputValue('');
  
  // Require an active session
  if (!currentSessionId) {
    setConversation(prev => [
      ...prev,
      { type: 'status', content: 'Start a session first to set the thinking level.' },
    ]);
    return;
  }
  
  // Parse optional argument
  const arg = userMessage.slice('/thinking'.length).trim().toLowerCase();
  
  if (!arg) {
    // No argument - open the dialog
    setShowThinkingLevelDialog(true);
    return;
  }
  
  // Parse level argument
  let level: JsThinkingLevel | null = null;
  if (arg === 'off') level = JsThinkingLevel.Off;
  else if (arg === 'low') level = JsThinkingLevel.Low;
  else if (arg === 'med' || arg === 'medium') level = JsThinkingLevel.Medium;
  else if (arg === 'high') level = JsThinkingLevel.High;
  
  if (level !== null) {
    getRustStateSource().setBaseThinkingLevel(currentSessionId, level);
    const levelNames = ['Off', 'Low', 'Medium', 'High'];
    setConversation(prev => [
      ...prev,
      { type: 'status', content: `Thinking level set to ${levelNames[level]}.` },
    ]);
  } else {
    setConversation(prev => [
      ...prev,
      { type: 'status', content: `Invalid thinking level "${arg}". Use: off, low, med, medium, or high.` },
    ]);
  }
  return;
}
```

### 5. Modify Thinking Level Resolution

```typescript
// When sending prompt, use max of base and detected levels
const detectedLevel = detectThinkingLevel(userMessage);
const effectiveLevel = Math.max(baseThinkingLevel, detectedLevel) as JsThinkingLevel;
setDetectedThinkingLevel(effectiveLevel);

if (effectiveLevel !== JsThinkingLevel.Off) {
  thinkingConfig = getThinkingConfig(currentProvider, effectiveLevel);
}
```

### 6. Disable Slash Command Palette When Dialog Shown

```typescript
// In useSlashCommandInput options
disabled: isResumeMode || isWatcherMode || showModelSelector || 
          showSettingsTab || showThinkingLevelDialog, // Add this
```

### 7. Render Dialog in AgentView

```typescript
{showThinkingLevelDialog && (
  <ThinkingLevelDialog
    currentLevel={baseThinkingLevel}
    onSelect={(level) => {
      setBaseThinkingLevel(level);
      setShowThinkingLevelDialog(false);
      // Show status message
      setConversation(prev => [...prev, {
        type: 'status',
        content: `Base thinking level set to ${getThinkingLevelLabel(level) || 'Off'}`
      }]);
    }}
    onClose={() => setShowThinkingLevelDialog(false)}
  />
)}
```

### 8. Update SessionHeader Display

Show base thinking level indicator when not actively streaming:

```typescript
// When not loading, show base level if set
const displayLevel = isLoading 
  ? thinkingLevel 
  : (baseThinkingLevel !== JsThinkingLevel.Off ? baseThinkingLevel : null);
```

## UI Design

```
┌──────────────────────────────────────┐
│ Thinking Level                       │
├──────────────────────────────────────┤
│   Off     No extended thinking       │
│ ▸ Low     ~4K tokens, quick analysis │ ← Selected (cyan bg)
│   Medium  ~10K tokens, balanced      │
│   High    ~32K tokens, deep reasoning│
├──────────────────────────────────────┤
│ ↑↓ Navigate │ Enter Select │ Esc Close│
└──────────────────────────────────────┘
```

Color scheme:
- Border: cyan (round style)
- Selected item: cyan background, white text
- Non-selected: white text, dimColor description
- Footer: dimColor

## Edge Cases

1. **Keyword override**: If user types "ultrathink" with base level Medium, effective level should be High
2. **Disable keywords**: "quickly" should still force Off even if base level is High
3. **Session persistence**: Base thinking level should persist within a session
4. **Provider compatibility**: Some providers may not support all thinking levels

## Testing Requirements

1. Dialog renders when /thinking command is executed
2. Up/down arrows navigate through levels with wrap-around
3. Enter selects the current level and closes dialog
4. Escape closes dialog without changing level
5. Effective level = max(baseLevel, detectedLevel)
6. Disable keywords still force level to Off
7. Status message shown after selection
8. Base level persists across prompts in same session

## Files to Create/Modify

### New Files
- `src/tui/components/ThinkingLevelDialog.tsx`
- `src/tui/components/__tests__/ThinkingLevelDialog.test.tsx`

### Modified Files
- `src/tui/utils/slashCommands.ts` - Add /thinking command
- `src/tui/components/AgentView.tsx` - Add state, handler, render dialog
- `src/tui/components/SessionHeader.tsx` - Optional: show base level indicator
- `src/utils/thinkingLevel.ts` - Optional: export level metadata for dialog

## Related Work Units

- TOOL-009: Thinking config facade for provider-specific reasoning
- TOOL-010: Dynamic thinking level detection via keywords
- TUI-050: Slash command palette (pattern reference)
- TUI-019: Attachment selection dialog (pattern reference)
