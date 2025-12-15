# Codex Text Styling Features - Research Document

**Date:** 2025-12-03
**Source:** https://github.com/openai/codex
**Purpose:** Document all text styling and highlighting features in codex for potential implementation in codelet

---

## Table of Contents

1. [Markdown Rendering](#markdown-rendering)
2. [Bash Syntax Highlighting](#bash-syntax-highlighting)
3. [Diff Rendering](#diff-rendering)
4. [Shimmer Effects](#shimmer-effects)
5. [Terminal Palette Adaptation](#terminal-palette-adaptation)
6. [User Message Styling](#user-message-styling)
7. [Implementation Recommendations](#implementation-recommendations)

---

## 1. Markdown Rendering

**File:** `/tmp/codex/codex-rs/tui/src/markdown_render.rs`

### Features Implemented

Codex uses `ratatui` + `pulldown-cmark` for comprehensive markdown rendering with the following styles:

#### Text Formatting
- **Bold** - `Style::new().bold()`
- *Italic* - `Style::new().italic()`
- ~~Strikethrough~~ - `Style::new().crossed_out()`
- `Inline code` - `Style::new().cyan()`

#### Headers (H1-H6)
Each header level has distinct styling:
- **H1:** Bold + Underlined (`Style::new().bold().underlined()`)
- **H2:** Bold (`Style::new().bold()`)
- **H3:** Bold + Italic (`Style::new().bold().italic()`)
- **H4-H6:** Italic (`Style::new().italic()`)

All headers show `#` prefix (e.g., `# H1 Header`, `## H2 Header`)

#### Lists
- **Unordered:** `-` markers with default style
- **Ordered:** Numbered markers (1., 2., 3.) in light blue (`Style::new().light_blue()`)
- Supports nested lists with proper indentation

#### Blockquotes
- Green color (`Style::new().green()`)
- `>` prefix on each line

#### Links
- Cyan + Underlined (`Style::new().cyan().underlined()`)
- Shows URL in parentheses: `[text](https://url)` displays as `text (https://url)`

#### Code Blocks
- **NO syntax highlighting** for code blocks
- Simple cyan color applied to all code (`Style::new().cyan()`)
- The `lang` parameter in `start_codeblock(_lang: Option<String>)` is **unused** (prefixed with `_`)
- Preserves whitespace and formatting

#### Horizontal Rules
- Rendered as: `———`

### Key Observations

✅ **Our current implementation matches codex exactly**
❌ **Codex does NOT do syntax highlighting for markdown code blocks**
✅ Simple, fast, and effective approach

---

## 2. Bash Syntax Highlighting

**File:** `/tmp/codex/codex-rs/tui/src/render/highlight.rs`

### Purpose

**NOT used in markdown rendering!** Only used for bash command approval overlays when showing commands before execution.

### Implementation

Uses `tree-sitter-highlight` with `tree-sitter-bash` grammar for minimal syntax highlighting.

#### Highlight Categories

```rust
enum BashHighlight {
    Comment,    // Dimmed
    Constant,   // Default
    Embedded,   // Default
    Function,   // Default
    Keyword,    // Default
    Number,     // Default
    Operator,   // Dimmed
    Property,   // Default
    String,     // Dimmed
}
```

#### Styling Applied

- **Dimmed elements:** Comments, Operators, Strings
- **Default elements:** Everything else (no special styling)
- **NO colors** - only `Modifier::DIM` is used
- For terminals without true color: Uses BOLD for highlights

#### Example

```bash
echo "hi" && bar | qux
```

Renders as:
- `echo` - Default (no dimming)
- `"hi"` - **Dimmed**
- `&&` - **Dimmed**
- `bar` - Default
- `|` - **Dimmed**
- `qux` - Default

### Usage Location

**File:** `app.rs:865`

```rust
ApprovalRequest::Exec { command, .. } => {
    let full_cmd = strip_bash_lc_and_escape(&command);
    let full_cmd_lines = highlight_bash_to_lines(&full_cmd);
    self.overlay = Some(Overlay::new_static_with_lines(
        full_cmd_lines,
        "E X E C".to_string(),
    ));
}
```

### Key Observations

✅ Minimal, tasteful highlighting
✅ Falls back gracefully on terminals without RGB support
✅ Fast (tree-sitter streaming)
❌ **Only for bash commands, not general code blocks**

---

## 3. Diff Rendering

**File:** `/tmp/codex/codex-rs/tui/src/diff_render.rs`

### Purpose

Render git diffs with color-coded additions and deletions.

### Features

#### Color Scheme

- **Additions:** Green (`Color::Green`)
- **Deletions:** Red (`Color::Red`)
- **Context:** Default color

#### Diff Summary Display

```
• Edited 2 files +15 -3
  └ src/main.rs +10 -2
    1 + added line
    2 - removed line
    3   context line
  └ src/lib.rs +5 -1
```

#### Styling Details

- Added lines: `+{count}` in green
- Removed lines: `-{count}` in red
- Line numbers with appropriate width padding
- File paths with relative paths from cwd
- Indentation for nested file chunks

#### Diff Parser

Uses `diffy` crate for unified diff parsing.

### Key Observations

✅ Clear visual distinction between additions/deletions
✅ Supports Add, Delete, and Update file changes
✅ Handles move/rename operations
✅ Word wrapping for long diff lines

---

## 4. Shimmer Effects

**File:** `/tmp/codex/codex-rs/tui/src/shimmer.rs`

### Purpose

Animated gradient effect for loading states or processing indicators.

### How It Works

Creates a time-based sweeping gradient across text:

1. **Synchronization:** Uses process start time for consistent animation
2. **Sweep:** 2-second period for gradient to traverse text
3. **Band:** 5-character-wide highlight band with cosine falloff
4. **Blending:** Interpolates between foreground and background colors

#### Color Calculation

```rust
// For true color terminals
let highlight = blend(highlight_color, base_color, intensity * 0.9);
Style::default().fg(Color::Rgb(r, g, b)).add_modifier(Modifier::BOLD)

// For terminals without RGB
if intensity < 0.2 { Style::default().add_modifier(Modifier::DIM) }
else if intensity < 0.6 { Style::default() }
else { Style::default().add_modifier(Modifier::BOLD) }
```

### Visual Effect

```
[DIM] processing... [BOLD] ... [DEFAULT]
      ^           ^         ^
      |           |         |
   Outside    In band   Outside
```

The band sweeps left-to-right continuously, creating a shimmer effect.

### Key Observations

✅ Elegant loading indicator
✅ Graceful fallback for non-RGB terminals
✅ Minimal CPU usage (time-based, not frame-based)
⚠️ Requires periodic redraws for animation

---

## 5. Terminal Palette Adaptation

**File:** `/tmp/codex/codex-rs/tui/src/terminal_palette.rs`

### Purpose

Detect terminal's default foreground/background colors and adapt UI styling accordingly.

### Features

#### Color Detection

- Queries terminal for default colors using ANSI escape sequences
- Caches colors in static `OnceLock` for performance
- Requeries on focus change events

#### API

```rust
pub fn default_fg() -> Option<(u8, u8, u8)>
pub fn default_bg() -> Option<(u8, u8, u8)>
pub fn best_color(target: (u8, u8, u8)) -> Color
pub fn requery_default_colors()
```

#### Usage

```rust
// Determine if terminal background is light or dark
let bg = default_bg().unwrap_or((0, 0, 0));
if is_light(bg) {
    // Use dark text
} else {
    // Use light text
}
```

### Key Observations

✅ Respects user's terminal theme
✅ Prevents poor contrast issues
✅ Platform-specific implementations (Unix/Windows)
⚠️ Falls back to defaults if detection fails

---

## 6. User Message Styling

**File:** `/tmp/codex/codex-rs/tui/src/style.rs`

### Purpose

Style user-authored messages with subtle background color that adapts to terminal theme.

### Implementation

```rust
pub fn user_message_style() -> Style {
    match terminal_bg {
        Some(bg) => {
            // Blend 10% toward white (dark terminal) or black (light terminal)
            let top = if is_light(bg) { (0, 0, 0) } else { (255, 255, 255) };
            let color = blend(top, bg, 0.1);
            Style::default().bg(best_color(color))
        }
        None => Style::default()
    }
}
```

### Visual Effect

Creates a subtle background tint that:
- Makes user messages visually distinct from assistant messages
- Adapts to light or dark terminals
- Uses minimal contrast (only 10% blend)

### Key Observations

✅ Subtle, professional appearance
✅ Theme-aware
✅ Accessible (minimal contrast change)

---

## 7. Implementation Recommendations

### Priority 1: Already Implemented ✅

- ✅ Markdown rendering (headers, lists, blockquotes, links, code)
- ✅ ANSI conversion for terminal output

### Priority 2: High Value 🎯

1. **Diff Rendering** (for file changes, patches)
   - Uses: `diffy` crate
   - Complexity: Medium
   - Value: High (essential for code editing workflows)

2. **Bash Syntax Highlighting** (for command previews)
   - Uses: `tree-sitter-highlight` + `tree-sitter-bash`
   - Complexity: Low (already proven in codex)
   - Value: Medium (nice-to-have for command confirmation)

### Priority 3: Nice-to-Have 💡

3. **Terminal Palette Adaptation**
   - Uses: ANSI escape sequence queries
   - Complexity: Medium (platform-specific)
   - Value: Medium (better theme compatibility)

4. **User Message Styling**
   - Uses: Background color blending
   - Complexity: Low
   - Value: Low (cosmetic improvement)

### Priority 4: Optional ⭐

5. **Shimmer Effects** (for loading states)
   - Uses: Time-based animation
   - Complexity: Low
   - Value: Low (purely aesthetic)

6. **General Syntax Highlighting** (for code blocks)
   - Uses: `tree-sitter-highlight` with multiple grammars
   - Complexity: HIGH (grammar loading, language detection, style mapping)
   - Value: Medium
   - **Note:** Codex does NOT implement this - they use simple cyan

---

## Dependencies Required

```toml
# For diff rendering
diffy = "0.4"

# For bash syntax highlighting
tree-sitter-highlight = "0.24"
tree-sitter-bash = "0.23"

# For terminal color detection
supports-color = "3.0"

# Already have
pulldown-cmark = "0.12"  # Markdown parsing
ratatui = "0.29"         # TUI rendering
```

---

## Technical Notes

### Why Codex Doesn't Do General Syntax Highlighting

1. **Performance:** Loading tree-sitter grammars for every language is expensive
2. **Complexity:** Need to bundle grammars, detect languages, map highlight queries
3. **Diminishing Returns:** Simple cyan color is fast and "good enough"
4. **Streaming:** Syntax highlighting complicates incremental rendering

### Why Bash Highlighting Works

1. **Single language:** Only need bash grammar
2. **Known context:** Command previews are always bash
3. **Minimal styling:** Just dimming, no complex color schemes
4. **Small scope:** Few hundred lines of code total

---

## Conclusion

**Current Status:**
✅ Codelet matches codex's markdown rendering exactly

**Recommended Next Steps:**
1. **Diff rendering** - High value for code editing workflows
2. **Bash highlighting** - Low complexity, professional touch for command previews
3. **Terminal adaptation** - Better theme compatibility

**Not Recommended:**
- General syntax highlighting for code blocks (too complex, codex doesn't do it)
- Shimmer effects (purely cosmetic)

---

## References

- Codex Repository: https://github.com/openai/codex
- Tree-sitter: https://tree-sitter.github.io/tree-sitter/
- Diffy Crate: https://docs.rs/diffy/
- Ratatui: https://ratatui.rs/
