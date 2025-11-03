# TUI-015 Visual Rules Guide

This document provides ASCII visual representations for every rule in TUI-015.

---

## Rule [0]: NO string injection - all content must be rendered as Ink components

### ❌ WRONG (String Injection):
```typescript
rows.push('│ ┏┓┏┓┏┓┏┓┏┓  Checkpoints: 3 Manual │');
```

### ✅ CORRECT (Component-Based):
```tsx
<Box borderStyle="single">
  <Box flexDirection="row">
    <Logo />
    <CheckpointStatus />
  </Box>
</Box>
```

---

## Rule [1]: Use ONLY flexbox (flexGrow/flexShrink) and static heights - NEVER percentages

### ❌ WRONG:
```tsx
<Box width="50%">  {/* NEVER use percentages */}
```

### ✅ CORRECT:
```tsx
<Box flexGrow={1}>  {/* Use flexGrow */}
<Box width={12} flexShrink={1}>  {/* Static width with flexShrink */}
```

---

## Rule [2]: Header container must have internal height of 4 lines

### Visual Structure:
```
┌────────────────────────────────────┐  ← Box top border (NOT counted in internal)
│ ┏┓┏┓┏┓┏┓┏┓  Checkpoints: 3 Manual  │  ← Line 1 (internal)
│ ┣ ┗┓┃┃┣ ┃  5 Auto                  │  ← Line 2 (internal)
│ ┻ ┗┛┣┛┗┛┗┛  ──────────────────────  │  ← Line 3 (internal)
│             C View ◆ F View         │  ← Line 4 (internal)
└────────────────────────────────────┘  ← Box bottom border (NOT counted)
```

**Total height: 6 rows (4 internal + 2 borders)**

---

## Rule [3]: Work unit details container must have internal height of 5 lines

### Visual Structure:
```
├────────────────────────────────────┤  ← Junction connecting to header above
│ TUI-015: Feature Title             │  ← Line 1: Title (height={1})
│ This is a long description text    │  ← Line 2: Description line 1
│ that wraps across multiple lines   │  ← Line 3: Description line 2
│ and uses flexGrow to fill space    │  ← Line 4: Description line 3
│ Epic: auth | Estimate: 5pts        │  ← Line 5: Metadata (height={1})
├────────────────────────────────────┤  ← Junction connecting to columns below
```

**Total height: 5 content lines (borders via junction separators)**

---

## Rule [4]: Borders must use hybrid border system

### Visual Connection (Single Continuous Border Structure):
```
┌────────────────────┐  ← Header Box: borderStyle='single' (top)
│ Logo    Info       │
│         ────────   │
│         Shortcuts  │
├────────────────────┤  ← Junction: header connects to details (ONE LINE)
│ Details            │  ← Details: borderLeft│ + borderRight│ only
│ Content            │
├─┬──────────┬───────┤  ← Junction with column dividers: details → columns
│ │ Column 1 │ Col 2 │  ← Columns with │ separators
└─┴──────────┴───────┘  ← Bottom junction closes structure
```

**Key: Single continuous structure using junctions (├┤┬┼┴) to connect sections**

---

## Rule [5]: Logo component width is 12 characters

### Visual Layout:
```
<────── 12 chars ──────><──── flexGrow fills remaining ────>
┌─────────────────────────────────────────────────────────┐
│ ┏┓┏┓┏┓┏┓┏┓  Checkpoints: 3 Manual, 5 Auto              │
│ ┣ ┗┓┃┃┣ ┃  ──────────────────────────────────────────  │
│ ┻ ┗┛┣┛┗┛┗┛  C View Checkpoints ◆ F View Changed Files  │
│             (InfoContainer flexGrow fills this space)   │
└─────────────────────────────────────────────────────────┘
```

---

## Rule [6]: KeybindingShortcuts component has borderTop only

### Visual (within InfoContainer):
```
│ Checkpoints: 3 Manual, 5 Auto                           │
├─────────────────────────────────────────────────────────┤  ← borderTop
│ C View Checkpoints ◆ F View Changed Files               │  ← Content
```

**Border props: `borderTop={true}`, all others `={false}`**
**Note: This borderTop is INSIDE the HeaderContainer Box, not a separate line**

---

## Rule [7]: WorkUnitDescription component height is 3 lines using flexGrow

### Visual:
```
│ TUI-015: Feature Title              │  ← Title (height={1})
│ This is the description line 1      │  ← Description (flexGrow fills 3 lines)
│ This is the description line 2      │
│ This is the description line 3...   │
│ Epic: auth | Estimate: 5pts         │  ← Metadata (height={1})
```

**Note: Uses `flexGrow={1}` NOT `height={3}`**

---

## Rule [8]: InfoContainer must use flexGrow to fill remaining horizontal space

### Visual:
```
<Box flexDirection="row">
  ┌───12ch───┐┌──────── flexGrow fills rest ─────────┐
  │ Logo     ││ InfoContainer                        │
  │ (fixed)  ││ (grows to fill)                      │
  └──────────┘└───────────────────────────────────────┘
</Box>
```

---

## Rule [9]: CheckpointStatus and KeybindingShortcuts use flexGrow for vertical space

### Visual (within HeaderContainer):
```
┌─────────────────────────────────────────┐
│ Logo    Checkpoints: 3 Manual, 5 Auto   │  ← CheckpointStatus (flexGrow ~2 lines)
│         ─────────────────────────────    │  ← KeybindingShortcuts borderTop
│         C View ◆ F View                  │  ← KeybindingShortcuts (flexGrow ~2 lines)
│         (internal to InfoContainer)      │
└─────────────────────────────────────────┘
```

**Both use flexGrow to distribute vertical space within InfoContainer height={4}**

---

## Rule [10]: WorkUnitDetailsContainer must NOT use Box borderStyle

### ❌ WRONG:
```tsx
<Box borderStyle="single">  {/* NO! Creates all 4 borders */}
  <WorkUnitDetails />
</Box>
```

### ✅ CORRECT:
```tsx
{/* Separator renders top border as junction */}
<Text>├────────────────┤</Text>
{/* Box only has left/right borders */}
<Box borderLeft borderRight>
  <WorkUnitDetails />
</Box>
{/* Separator renders bottom border as junction */}
<Text>├────────────────┤</Text>
```

---

## Rule [11]: All text rendering must be Text components

### ❌ WRONG:
```typescript
rows.push('│' + content + '│');  // String concatenation
```

### ✅ CORRECT:
```tsx
<Box>
  <Text>{content}</Text>  {/* Text component */}
</Box>
```

---

## Rule [12]: HeaderContainer uses Box with borderStyle='single' for all four borders

### Visual:
```
┌─────────────────────┐  ← borderTop (from Box)
│                     │  ← borderLeft
│   Header Content    │
│                     │  ← borderRight
└─────────────────────┘  ← borderBottom (from Box, becomes junction)
```

**All 4 borders from single `borderStyle='single'` prop**
**Bottom border connects to junction separator below**

---

## Rule [13]: Logo component width MUST be exactly 12 characters (4 rows of 12ch each)

### Visual (with char counting):
```
Row 1: "┏┓┏┓┏┓┏┓┏┓ " (12 chars including trailing space)
       123456789012

Row 2: "┣ ┗┓┃┃┣ ┃ " (12 chars)
       123456789012

Row 3: "┻ ┗┛┣┛┗┛┗┛ " (12 chars)
       123456789012

Row 4: "           " (12 spaces)
       123456789012
```

---

## Rule [14]: WorkUnitTitle must truncate WITHOUT ellipsis

### Visual:
```
Container width: 30 chars
Title: "BOARD-001: Very Long Feature Title That Exceeds Width"

❌ WRONG: "BOARD-001: Very Long Feat..."  (with ellipsis)
✅ CORRECT: "BOARD-001: Very Long Featur"  (clean truncate)
```

---

## Rule [15]: WorkUnitDescription must truncate line 3 WITH ellipsis

### Visual:
```
Full text: "This is a very long description that spans many lines and needs to be truncated properly when it exceeds the three line limit"

Line 1: "This is a very long description that"
Line 2: "spans many lines and needs to be"
Line 3: "truncated properly when it exce..."  ← Note ellipsis
(Lines 4+ discarded)
```

---

## Rule [16]: WorkUnitMetadata displays available fields only with ' | ' separator

### Visual:
```
All fields:
│ Epic: auth | Estimate: 5pts | Status: implementing │

Some fields:
│ Estimate: 3pts │  (no epic, no status = no separators)

One field:
│ Epic: authentication │  (just epic, no separators)
```

---

## Rule [17]: When no work unit selected: centered message, empty lines

### Visual:
```
│            No work unit selected           │  ← Centered on line 1
│                                            │  ← Line 2: empty
│                                            │  ← Line 3: empty
│                                            │  ← Line 4: empty
│                                            │  ← Line 5: empty
```

**NOT "No description" - just centered title and empty lines**

---

## Rule [19]: KeybindingShortcuts component renders its own borderTop

### Component Structure:
```tsx
<Box borderTop={true} borderBottom={false} borderLeft={false} borderRight={false}>
  <Text>C View Checkpoints ◆ F View Changed Files</Text>
</Box>
```

### Visual Result (within InfoContainer):
```
│ Checkpoints: 3 Manual, 5 Auto              │
├─────────────────────────────────────────────┤  ← borderTop from KeybindingShortcuts Box
│ C View Checkpoints ◆ F View Changed Files  │  ← Text content
```

**Note: This is INSIDE HeaderContainer, not a separate structure**

---

## Rule [20]: Separator after header must use 'plain' separator type

### Visual (Single Continuous Structure):
```
┌─────────────────────┐  ← Header Box top
│ Logo    Info        │
│         ────────    │
│         Shortcuts   │
├─────────────────────┤  ← Junction: header → details (ONE LINE, 'plain' type)
│ Details content     │  ← Work unit details start
```

**buildBorderRow(..., 'plain') produces junction `├─────┤` that extends vertical borders**

---

## Rule [21]: WorkUnitDescription MUST be bold cyan text

### Code:
```tsx
<Text bold color="cyan">
  {descriptionLine}
</Text>
```

### Visual (with color indication):
```
│ TUI-015: Feature Title                      │  ← Normal text
│ [BOLD CYAN] This is description line 1      │  ← chalk.cyan.bold
│ [BOLD CYAN] This is description line 2      │
│ [BOLD CYAN] This is description line 3...   │
│ Epic: auth | Estimate: 5pts                 │  ← Normal text
```

---

## Rule [22]: Header total height is 6 lines (1+4+1)

### Height Breakdown:
```
Row 1: ┌─────────────────────┐  ← Top border (1 line)
Row 2: │ Logo line 1         │  ┐
Row 3: │ Logo line 2         │  │ Content (4 lines)
Row 4: │ Logo line 3         │  │
Row 5: │ Logo line 4         │  ┘
Row 6: └─────────────────────┘  ← Bottom border (1 line, becomes junction)

Total: 6 rows
```

---

## Rule [23]: Details total height is 5 content lines

### Height Breakdown:
```
├─────────────────────┤  ← Junction (NOT counted in details height)
│ Line 1: Title       │  ┐
│ Line 2: Desc line 1 │  │
│ Line 3: Desc line 2 │  │ 5 content lines
│ Line 4: Desc line 3 │  │
│ Line 5: Metadata    │  ┘
├─────────────────────┤  ← Junction (NOT counted in details height)
```

---

## Rule [24]: WorkUnitTitle and WorkUnitMetadata must have height={1}

### Component Structure:
```tsx
<Box flexDirection="column" height={5}>
  <Box height={1}>  {/* Title: static 1 line */}
    <Text>TUI-015: Feature Title</Text>
  </Box>

  <Box flexGrow={1}>  {/* Description: grows to fill 3 lines */}
    <Text>Description...</Text>
  </Box>

  <Box height={1}>  {/* Metadata: static 1 line */}
    <Text>Epic: auth | Estimate: 5pts</Text>
  </Box>
</Box>
```

---

## Rule [25]: WorkUnitDescription must use flexGrow={1}, NOT height={3}

### ❌ WRONG:
```tsx
<Box height={3}>  {/* Hardcoded height */}
  <Text>Description</Text>
</Box>
```

### ✅ CORRECT:
```tsx
<Box flexGrow={1}>  {/* Flexbox grows to fill available space */}
  <Text>Description</Text>
</Box>
```

---

## Rule [26]: Text wrapping splits on spaces (word-wrap)

### Input:
```
"This is a long description"
Width: 15 chars
```

### ✅ CORRECT (word-wrap):
```
Line 1: "This is a long"
Line 2: "description"
```

### ❌ WRONG (char-wrap):
```
Line 1: "This is a long "
Line 2: "description"
```

---

## Rule [27]: Newlines must be normalized to spaces before wrapping

### Input:
```typescript
description = "Line 1\nLine 2\nLine 3"
```

### Processing:
```typescript
// Normalize newlines to spaces
const normalized = description.replace(/\n/g, ' ');
// Result: "Line 1 Line 2 Line 3"

// Then wrap
wrapText(normalized, width, 3);
```

### Output:
```
│ Line 1 Line 2     │
│ Line 3            │
│                   │
```

---

## Rule [28]: Component hierarchy - HeaderContainer

### Component Tree:
```
<HeaderContainer (Box borderStyle="single")>
  <Row (Box flexDirection="row")>
    <Logo (width={12} flexShrink={1}) />
    <InfoContainer (Box flexGrow={1} flexDirection="column" height={4})>
      <CheckpointStatus (flexGrow={1}) />
      <KeybindingShortcuts (flexGrow={1} borderTop) />
    </InfoContainer>
  </Row>
</HeaderContainer>
```

### Visual Structure:
```
┌─────────────────────────────────────────┐
│ ┌────┐ ┌──────────────────────────────┐ │
│ │Logo│ │CheckpointStatus (flexGrow)   │ │
│ │    │ ├──────────────────────────────┤ │  ← borderTop internal to InfoContainer
│ │    │ │KeybindingShortcuts(flexGrow) │ │
│ └────┘ └──────────────────────────────┘ │
└─────────────────────────────────────────┘
```

---

## Rule [29]: Component hierarchy - WorkUnitDetailsContainer

### Component Tree:
```
<WorkUnitDetailsContainer (Box borderLeft borderRight height={5} flexDirection="column")>
  <WorkUnitTitle (Box height={1})>
    <Text>TUI-015: Title</Text>
  </WorkUnitTitle>

  <WorkUnitDescription (Box flexGrow={1})>
    <Text bold color="cyan">Line 1</Text>
    <Text bold color="cyan">Line 2</Text>
    <Text bold color="cyan">Line 3...</Text>
  </WorkUnitDescription>

  <WorkUnitMetadata (Box height={1})>
    <Text>Epic: auth | Estimate: 5pts</Text>
  </WorkUnitMetadata>
</WorkUnitDetailsContainer>
```

---

## Rule [30]: Text content wrapped in Text components, Box handles layout

### ❌ WRONG:
```tsx
<Box>
  Title text here  {/* Text not wrapped */}
</Box>
```

### ✅ CORRECT:
```tsx
<Box>  {/* Box for layout */}
  <Text>Title text here</Text>  {/* Text for content */}
</Box>
```

---

## Rule [31]: String injection lines removed from 496-617

### Current Code (lines 496-617):
```typescript
rows.push('┌────────────────────────┐');
rows.push('│ ' + logoLine1 + ' ' + content + '│');
rows.push('│ ' + logoLine2 + ' ' + content + '│');
// ... DELETE ALL OF THIS
```

### Replace With (lines 707-740):
```tsx
<Box borderStyle="single">
  <Logo />
  <InfoContainer>
    <CheckpointStatus />
    <KeybindingShortcuts />
  </InfoContainer>
</Box>
```

---

## Rule [32]: Column viewport height is DYNAMIC

### Calculation:
```
Terminal Height: 24 rows

Static Components:
  Header Box:          6 rows  (1 top + 4 content + 1 bottom/junction)
  Separator:           1 row   (├────┤ junction)
  Details:             5 rows  (content only)
  Separator:           1 row   (├─┬──┤ junction with dividers)
  Column Headers:      2 rows  (header + separator)
  Footer Separator:    1 row   (├────┤ junction)
  Footer Text:         1 row   (text)
  Bottom Border:       1 row   (└────┘)
  ────────────────────────────
  Total Static:       18 rows

Column Viewport: 24 - 18 = 6 rows for columns
```

### Visual (Single Continuous Border Structure):
```
┌────────────────┐  ← Row 1-6: Header (static)
├────────────────┤  ← Row 7: Junction separator (static)
│ Details        │  ← Row 8-12: Details (static)
├─┬──────┬───────┤  ← Row 13: Junction with column dividers (static)
│ │COLUMN│COLUMN │  ← Row 14-15: Headers (static)
├─┼──────┼───────┤  ← Cross junction
│ │ WU-1 │ WU-5  │  ┐
│ │ WU-2 │ WU-6  │  │ Rows 16-21: DYNAMIC
│ │ WU-3 │ WU-7  │  │ (grows with terminal)
│ │ WU-4 │ WU-8  │  │
│ │      │       │  │
│ │      │       │  ┘
├─┴──────┴───────┤  ← Row 22: Footer junction (static)
│ Footer         │  ← Row 23: Footer text (static)
└────────────────┘  ← Row 24: Bottom border (static)
```

**Note: ALL borders are part of a single continuous structure using junctions**

---

## Rule [33]: Static component heights derived from structure, NOT magic numbers

### ❌ WRONG:
```typescript
const fixedRows = 19;  // Magic number!
```

### ✅ CORRECT:
```typescript
const headerHeight = 6;  // Calculated from Box structure
const separatorCount = 4;  // Count of separator lines
const detailsHeight = 5;  // Calculated from content structure
const columnHeadersHeight = 2;  // Header row + separator
const footerHeight = 2;  // Separator + text

const staticRows = headerHeight + separatorCount + detailsHeight +
                   columnHeadersHeight + footerHeight + 1;  // +1 for bottom border
```

---

## Rule [34]: Terminal resize recalculates column viewport height

### Before Resize (24 rows):
```
Header:   6 rows
Other:   12 rows
────────────────
Columns:  6 rows  ← Dynamic
```

### After Resize (40 rows):
```
Header:   6 rows  (unchanged)
Other:   12 rows  (unchanged)
────────────────
Columns: 22 rows  ← Grew!
```

---

## Rule [35]: NEVER hardcode fixedRows as magic number

### ❌ WRONG:
```typescript
const fixedRows = 17;  // Where did 17 come from?!
const viewport = terminalHeight - fixedRows;
```

### ✅ CORRECT:
```typescript
const staticComponents = {
  header: 6,
  separators: 4,
  details: 5,
  columnHeaders: 2,
  footer: 2,
  bottomBorder: 1
};
const fixedRows = Object.values(staticComponents).reduce((a, b) => a + b, 0);
const viewport = terminalHeight - fixedRows;
```

---

## Rule [36]: Each separator line counts as 1 row

### Visual Count:
```
Row 1:  ┌────────────────┐  ← Header top (Box border)
...
Row 6:  └────────────────┘  ← Header bottom (Box border, becomes junction)
Row 7:  ├────────────────┤  ← Junction separator (1 row)
...
Row 13: ├─┬──────┬───────┤  ← Junction with column dividers (1 row)
Row 14: │ │COLUMN│COLUMN │  ← Column header (1 row)
Row 15: ├─┼──────┼───────┤  ← Cross junction separator (1 row)
```

**Each buildBorderRow() call = 1 row**

---

## Rule [37]: InfoContainer height={4} is STATIC

### Visual:
```tsx
<InfoContainer height={4}>  {/* STATIC, NOT flexible */}
  ┌─ Line 1: Checkpoints text (flexGrow)
  │  Line 2: (CheckpointStatus continues)
  ├─ Line 3: Border separator (borderTop)
  └─ Line 4: Keybindings text (flexGrow)
</InfoContainer>
```

**Matches Logo's 4 internal lines for vertical alignment**

---

## Rule [38]: WorkUnitDetailsContainer must have explicit height={5}

### Visual:
```tsx
<Box height={5}>  {/* Explicit height ensures 5 rows */}
  Line 1: Title (height={1})
  Line 2: Description
  Line 3: Description    } flexGrow fills these 3
  Line 4: Description
  Line 5: Metadata (height={1})
</Box>
```

---

## Rule [39]: Current calculateViewportHeight uses hardcoded fixedRows=17 (WRONG)

### Current Code (line 79-99):
```typescript
const calculateViewportHeight = (terminalHeight: number): number => {
  const fixedRows = 17;  // ❌ WRONG: Magic number
  const availableRows = terminalHeight - fixedRows;
  return Math.max(5, availableRows);
};
```

### Must Calculate From Structure:
```typescript
const calculateViewportHeight = (terminalHeight: number): number => {
  const headerBoxHeight = 6;  // Box borders + 4 content lines
  const separatorAfterHeader = 1;
  const detailsHeight = 5;
  const separatorBeforeColumns = 1;
  const columnHeaderRow = 1;
  const headerSeparatorRow = 1;
  const footerSeparator = 1;
  const footerText = 1;
  const bottomBorder = 1;

  const fixedRows = headerBoxHeight + separatorAfterHeader + detailsHeight +
                    separatorBeforeColumns + columnHeaderRow + headerSeparatorRow +
                    footerSeparator + footerText + bottomBorder;

  return Math.max(5, terminalHeight - fixedRows);
};
```

---

## Rule [40]: wrapText helper used internally by WorkUnitDescription

### Helper Function (lines 110-133):
```typescript
const wrapText = (text: string, width: number, maxLines: number): string[] => {
  // Word-wrapping logic
};
```

### Usage in Component:
```tsx
const WorkUnitDescription = ({ text, width }) => {
  const lines = wrapText(text, width, 3);  // Internal usage

  return (
    <Box flexGrow={1}>
      {lines.map(line => (
        <Text bold color="cyan">{line}</Text>
      ))}
    </Box>
  );
};
```

---

## Rule [41]: fitToWidth replaced by Ink Text wrap='truncate' prop

### Current Code (lines 102-107):
```typescript
const fitToWidth = (text: string, width: number): string => {
  if (text.length > width) {
    return text.substring(0, width);  // Manual truncation
  }
  return text.padEnd(width, ' ');
};
```

### Replace With:
```tsx
<Text wrap="truncate">{text}</Text>  {/* Ink handles it */}
```

---

## Rule [42]: centerText helper used when selectedWorkUnit is null

### Helper (lines 136-144):
```typescript
const centerText = (text: string, width: number): string => {
  const totalPadding = width - text.length;
  const leftPadding = Math.floor(totalPadding / 2);
  const rightPadding = totalPadding - leftPadding;
  return ' '.repeat(leftPadding) + text + ' '.repeat(rightPadding);
};
```

### Usage:
```tsx
<WorkUnitTitle>
  {selectedWorkUnit
    ? <Text>{workUnit.id}: {workUnit.title}</Text>
    : <Text>{centerText('No work unit selected', totalWidth)}</Text>
  }
</WorkUnitTitle>
```

### Visual Result:
```
│         No work unit selected          │  ← Centered
```

---

## Rule [43]: String injection pattern at lines 504-616 MUST BE DELETED

### Lines to Delete (504-616):
```typescript
// Line 504-526: Git Stashes panel string injection
rows.push('│' + fitToWidth(`Git Stashes (${stashes.length})`, totalWidth) + '│');

// Line 531-616: Work Unit Details string injection
rows.push('│' + fitToWidth(titleLine, totalWidth) + '│');
rows.push('│' + chalk.cyan.bold(fitToWidth(wrappedLines[0], totalWidth)) + '│');
// ... ALL OF THIS MUST GO
```

### Replace With Components:
```tsx
<Box borderLeft borderRight height={5}>
  <WorkUnitTitle />
  <WorkUnitDescription />
  <WorkUnitMetadata />
</Box>
```

---

## Rule [44]: Hybrid rendering at lines 707-740 must be fully component-based

### Current Hybrid (lines 707-740):
```tsx
return (
  <Box>
    {/* Component-based header */}
    <Box borderStyle="single">
      <Logo />
      <CheckpointPanel />
    </Box>

    {/* String-based separator */}
    <Text>{separatorAfterGit}</Text>

    {/* String-based rest */}
    {restOfRows.map(row => <Text>{row}</Text>)}
  </Box>
);
```

### Must Become Fully Component-Based:
```tsx
return (
  <Box>
    <HeaderContainer />
    <Separator type="plain" />
    <WorkUnitDetailsContainer />
    <Separator type="top" />
    <ColumnContainer />
  </Box>
);
```

---

## Rule [45]: Current implementation rows array breakdown

### Rows Array Structure:
```typescript
rows[0-4]:   Header strings (lines 499-527)     ← DELETE
rows[5]:     Separator after git (line 529)     ← KEEP as Text component
rows[6-11]:  Details strings (lines 531-616)    ← DELETE
rows[12]:    Separator before columns (line 619) ← KEEP as Text component
rows[13+]:   Column rendering (lines 622-705)   ← KEEP
```

---

## Rule [46]: Separators must match column widths exactly

### Calculation:
```typescript
const totalWidth = STATES.reduce((sum, _, idx) =>
  sum + getColumnWidth(idx, colWidth, colRemainder), 0
) + (STATES.length - 1);  // +1 for each separator between columns

buildBorderRow(colWidth, colRemainder, '├', '─', '┤', 'plain');
```

### Visual Alignment:
```
├───────┬───────┬───────┤  ← Separator width matches
│ Col 1 │ Col 2 │ Col 3 │  ← Column widths exactly
├───────┼───────┼───────┤  ← Perfect alignment
```

---

## Rule [47]: WorkUnitDetailsContainer width must match totalWidth

### Calculation:
```typescript
const totalWidth = STATES.reduce((sum, _, idx) =>
  sum + getColumnWidth(idx, colWidth, colRemainder), 0
) + (STATES.length - 1);
```

### Component:
```tsx
<Box width={totalWidth} borderLeft borderRight>
  {/* Details content */}
</Box>
```

### Visual Alignment (Single Continuous Structure):
```
├────────────────────────────────────┤  ← Junction separator
│ TUI-015: Title                     │  ← Details (same width)
│ Description                        │
├─┬──────┬──────┬──────┬──────┬─────┤  ← Junction to columns (same width)
```

---

## Rule [48]: WorkUnitDescription empty/whitespace displays 3 blank lines

### Input:
```typescript
description = "" // or "   " or "\n\n"
```

### Visual Output:
```
│ TUI-015: Feature Title              │  ← Line 1: Title
│                                      │  ← Line 2: Empty
│                                      │  ← Line 3: Empty
│                                      │  ← Line 4: Empty
│ Epic: auth | Estimate: 5pts         │  ← Line 5: Metadata
```

**NO "No description" text shown**

---

## Rule [49]: WorkUnitMetadata fields joined with ' | ' ONLY between present fields

### All Fields Present:
```typescript
epic = "auth"
estimate = 5
status = "implementing"

Result: "Epic: auth | Estimate: 5pts | Status: implementing"
```

### Some Fields Missing:
```typescript
epic = undefined
estimate = 3
status = undefined

Result: "Estimate: 3pts"  // No leading/trailing ' | '
```

### Visual:
```
│ Epic: auth | Estimate: 5pts | Status: implementing │  ← All fields
│ Estimate: 3pts                                     │  ← One field
│ Epic: authentication | Status: done                │  ← Two fields
```

---

## Rule [50]: CheckpointPanel replaced with simpler CheckpointStatus

### Current CheckpointPanel (105 lines):
- File watching with chokidar
- JSON parsing
- Complex state management

### New CheckpointStatus (simple):
```tsx
interface CheckpointStatusProps {
  manualCount: number;
  autoCount: number;
}

const CheckpointStatus: React.FC<CheckpointStatusProps> = ({ manualCount, autoCount }) => {
  const text = manualCount === 0 && autoCount === 0
    ? 'Checkpoints: None'
    : `Checkpoints: ${manualCount} Manual, ${autoCount} Auto`;

  return (
    <Box flexGrow={1}>
      <Text>{text}</Text>
    </Box>
  );
};
```

**Data fetching happens at parent level, component just displays**

---

## Rule [51]: ChangedFilesPanel renamed to KeybindingShortcuts with borderTop

### Current ChangedFilesPanel:
```tsx
// ChangedFilesPanel.tsx (lines 1-19)
export const ChangedFilesPanel: React.FC = () => {
  return (
    <Box flexDirection="column">
      <Text>C View Checkpoints ◆ F View Changed Files</Text>
    </Box>
  );
};
```

### Rename to KeybindingShortcuts with borderTop:
```tsx
// KeybindingShortcuts.tsx
export const KeybindingShortcuts: React.FC = () => {
  return (
    <Box
      flexGrow={1}
      borderTop={true}
      borderBottom={false}
      borderLeft={false}
      borderRight={false}
    >
      <Text>C View Checkpoints ◆ F View Changed Files</Text>
    </Box>
  );
};
```

### Visual Change (within InfoContainer):
```
Before:
│ Checkpoints text                        │
│ C View Checkpoints ◆ F View Changed... │  (no border)

After:
│ Checkpoints text                        │
├─────────────────────────────────────────┤  ← borderTop added (internal)
│ C View Checkpoints ◆ F View Changed... │
```

---

## Summary Visual: Complete Layout Structure

**Single Continuous Border System with Junctions**

```
┌─────────────────────────────────────────────────────────────┐ ← Row 1
│ ┏┓┏┓┏┓┏┓┏┓  Checkpoints: 3 Manual, 5 Auto                  │ ← Row 2
│ ┣ ┗┓┃┃┣ ┃  ───────────────────────────────────────────────  │ ← Row 3
│ ┻ ┗┛┣┛┗┛┗┛  C View Checkpoints ◆ F View Changed Files      │ ← Row 4
│             ^ borderTop here                                 │ ← Row 5
├─────────────────────────────────────────────────────────────┤ ← Row 6 (junction)
│ TUI-015: Refactor header and details to component-based     │ ← Row 7
│ [CYAN BOLD] NO string injection - all content must be       │ ← Row 8
│ [CYAN BOLD] rendered as Ink components instead of manual    │ ← Row 9
│ [CYAN BOLD] string concatenation in rows array...           │ ← Row 10
│ Epic: tui | Estimate: 8pts | Status: specifying             │ ← Row 11
├─┬───────┬─────────┬─────────┬────────────┬──────────┬──────┤ ← Row 12 (junction)
│ │BACKLOG│SPECIFYIN│TESTING  │IMPLEMENTIN │VALIDATING│DONE  │ ← Row 13
├─┼───────┼─────────┼─────────┼────────────┼──────────┼──────┤ ← Row 14 (cross)
│ │TUI-015│         │         │            │          │      │ ← Row 15
│ │       │         │         │            │          │      │ ← Row 16
│ │       │         │         │            │          │      │ ← Row 17
│ │       │         │         │            │          │      │ ← Row 18
│ │       │         │         │            │          │      │ ← Row 19
│ │       │         │         │            │          │      │ ← Row 20
├─┴───────┴─────────┴─────────┴────────────┴──────────┴──────┤ ← Row 21 (junction)
│ ← → Columns ◆ ↑↓ Work Units ◆ ↵ Details ◆ ESC Back         │ ← Row 22
└─────────────────────────────────────────────────────────────┘ ← Row 23

Static rows: 17 (rows 1-14 + 21-23)
Dynamic rows: 6 (rows 15-20, grows with terminal)
```

**Key Junction Characters:**
- `├` and `┤` - Left/right junctions extending vertical borders
- `┬` - Top junction where columns begin
- `┼` - Cross junction at header separator
- `┴` - Bottom junction where columns end

**Note: This is ONE continuous border structure, NOT separate boxes stacked together**

---

## End of Visual Rules Guide
