# TUI-039: Multi-line Text Input Research Findings

## Executive Summary

This document contains research findings from analyzing multi-line text input implementations in several code editors and TUI frameworks:
- **OpenCode** (Go/Bubble Tea) - Uses charmbracelet/bubbles/textarea
- **VTCode** (Rust/Ratatui) - Custom InputManager implementation
- **Ink** (React/TypeScript) - React-based terminal rendering framework
- **Bubbles** (Go) - Charm's component library with textarea and textinput

The current `SafeTextInput` component in fspec's AgentView is extremely basic and needs significant enhancement to support proper multi-line editing with cursor navigation.

---

## 1. Current State Analysis

### Current SafeTextInput Implementation (AgentView.tsx lines 203-301)

The current implementation has critical limitations:

```typescript
const SafeTextInput: React.FC<{
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  isActive?: boolean;
} & SafeTextInputCallbacks> = ({ ... }) => {
  useInput((input, key) => {
    if (key.return) { onSubmit(); return; }
    if (key.backspace || key.delete) {
      onChange(valueRef.current.slice(0, -1));  // Only removes LAST char
      return;
    }
    // Arrow keys are IGNORED
    if (key.upArrow || key.downArrow || key.leftArrow || key.rightArrow) {
      return;
    }
    // Only appends to END
    onChange(valueRef.current + clean);
  });
  
  return <Text>{value}<Text inverse> </Text></Text>;  // No cursor visualization
};
```

**Critical Limitations:**
- ❌ **No cursor position tracking** - cannot insert/delete at arbitrary positions
- ❌ **No arrow key navigation** - left/right/up/down ignored
- ❌ **No word-level operations** - no Alt+Left/Right, Alt+Backspace
- ❌ **No Home/End support** - cannot jump to line start/end
- ❌ **No proper multi-line support** - Enter key is blocked
- ❌ **Backspace only removes last character** - not at cursor position

---

## 2. Reference Implementation Analysis

### 2.1 Bubbles Textarea (Go) - The Gold Standard

**File:** `/tmp/bubbles/textarea/textarea.go` (~1,400 lines)

**Data Structure:**
```go
type Model struct {
    value [][]rune         // Text as 2D array: rows of runes
    col   int              // Cursor column position
    row   int              // Cursor row position
    lastCharOffset int     // For vertical navigation consistency
    viewport *viewport.Model // For scrolling
    cache *memoization.MemoCache[line, [][]rune]  // Memoized wrapping
}
```

**Key Bindings (DefaultKeyMap):**
| Key | Action |
|-----|--------|
| `left`, `ctrl+f` | Character forward |
| `right`, `ctrl+b` | Character backward |
| `up`, `ctrl+n` | Next line |
| `down`, `ctrl+p` | Previous line |
| `alt+right`, `alt+f` | Word forward |
| `alt+left`, `alt+b` | Word backward |
| `home`, `ctrl+a` | Line start |
| `end`, `ctrl+e` | Line end |
| `alt+<`, `ctrl+home` | Input begin |
| `alt+>`, `ctrl+end` | Input end |
| `backspace`, `ctrl+h` | Delete character backward |
| `delete`, `ctrl+d` | Delete character forward |
| `alt+backspace`, `ctrl+w` | Delete word backward |
| `alt+delete`, `alt+d` | Delete word forward |
| `ctrl+k` | Delete after cursor |
| `ctrl+u` | Delete before cursor |
| `enter`, `ctrl+m` | Insert newline |
| `ctrl+v` | Paste |
| `alt+c` | Capitalize word forward |
| `alt+l` | Lowercase word forward |
| `alt+u` | Uppercase word forward |
| `ctrl+t` | Transpose characters |

**Core Algorithms:**

1. **Cursor Movement Across Lines:**
   ```go
   func (m *Model) CursorDown() {
       li := m.LineInfo()
       charOffset := max(m.lastCharOffset, li.CharOffset)
       m.lastCharOffset = charOffset  // Remember column intent
       
       if li.RowOffset+1 >= li.Height && m.row < len(m.value)-1 {
           m.row++
           m.col = 0
       }
       // ... maintain column position on new line
   }
   ```

2. **Character Left with Line Wrapping:**
   ```go
   func (m *Model) characterLeft(insideLine bool) {
       if m.col == 0 && m.row != 0 {
           m.row--
           m.CursorEnd()  // Go to end of previous line
           if !insideLine { return }
       }
       if m.col > 0 {
           m.SetCursor(m.col - 1)
       }
   }
   ```

3. **Line Splitting (Enter Key):**
   ```go
   func (m *Model) splitLine(row, col int) {
       head, tailSrc := m.value[row][:col], m.value[row][col:]
       tail := make([]rune, len(tailSrc))
       copy(tail, tailSrc)
       
       m.value = append(m.value[:row+1], m.value[row:]...)
       m.value[row] = head
       m.value[row+1] = tail
       
       m.col = 0
       m.row++
   }
   ```

4. **Line Merging (Backspace at start of line):**
   ```go
   func (m *Model) mergeLineAbove(row int) {
       if row <= 0 { return }
       m.col = len(m.value[row-1])
       m.row = m.row - 1
       m.value[row-1] = append(m.value[row-1], m.value[row]...)
       // Shift lines up...
       m.value = m.value[:len(m.value)-1]
   }
   ```

### 2.2 VTCode InputManager (Rust)

**File:** `/tmp/vtcode/vtcode-core/src/ui/tui/session/input_manager.rs` (~313 lines)

**Data Structure:**
```rust
pub struct InputManager {
    content: String,           // Full text as single string
    cursor: usize,             // Byte offset in string
    history: Vec<String>,      // Command history
    history_index: Option<usize>,
    history_draft: Option<String>,
}
```

**Key Operations:**

1. **UTF-8 Aware Cursor Movement:**
   ```rust
   pub fn move_cursor_left(&mut self) {
       if self.cursor > 0 {
           let mut pos = self.cursor - 1;
           while pos > 0 && !self.content.is_char_boundary(pos) {
               pos -= 1;  // Skip continuation bytes
           }
           self.cursor = pos;
       }
   }
   ```

2. **Insert at Cursor:**
   ```rust
   pub fn insert_char(&mut self, ch: char) {
       self.content.insert(self.cursor, ch);
       self.cursor += ch.len_utf8();
   }
   ```

3. **Backspace with UTF-8 Handling:**
   ```rust
   pub fn backspace(&mut self) {
       if self.cursor > 0 {
           let mut pos = self.cursor - 1;
           while pos > 0 && !self.content.is_char_boundary(pos) {
               pos -= 1;
           }
           self.content.drain(pos..self.cursor);
           self.cursor = pos;
       }
   }
   ```

**Word-Level Operations (editing.rs):**
```rust
pub(super) fn delete_word_backward(&mut self) {
    let graphemes: Vec<(usize, &str)> = self.input_manager.content()
        .grapheme_indices(true)
        .take_while(|(idx, _)| *idx < self.input_manager.cursor())
        .collect();
    
    // Skip trailing whitespace
    // Find word start
    // Delete from word start to cursor
}
```

### 2.3 Ink Framework (TypeScript/React)

**Files:** `/tmp/ink/src/hooks/use-input.ts`, `/tmp/ink/src/parse-keypress.ts`

**Key Detection:**
```typescript
type Key = {
    upArrow: boolean;
    downArrow: boolean;
    leftArrow: boolean;
    rightArrow: boolean;
    home: boolean;
    end: boolean;
    return: boolean;
    backspace: boolean;
    delete: boolean;
    ctrl: boolean;
    shift: boolean;
    meta: boolean;
    tab: boolean;
    pageUp: boolean;
    pageDown: boolean;
};

useInput((input: string, key: Key) => {
    // Handle each key...
});
```

**Escape Sequence Parsing (`parse-keypress.ts`):**
```typescript
const keyName: Record<string, string> = {
    '[A': 'up',
    '[B': 'down',
    '[C': 'right',
    '[D': 'left',
    '[H': 'home',
    '[F': 'end',
    '[3~': 'delete',
    // ... many more
};
```

Ink already parses all arrow keys, modifiers, and special keys - we just need to USE them.

---

## 3. Proposed Solution

### 3.1 State Structure

```typescript
interface MultiLineInputState {
    lines: string[];           // Array of lines (split on \n)
    cursorRow: number;         // Current line index (0-based)
    cursorCol: number;         // Column within line (character offset)
    lastColIntent: number;     // For vertical navigation (remember column)
    selectionStart?: { row: number; col: number };  // Optional selection
    selectionEnd?: { row: number; col: number };
}
```

### 3.2 Key Bindings

| Key | Action | Implementation |
|-----|--------|----------------|
| `←` | Move cursor left | `moveCursorLeft()` - wrap to previous line at col 0 |
| `→` | Move cursor right | `moveCursorRight()` - wrap to next line at end |
| `↑` | Move cursor up | `moveCursorUp()` - maintain column intent |
| `↓` | Move cursor down | `moveCursorDown()` - maintain column intent |
| `Home` / `Ctrl+A` | Start of line | `setCursorCol(0)` |
| `End` / `Ctrl+E` | End of line | `setCursorCol(lines[row].length)` |
| `Ctrl+Home` | Start of input | `setCursor(0, 0)` |
| `Ctrl+End` | End of input | `setCursor(lastRow, lastCol)` |
| `Alt+←` | Word left | `moveWordLeft()` |
| `Alt+→` | Word right | `moveWordRight()` |
| `Backspace` | Delete before cursor | `deleteCharBefore()` - merge lines if at col 0 |
| `Delete` | Delete at cursor | `deleteCharAt()` - merge lines if at end |
| `Alt+Backspace` | Delete word backward | `deleteWordBackward()` |
| `Ctrl+U` | Delete to line start | `deleteToLineStart()` |
| `Ctrl+K` | Delete to line end | `deleteToLineEnd()` |
| `Enter` | Insert newline | `insertNewline()` - split line at cursor |
| Printable | Insert character | `insertChar(ch)` |

### 3.3 Core Functions

```typescript
// Cursor Movement
function moveCursorLeft(): void {
    if (cursorCol > 0) {
        setCursorCol(cursorCol - 1);
    } else if (cursorRow > 0) {
        // Wrap to end of previous line
        setCursorRow(cursorRow - 1);
        setCursorCol(lines[cursorRow - 1].length);
    }
    setLastColIntent(cursorCol);
}

function moveCursorUp(): void {
    if (cursorRow > 0) {
        setCursorRow(cursorRow - 1);
        // Use lastColIntent, clamped to line length
        const targetCol = Math.min(lastColIntent, lines[cursorRow - 1].length);
        setCursorCol(targetCol);
    }
}

// Text Editing
function insertChar(ch: string): void {
    const line = lines[cursorRow];
    const newLine = line.slice(0, cursorCol) + ch + line.slice(cursorCol);
    setLines([...lines.slice(0, cursorRow), newLine, ...lines.slice(cursorRow + 1)]);
    setCursorCol(cursorCol + ch.length);
}

function deleteCharBefore(): void {
    if (cursorCol > 0) {
        const line = lines[cursorRow];
        const newLine = line.slice(0, cursorCol - 1) + line.slice(cursorCol);
        setLines([...lines.slice(0, cursorRow), newLine, ...lines.slice(cursorRow + 1)]);
        setCursorCol(cursorCol - 1);
    } else if (cursorRow > 0) {
        // Merge with previous line
        const prevLine = lines[cursorRow - 1];
        const currentLine = lines[cursorRow];
        const mergedLine = prevLine + currentLine;
        const newCursorCol = prevLine.length;
        setLines([
            ...lines.slice(0, cursorRow - 1),
            mergedLine,
            ...lines.slice(cursorRow + 1)
        ]);
        setCursorRow(cursorRow - 1);
        setCursorCol(newCursorCol);
    }
}

function insertNewline(): void {
    const line = lines[cursorRow];
    const beforeCursor = line.slice(0, cursorCol);
    const afterCursor = line.slice(cursorCol);
    setLines([
        ...lines.slice(0, cursorRow),
        beforeCursor,
        afterCursor,
        ...lines.slice(cursorRow + 1)
    ]);
    setCursorRow(cursorRow + 1);
    setCursorCol(0);
}

// Word Operations
function moveWordLeft(): void {
    // Skip whitespace backwards, then skip non-whitespace
    let col = cursorCol;
    let row = cursorRow;
    const text = lines[row];
    
    // Skip trailing whitespace
    while (col > 0 && /\s/.test(text[col - 1])) col--;
    // Skip word characters
    while (col > 0 && !/\s/.test(text[col - 1])) col--;
    
    setCursorCol(col);
}
```

### 3.4 Rendering with Cursor

```typescript
const renderContent = (): React.ReactNode => {
    return lines.map((line, lineIdx) => {
        if (lineIdx !== cursorRow) {
            return <Text key={lineIdx}>{line || ' '}</Text>;
        }
        
        // Current line - render with cursor
        const before = line.slice(0, cursorCol);
        const cursorChar = line[cursorCol] || ' ';
        const after = line.slice(cursorCol + 1);
        
        return (
            <Text key={lineIdx}>
                {before}
                <Text inverse>{cursorChar}</Text>
                {after}
            </Text>
        );
    });
};
```

### 3.5 Input Handler

```typescript
useInput((input, key) => {
    // Arrow keys
    if (key.leftArrow) {
        if (key.meta || key.alt) moveWordLeft();
        else moveCursorLeft();
        return;
    }
    if (key.rightArrow) {
        if (key.meta || key.alt) moveWordRight();
        else moveCursorRight();
        return;
    }
    if (key.upArrow) { moveCursorUp(); return; }
    if (key.downArrow) { moveCursorDown(); return; }
    
    // Home/End
    if (key.home) {
        if (key.ctrl) { setCursor(0, 0); }
        else { setCursorCol(0); }
        return;
    }
    if (key.end) {
        if (key.ctrl) { setCursor(lastRow, lines[lastRow].length); }
        else { setCursorCol(lines[cursorRow].length); }
        return;
    }
    
    // Deletion
    if (key.backspace) {
        if (key.meta || key.alt) deleteWordBackward();
        else deleteCharBefore();
        return;
    }
    if (key.delete) { deleteCharAt(); return; }
    
    // Ctrl shortcuts
    if (key.ctrl) {
        if (input === 'u') { deleteToLineStart(); return; }
        if (input === 'k') { deleteToLineEnd(); return; }
        if (input === 'a') { setCursorCol(0); return; }
        if (input === 'e') { setCursorCol(lines[cursorRow].length); return; }
    }
    
    // Enter
    if (key.return) {
        if (key.shift) { insertNewline(); }  // Shift+Enter for newline
        else { onSubmit(); }  // Enter to submit
        return;
    }
    
    // Printable characters
    if (input && !key.ctrl && !key.meta) {
        insertString(input);
    }
}, { isActive });
```

---

## 4. Implementation Considerations

### 4.1 Unicode/Grapheme Handling

For proper Unicode support, use grapheme segmentation:

```typescript
import GraphemeSplitter from 'grapheme-splitter';
const splitter = new GraphemeSplitter();

// Split line into graphemes for proper cursor movement
const graphemes = splitter.splitGraphemes(line);
// Move cursor by grapheme, not by UTF-16 code unit
```

This handles:
- Emoji (👨‍👩‍👧‍👦 = 1 grapheme, 11 code points)
- Combining characters (é = 1 grapheme, may be 2 code points)
- CJK characters (proper width calculation needed)

### 4.2 Wide Characters (CJK)

Use `wcwidth` or similar for display width:

```typescript
import wcwidth from 'wcwidth';

const displayWidth = (str: string): number => {
    let width = 0;
    for (const ch of str) {
        width += wcwidth(ch) || 1;
    }
    return width;
};
```

### 4.3 Soft Wrapping

For display purposes, may need to soft-wrap long lines:

```typescript
const wrapLine = (line: string, maxWidth: number): string[] => {
    const wrapped: string[] = [];
    let current = '';
    let currentWidth = 0;
    
    for (const grapheme of splitter.splitGraphemes(line)) {
        const w = displayWidth(grapheme);
        if (currentWidth + w > maxWidth && current) {
            wrapped.push(current);
            current = '';
            currentWidth = 0;
        }
        current += grapheme;
        currentWidth += w;
    }
    if (current) wrapped.push(current);
    return wrapped;
};
```

### 4.4 Scroll/Viewport for Tall Content

If content exceeds available height, implement scrolling:

```typescript
const [scrollOffset, setScrollOffset] = useState(0);
const visibleHeight = 5; // configurable

// Ensure cursor is visible
useEffect(() => {
    if (cursorRow < scrollOffset) {
        setScrollOffset(cursorRow);
    } else if (cursorRow >= scrollOffset + visibleHeight) {
        setScrollOffset(cursorRow - visibleHeight + 1);
    }
}, [cursorRow]);

// Render only visible lines
const visibleLines = lines.slice(scrollOffset, scrollOffset + visibleHeight);
```

---

## 5. Estimated Effort

| Component | Lines of Code | Complexity |
|-----------|--------------|------------|
| State management | ~50 | Low |
| Cursor movement functions | ~80 | Medium |
| Text editing functions | ~100 | Medium |
| Word operations | ~60 | Medium |
| Rendering with cursor | ~40 | Low |
| Input handler | ~80 | Medium |
| Unicode/grapheme handling | ~40 | Medium |
| Optional: Soft wrapping | ~50 | Medium |
| Optional: Scroll/viewport | ~40 | Low |

**Total: 400-540 lines of TypeScript**

For comparison:
- Bubbles textarea: ~1,400 lines (Go)
- VTCode InputManager + editing: ~500+ lines (Rust)

---

## 6. Recommended Approach

### Phase 1: Core Functionality (MVP)
1. State structure with lines array and cursor position
2. Basic cursor movement (left/right/up/down)
3. Character insertion at cursor
4. Backspace/delete at cursor (with line merging)
5. Home/End navigation
6. Cursor rendering with inverse character

### Phase 2: Enhanced Editing
1. Word-level navigation (Alt+Arrow)
2. Word-level deletion (Alt+Backspace)
3. Line-level deletion (Ctrl+U, Ctrl+K)
4. Ctrl+Home/End for input boundaries

### Phase 3: Polish
1. Grapheme-aware cursor movement
2. Wide character handling
3. Soft wrapping for display
4. Viewport scrolling for tall content
5. Paste handling (multi-line)

---

## 7. Files to Reference

When implementing, refer to these files for patterns:

1. **Bubbles textarea** (most comprehensive):
   - `/tmp/bubbles/textarea/textarea.go` - Full implementation

2. **VTCode** (Rust patterns):
   - `/tmp/vtcode/vtcode-core/src/ui/tui/session/input_manager.rs` - State management
   - `/tmp/vtcode/vtcode-core/src/ui/tui/session/editing.rs` - Editing operations
   - `/tmp/vtcode/vtcode-core/src/ui/tui/session/input.rs` - Rendering

3. **Ink** (TypeScript/React):
   - `/tmp/ink/src/hooks/use-input.ts` - Input hook API
   - `/tmp/ink/src/parse-keypress.ts` - Key parsing

4. **Current fspec** (to replace):
   - `/Users/rquast/projects/fspec/src/tui/components/AgentView.tsx` lines 203-301

---

## 8. Conclusion

The current `SafeTextInput` component is inadequate for professional text editing. By implementing the proposed solution based on patterns from bubbles/textarea, VTCode, and leveraging Ink's existing key parsing, we can create a robust multi-line text input that supports:

- Full cursor navigation (arrows, Home/End, Ctrl+Home/End)
- Word-level operations (Alt+arrows, Alt+Backspace)
- Proper text editing (insert/delete at cursor position)
- Line operations (Enter to split, Backspace to merge)
- Multi-line display with cursor visualization

The estimated effort is 400-540 lines of TypeScript, which is reasonable given the complexity of the feature and the reference implementations.
