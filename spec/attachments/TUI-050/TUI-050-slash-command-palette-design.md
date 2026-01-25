# TUI-050: Slash Command Autocomplete Palette - Design Notes

## Overview

A floating popup palette that appears above the input area when `/` is typed at position 0, showing matching commands with descriptions. Based on research of OpenCode, VTCode, and Codex implementations.

## Visual Design

```
┌──────────────────────────────────────────────────────────────────┐
│ Assistant: Here's the code you requested...                      │
│                                                                   │
│ [conversation area with VirtualList scrolling...]                 │
│                                                                   │
│ ┌────────────────────────────────────────────────────┐            │
│ │ Slash Commands                                     │            │
│ ├────────────────────────────────────────────────────┤            │
│ │ ▸ /model       Select AI model                     │            │
│ │   /mode        Cycle through Edit/Plan/Agent       │            │
│ │   /merge       Merge messages from session         │            │
│ │   /mcp         Manage MCP providers                │            │
│ ├────────────────────────────────────────────────────┤            │
│ │ ↑↓ Navigate | Tab/Enter Select | Esc Close         │            │
│ └────────────────────────────────────────────────────┘            │
├──────────────────────────────────────────────────────────────────┤
│ > /m▌                                                             │
└──────────────────────────────────────────────────────────────────┘
```

## Research Summary

### OpenCode (SolidJS/TypeScript)
- **Trigger:** `/` only at position 0, `@` anywhere for mentions
- **Matching:** Fuzzy matching with `fuzzysort` library
- **Navigation:** Up/Down arrows, Tab autocomplete, Enter select, Escape dismiss
- **State:** Tracks `false | "@" | "/"` visibility mode
- **Behavior:** Inline replacement with trailing space

### VTCode (Rust/Ratatui)
- **Commands:** ~25+ static commands with name and description
- **Matching:** Three-tier: prefix → substring → fuzzy fallback
- **Features:** Live preview updates input as you navigate
- **Custom prompts:** Supports user-defined prompt templates

### Key Patterns to Adopt
1. **Command registry** - Static list with name, description, handler
2. **Prefix-first matching** - Show exact prefix matches first, then fuzzy
3. **Live filtering** - Update as user types after `/`
4. **Keyboard-driven** - Full navigation without mouse

## Architecture

### New Components

#### `SlashCommandPalette.tsx`
Floating palette component using absolute positioning.

```typescript
interface SlashCommand {
  name: string;           // "model"
  description: string;    // "Select AI model"
  aliases?: string[];     // ["m"] for shortcuts
  requiresSession?: boolean;  // false for /model, /provider
}

interface SlashCommandPaletteProps {
  isVisible: boolean;
  filter: string;         // Text after "/"
  commands: SlashCommand[];
  selectedIndex: number;
  onSelect: (command: SlashCommand) => void;
  onClose: () => void;
  terminalWidth: number;
  terminalHeight: number;
}
```

#### `useSlashCommand.ts` Hook
Manages slash command state and filtering.

```typescript
interface UseSlashCommandResult {
  isVisible: boolean;
  filter: string;
  filteredCommands: SlashCommand[];
  selectedIndex: number;
  handleKeyPress: (input: string, key: Key) => boolean;
  show: () => void;
  hide: () => void;
  moveUp: () => void;
  moveDown: () => void;
  select: () => void;
}
```

### Command Registry

Located in `src/tui/utils/slashCommands.ts`:

```typescript
export const SLASH_COMMANDS: SlashCommand[] = [
  // Session management
  { name: 'model', description: 'Select AI model', requiresSession: false },
  { name: 'provider', description: 'Configure API providers', requiresSession: false },
  { name: 'debug', description: 'Toggle debug capture mode' },
  { name: 'clear', description: 'Clear conversation history' },
  { name: 'compact', description: 'Compact context window' },
  
  // Session operations  
  { name: 'resume', description: 'Resume a previous session' },
  { name: 'switch', description: 'Switch to another session', syntax: '/switch <name>' },
  { name: 'fork', description: 'Fork session at index', syntax: '/fork <index> <name>' },
  { name: 'merge', description: 'Merge from another session', syntax: '/merge <session> <indices>' },
  { name: 'rename', description: 'Rename current session', syntax: '/rename <new-name>' },
  { name: 'detach', description: 'Detach session from work unit' },
  
  // History
  { name: 'history', description: 'Show command history' },
  { name: 'search', description: 'Search command history' },
  { name: 'cherry-pick', description: 'Cherry-pick from session', syntax: '/cherry-pick <session> <index>' },
  
  // Watchers
  { name: 'watcher', description: 'Manage watcher sessions' },
  { name: 'parent', description: 'Switch to parent session' },
];
```

### Integration Points

#### MultiLineInput.tsx
- Detect `/` at position 0
- Pass filter text to parent
- Handle palette keyboard events before normal input

#### AgentView.tsx
- Render `SlashCommandPalette` when visible
- Position above input area using absolute positioning
- Handle command selection and execute

#### SplitSessionView.tsx
- Same integration as AgentView
- Palette appears in watcher pane when input is focused

## Matching Algorithm

```typescript
function filterCommands(commands: SlashCommand[], filter: string): SlashCommand[] {
  if (!filter) return commands;
  
  const lower = filter.toLowerCase();
  
  // 1. Exact prefix matches first
  const prefixMatches = commands.filter(c => 
    c.name.toLowerCase().startsWith(lower)
  );
  
  // 2. Substring matches second
  const substringMatches = commands.filter(c => 
    !c.name.toLowerCase().startsWith(lower) &&
    c.name.toLowerCase().includes(lower)
  );
  
  // 3. Description matches third
  const descMatches = commands.filter(c =>
    !c.name.toLowerCase().includes(lower) &&
    c.description.toLowerCase().includes(lower)
  );
  
  return [...prefixMatches, ...substringMatches, ...descMatches];
}
```

## Keyboard Handling

| Key | Action |
|-----|--------|
| `/` at pos 0 | Show palette |
| Up Arrow | Move selection up (wrap around) |
| Down Arrow | Move selection down (wrap around) |
| Tab | Accept selected command |
| Enter | Accept and execute (if no args needed) |
| Escape | Close palette |
| Backspace past `/` | Close palette |
| Any char | Filter commands |
| Space | Close palette (command complete) |

## Styling

Using existing fspec TUI patterns:
- `borderStyle="round"` for palette border
- `borderColor="cyan"` for active state
- `backgroundColor="black"` for palette background
- Selected item: `backgroundColor="blue"` with `color="white"`
- Descriptions: `dimColor` text

## Edge Cases

1. **No matches** - Show "No matching commands" message
2. **Terminal too small** - Limit palette height, show scroll indicator
3. **Long descriptions** - Truncate with ellipsis
4. **Commands with arguments** - Accept command, keep cursor for arg input
5. **Rapid typing** - Debounce filter updates (50ms)

## Testing Strategy

1. **Unit tests** for filtering algorithm
2. **Component tests** for SlashCommandPalette rendering
3. **Integration tests** for keyboard navigation
4. **E2E tests** for full flow in AgentView

## Implementation Order

1. Create command registry (`slashCommands.ts`)
2. Create `useSlashCommand` hook
3. Create `SlashCommandPalette` component
4. Integrate into `MultiLineInput` for trigger detection
5. Integrate into `AgentView` for rendering and handling
6. Integrate into `SplitSessionView`
7. Add tests
