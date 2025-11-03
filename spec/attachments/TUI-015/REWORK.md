# TUI Header and Details Refactoring

## Overview
Refactor UnifiedBoardLayout header and work unit details sections to use pure component-based flexbox layout with NO string injection.

## Critical Requirements

### 1. Component-Based Architecture
- **EVERYTHING** must have its own component
- **NO string injection** for layout
- Only use flexbox, flex grow, flex shrink
- Use static heights **only** where absolutely necessary
- **NEVER** use percentages

### 2. Header Section (Logo + Info Area)

#### Layout Structure
```
┌────────────────────────────────────────┐
│ Logo    Checkpoints: X Manual, Y Auto  │
│ (12ch)  C View ◆ F View (flex grow)   │
│         ────────────────── (borderTop) │
│                                        │
└────────────────────────────────────────┘
```

#### Components
1. **HeaderContainer**
   - Flexbox row layout
   - Height: 4 lines (static)
   - Border: single (all sides)

2. **Logo** (left side)
   - Width: 12 characters (fixed)
   - Flex shrink: yes
   - Height: 4 lines
   - Content: ASCII logo

3. **InfoContainer** (right side)
   - Flex grow: yes
   - Flexbox column layout
   - Height: 4 lines
   - Contains:
     - CheckpointStatus component
     - KeybindingShortcuts component

4. **CheckpointStatus**
   - Height: dynamic (flex grow)
   - Displays: "Checkpoints: X Manual, Y Auto" or "Checkpoints: None"
   - No borders

5. **KeybindingShortcuts**
   - Height: dynamic (flex grow)
   - Displays: "C View Checkpoints ◆ F View Changed Files"
   - Border: top only (borderTop: 'single', all others: false)

### 3. Work Unit Details Section

#### Layout Structure
```
├────────────────────────────────────────┤
│ WORK-001: Feature Title                │
│ Description line 1 text wraps here and │
│ continues on line 2 and can extend to  │
│ line 3 with truncation...              │
│ Epic: epic-name | Estimate: 5pts       │
├────────────────────────────────────────┤
```

#### Components
1. **WorkUnitDetailsContainer**
   - Flexbox column layout
   - Height: 5 lines (static)
   - Border: left and right only
   - Top/bottom borders rendered by separators

2. **WorkUnitTitle**
   - Height: 1 line (static)
   - Displays: "{id}: {title}"
   - Text truncates if exceeds width
   - No background padding

3. **WorkUnitDescription**
   - Height: 3 lines (dynamic via flex grow)
   - Wraps text to fit width
   - Maximum 3 lines with "..." truncation if longer
   - Bold cyan color
   - Empty if no description

4. **WorkUnitMetadata**
   - Height: 1 line (static)
   - Displays: "Epic: {epic} | Estimate: {estimate}pts | Status: {status}"
   - Shows only available metadata
   - No background padding

### 4. Border System Requirements

#### Hybrid Border Connection
All borders must use the hybrid border system where corners connect properly:

```
Box (borderStyle: single)
├── separator (├─...─┤)
│   WorkUnitDetails (borderLeft/Right only)
│   ...
├── separator (├─┬─┤) with column dividers
```

#### Border Characters
- Top border: `┌─...─┐`
- Side borders: `│`
- Separators: `├─...─┤` (plain) or `├─┬─┤` (with column dividers)
- Bottom border: `└─...─┘`

### 5. Height Calculations

#### Header Section
- Total height: 6 lines
  - Border top: 1 line (Box border)
  - Content: 4 lines (Logo + Info)
  - Border bottom: 1 line (Box border)

#### Work Unit Details Section
- Total height: 5 lines
  - Title: 1 line
  - Description: 3 lines
  - Metadata: 1 line
- Borders rendered via separators (not counted in content height)

#### Fixed Rows Calculation
```
- Header Box: 6 lines (1 top border + 4 content + 1 bottom border)
- Separator after header: 1 line
- Work Unit Details: 5 lines
- Separator before columns: 1 line
- Column headers: 1 line
- Header separator: 1 line
- Footer separator: 1 line
- Footer text: 1 line
- Bottom border: 1 line
Total: 19 fixed rows
```

### 6. No String Injection Examples

#### ❌ WRONG (String Injection)
```tsx
rows.push('│ ┏┓┏┓┏┓┏┓┏┓  ' + checkpointsText + '│');
```

#### ✅ CORRECT (Component-Based)
```tsx
<Box borderStyle="single">
  <Box flexDirection="row">
    <Logo />
    <InfoContainer>
      <CheckpointStatus />
      <KeybindingShortcuts />
    </InfoContainer>
  </Box>
</Box>
```

### 7. Flexbox Rules

#### Use Flex Grow
For components that should expand to fill available space:
```tsx
<Box flexGrow={1}>
  <Text>Content expands</Text>
</Box>
```

#### Use Flex Shrink
For components with fixed size that can shrink if needed:
```tsx
<Box width={12} flexShrink={1}>
  <Logo />
</Box>
```

#### Use Static Height
Only when component must be exact height:
```tsx
<Box height={1}>
  <Text>Single line</Text>
</Box>
```

## Implementation Checklist

- [ ] Create HeaderContainer component
- [ ] Create InfoContainer component
- [ ] Create CheckpointStatus component
- [ ] Create KeybindingShortcuts component (with borderTop)
- [ ] Create WorkUnitDetailsContainer component
- [ ] Create WorkUnitTitle component (height: 1)
- [ ] Create WorkUnitDescription component (height: 3, flex grow)
- [ ] Create WorkUnitMetadata component (height: 1)
- [ ] Remove all string injection from rows array for header/details
- [ ] Update fixedRows calculation to 19
- [ ] Test border connections (corners align properly)
- [ ] Test with various terminal sizes
- [ ] Test with/without work unit selected
- [ ] Test with/without description
- [ ] Test text wrapping and truncation

## Testing Requirements

### Visual Tests
1. Borders connect at all corners (┌├└┬┼┴┐┤┘)
2. Header is exactly 6 lines tall
3. Work unit details is exactly 5 lines tall
4. Logo is exactly 12 characters wide
5. Info container expands to fill remaining width
6. Description wraps and truncates at 3 lines
7. Title truncates if too long
8. Metadata shows all available fields separated by " | "

### Functional Tests
1. Checkpoint counts update dynamically
2. Work unit title displays correctly
3. Description wraps properly
4. Metadata shows Epic/Estimate/Status
5. Empty description shows blank lines (not "No description")
6. No work unit selected shows centered message in details

## Edge Cases

1. **Very narrow terminal**: Components should not break, use flex shrink
2. **No checkpoints**: Display "Checkpoints: None"
3. **No work unit selected**: Show "No work unit selected" centered in line 1, lines 2-5 empty
4. **No description**: Lines 2-4 empty in work unit details
5. **Very long title**: Truncate with no "..."
6. **Very long description**: Truncate line 3 with "..."
7. **Missing metadata fields**: Show only available fields with " | " separator

## Anti-Patterns to Avoid

### ❌ DO NOT DO
1. Using percentages: `width="50%"`
2. String injection: `rows.push('│' + content + '│')`
3. Hardcoded widths except for Logo (12ch)
4. Using Box borderStyle for work unit details (use separators)
5. Rendering content as strings in rows array

### ✅ DO THIS
1. Use flexGrow/flexShrink: `<Box flexGrow={1}>`
2. Use components: `<CheckpointStatus />`
3. Calculate widths from terminal: `terminalWidth - logoWidth`
4. Use separators for connecting borders
5. Render all content as Ink components
