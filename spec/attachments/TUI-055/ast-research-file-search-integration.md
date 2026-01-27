# AST Research: File Search Popup Modal Integration Analysis

## Research Summary

This document contains AST analysis results for implementing a file search popup modal triggered by @ symbol in MultiLineInput.tsx, following the same UI pattern as the existing slash command palette.

## Key Findings

### 1. MultiLineInput Component Structure

**Location**: `src/tui/components/MultiLineInput.tsx`

**Interface Analysis**:
```typescript
export interface MultiLineInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  isActive?: boolean;
  maxVisibleLines?: number;
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
  /**
   * TUI-050: When true, Enter key is NOT handled by this component.
   * This allows parent components (like slash command palette) to handle Enter.
   * Use this when an overlay is active that needs to capture Enter.
   */
  suppressEnter?: boolean;
}
```

**Key Integration Points**:
- ✅ **Already has `suppressEnter` prop** for overlay integration
- ✅ Uses `useInputCompat` with `InputPriority.MEDIUM` for input handling
- ✅ Has keyboard event handling infrastructure in place
- ✅ Component exports at line 38: `export const MultiLineInput: React.FC<MultiLineInputProps>`

**Required Changes**:
- Need to add @ symbol detection logic similar to existing input handling
- Need to add prop for file search popup visibility/state
- Need to integrate with existing input priority system

### 2. Slash Command Palette Pattern Analysis

**Location**: `src/tui/components/SlashCommandPalette.tsx`

**UI Structure Analysis**:
```typescript
export interface SlashCommandPaletteProps {
  /** Whether the palette is visible */
  isVisible: boolean;
  /** Current filter text (after "/") */
  filter: string;
  /** Filtered list of commands to display */
  commands: SlashCommand[];
  /** Currently selected command index */
  selectedIndex: number;
  /** Fixed dialog width (calculated from all commands) */
  dialogWidth: number;
  /** Maximum height of the palette (number of visible items) */
  maxVisibleItems?: number;
}
```

**Key UI Patterns**:
- ✅ **Center screen positioning** using floating Box layout
- ✅ **Filtering by search text** with real-time updates
- ✅ **Keyboard navigation** with selectedIndex state
- ✅ **Dynamic width calculation** based on content
- ✅ **Visible/hidden state management** with isVisible prop

**Reusable Patterns**:
- Center screen Box positioning strategy
- Keyboard navigation with Up/Down arrows
- Real-time filtering implementation
- Dynamic dialog sizing
- Visual selection indicators

### 3. Tool Integration Analysis

**Location**: `src/tui/components/AgentView.tsx`

**Tool Usage Pattern**:
- Tools are referenced by string name (e.g., "Glob", "Grep", "Read")
- Tool execution happens through message-based system
- Tool results are handled through streaming interface

**File Search Integration**:
- Glob tool available in context as confirmed by user
- Pattern: `**/*query*` for fuzzy file matching
- Tool calls return file paths as results

### 4. Input Hook Architecture

**Location**: `src/tui/hooks/` (referenced from MultiLineInput)

**Key Hooks**:
- `useMultiLineInput`: Handles text state and cursor management
- `useInputCompat`: Provides priority-based input handling system

**Integration Strategy**:
- Can extend useInputCompat with HIGH priority for @ symbol detection
- Need new hook for file search state management similar to slash command pattern

## Implementation Architecture

### Required New Components

1. **FileSearchPopup Component**
   - Copy SlashCommandPalette structure
   - Props: isVisible, filter, files, selectedIndex, dialogWidth
   - Same center screen positioning strategy

2. **useFileSearchInput Hook**
   - Manage @ symbol detection with regex `@(\S*)$`
   - Handle Glob tool integration for file search
   - Manage popup visibility and filtering state
   - Handle keyboard navigation (Up/Down/Enter/Escape)

3. **MultiLineInput Enhancement**
   - Add file search popup integration
   - New props: onFileSearchTrigger, fileSearchVisible, etc.
   - Coordinate suppressEnter with file search popup state

### Integration Points

1. **@ Symbol Detection**
   - Add to MultiLineInput input handler
   - Pattern: `const atMatch = text.match(/@(\S*)$/)`
   - Trigger popup on match, update filter on continued typing

2. **Glob Tool Integration**
   - Call Glob tool with pattern `**/*{filter}*`
   - Parse results into file path array
   - Handle empty results with "No files found" message

3. **UI Consistency**
   - Copy exact styling from SlashCommandPalette
   - Same keyboard navigation behavior
   - Same center screen positioning
   - Same visual selection indicators

## File References

- **MultiLineInput**: `src/tui/components/MultiLineInput.tsx:21-36` (interface) and line 38 (export)
- **SlashCommandPalette**: `src/tui/components/SlashCommandPalette.tsx:17-30` (interface) and line 32 (export)
- **Tool Integration**: `src/tui/components/AgentView.tsx:500-501, 782-783` (toolName references)
- **Input System**: `src/tui/hooks/useMultiLineInput.js` and `src/tui/input/index.js` (InputPriority)

## Implementation Approach

1. **Create FileSearchPopup component** using SlashCommandPalette as template
2. **Create useFileSearchInput hook** for state management
3. **Enhance MultiLineInput** with @ symbol detection and popup integration
4. **Integrate Glob tool calls** for file searching
5. **Test integration** with existing input priority system

This architecture maintains consistency with existing patterns while adding the file search functionality in a clean, maintainable way.