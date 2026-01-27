# File Search Popup Modal Research & Discovery

## Research Summary

This document contains research and discovery questions for implementing a file search popup modal for the @ symbol input in MultiLineInput.tsx, similar to OpenCode's implementation.

## OpenCode Implementation Analysis

### Architecture Overview

OpenCode implements a sophisticated @ file reference system with the following components:

**Detection & Parsing:**
- Uses regex `@(\S*)$` to detect @ symbol at cursor position in real-time
- Triggers on any character following @ symbol
- Maintains cursor position tracking for accurate insertion

**Search Backend:**
- Calls `file.searchFiles(query)` for fuzzy file matching
- Returns array of file paths matching the query
- Integrates with existing file indexing system

**UI Components:**
- Popup appears as overlay positioned near cursor
- Filterable list component with keyboard navigation
- File icons and directory/filename split display
- Similar styling to command palette for consistency

**Integration Pattern:**
```typescript
// Detection in input handler
const atMatch = rawText.substring(0, cursorPosition).match(/@(\S*)$/)
if (atMatch) {
  atOnInput(atMatch[1])  // Trigger search with query
  setStore("popover", "at")  // Show popup
}

// Search integration
const fileOptions: AtOption[] = paths.map((path) => ({ 
  type: "file", 
  path, 
  display: path 
}))
```

**Result Handling:**
- Creates "pills" when files are selected (visual tokens in input)
- Inserts `@filepath` format into text
- Maintains special formatting for file references

### Key Features from OpenCode

1. **Real-time Search**: Searches as user types after @
2. **Fuzzy Matching**: Finds files even with partial/non-contiguous matches  
3. **Visual Pills**: File references become styled tokens in the input
4. **Recent Files**: Shows recently accessed files when no query
5. **Directory Context**: Displays both directory and filename for clarity
6. **Keyboard Navigation**: Full arrow key navigation in popup
7. **Escape Handling**: Closes popup without selection

## Target Implementation Context

### Current System (fspec TUI)

**Technology Stack:**
- Terminal UI using React + Ink framework
- Existing MultiLineInput.tsx component with sophisticated input handling
- Priority-based input system with `useInputCompat`
- Already supports overlays with `suppressEnter` pattern

**MultiLineInput.tsx Capabilities:**
- Multi-line text input with cursor navigation
- Word-level operations (Alt+Left/Right, Alt+Backspace)
- Line operations (Shift+Enter for newline)
- History navigation (Shift+Up/Down)
- Visual cursor rendering with inverse style
- Viewport scrolling for long content

**Existing Input Architecture:**
```typescript
useInputCompat({
  id: 'multi-line-input',
  priority: InputPriority.MEDIUM,
  description: 'Multi-line text input keyboard handler',
  isActive,
  handler: (input, key) => {
    // Current input handling logic
  }
});
```

### Available Tools & Infrastructure

**File Operations:**
- User mentioned ripgrep is already built into the app
- Likely existing file system utilities
- Potential for file indexing/caching

**UI Components:**
- Terminal-based UI system (Ink/React)
- Need to design popup overlay for terminal context
- Limited screen real estate compared to web UI

## Discovery Questions

### 1. Trigger & Detection Behavior

**Question:** Should we use the same `@` symbol trigger as OpenCode, or do you prefer something else?

**Options:**
- `@` - immediate trigger (like OpenCode)
- `@` + minimum characters (e.g., `@` + 2 chars)
- Alternative trigger (e.g., `#files`, `ctrl+f`)

**Question:** When exactly should the popup appear?
- Immediately when `@` is typed?
- After `@` + space?
- After `@` + minimum number of characters?

### 2. File Search Implementation

**Question:** You mentioned ripgrep is built into your app - do you already have file search functionality we can reuse?

**Follow-ups:**
- Is there an existing file search API/function we can integrate with?
- Are files already indexed, or should we implement on-demand search?

**Question:** What files should be included in search?
- All files in the project?
- Filtered by type (e.g., only source code: .ts, .tsx, .js, .py, etc.)?
- Exclude certain directories (node_modules, .git, dist, build)?

**Question:** Search scope - filenames only or content too?
- Filename/path matching only (faster, simpler)
- File content search as well (more comprehensive)
- Configurable option?

### 3. Terminal UI/UX Design

**Question:** How should the popup appear in the terminal interface?

**Options:**
- Overlay above the current line
- Overlay below the current line  
- Replace input area temporarily
- Side panel approach

**Question:** How many search results should be visible?
- Fixed number (e.g., 5-10 items)?
- Dynamic based on terminal height?
- Scrollable list with indicators?

**Question:** What should the search results display show?
- Full path: `/src/components/Button.tsx`
- Directory + filename: `src/components/` + `Button.tsx`
- Just filename with path on hover/selection?
- File type icons (if supported in terminal)?

### 4. Integration with MultiLineInput

**Question:** How should this integrate with your existing `suppressEnter` pattern?

**Options:**
- New `suppressArrows` prop for popup navigation?
- Higher priority input handler during popup?
- Integration with existing overlay system?

**Question:** After selecting a file, what should be inserted?

**Options:**
- Just the file path: `src/components/Button.tsx`
- @ format like OpenCode: `@src/components/Button.tsx`  
- Custom format: `[file:src/components/Button.tsx]`
- Configurable format?

**Question:** Should file references get special visual treatment?
- Plain text insertion?
- Special highlighting/styling in terminal?
- Visual indicators that it's a file reference?

### 5. Performance & Caching

**Question:** How should we handle file indexing for performance?

**Options:**
- Pre-index all files on startup?
- Index on first use, cache results?
- On-demand search each time (simpler but potentially slower)?
- Background indexing with file system watchers?

**Question:** Should we implement caching for search results?
- Cache recent searches?
- Cache file list snapshots?
- What's the invalidation strategy?

### 6. User Experience Features

**Question:** Should we implement "recent files" like OpenCode?
- Show recently opened/referenced files when no query?
- How many recent files to track?
- Where to store this data?

**Question:** What keyboard shortcuts should be supported?
- Arrow keys for navigation (Up/Down)?
- Tab for completion?
- Enter to select?
- Escape to cancel?
- Page Up/Down for long lists?

**Question:** How should search matching work?
- Exact substring matching?
- Fuzzy matching (characters can be non-contiguous)?
- Case sensitive or insensitive?
- Support for wildcards or regex?

### 7. Error Handling & Edge Cases

**Question:** How should we handle edge cases?
- No files found for query?
- Search service unavailable?
- Very large file lists (thousands of files)?
- Binary files vs. text files?

**Question:** Should there be any file access restrictions?
- Permission checking before showing files?
- Hide certain file types?
- Project-relative paths only?

### 8. Configuration & Customization

**Question:** Should this feature be configurable?
- Enable/disable the @ file search?
- Customize the trigger character/sequence?
- Configure included/excluded file patterns?
- Customize the display format?

**Question:** Where should configuration be stored?
- Project-level config file?
- User-level preferences?
- Command line options?

## Next Steps

Based on your answers to these discovery questions, we can:

1. Define the specific requirements and acceptance criteria
2. Design the technical implementation approach
3. Plan the UI/UX for terminal environment
4. Identify integration points with existing code
5. Implement the feature following ACDD methodology

## Implementation Considerations

### Technical Challenges
- Terminal UI constraints vs. web UI flexibility
- Input event handling coordination between popup and text input
- File system integration and performance
- Visual design limitations in terminal environment

### Integration Points
- MultiLineInput.tsx component modification
- Input priority system coordination  
- File search backend integration
- Terminal rendering system

### Success Criteria
- Smooth user experience without input lag
- Intuitive keyboard navigation
- Fast file search response times
- Clean integration with existing UI patterns
- Maintainable and testable code architecture