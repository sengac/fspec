# Remaining Test Failures Analysis

## Summary
7 test failures remain after completing BUG-055 (EXMAP-001 refactor fixes). All failures are TUI component tests for unimplemented BOARD-003 features.

**Test Status:**
- Total: 1885 tests
- Passing: 1878 tests (99.6%)
- Failing: 7 tests (0.4%)
- Files affected: 4 test files

## Detailed Failure Analysis

### 1. BoardView-realtime-updates.test.tsx (2 failures)

**Feature:** Real-time board updates with git stash and file inspection (BOARD-003)

#### Failure 1: Inspect stash details with Enter key
**Test:** `should open stash detail view using git.readBlob`
**Location:** `src/tui/__tests__/BoardView-realtime-updates.test.tsx:149`

**Error:**
```
AssertionError: expected '┌────────────────────────────────────…' to contain 'GIT-001-auto-testing'
```

**Expected Behavior:**
- When checkpoint panel is focused
- And user presses Enter key
- Then stash detail view should open
- And display stash message "GIT-001-auto-testing"
- And display changed files (src/auth.ts, README.md)

**Actual Behavior:**
- Enter key is not handled in BoardView for checkpoint panel
- No detail view opens
- Still showing main board view

**Root Cause:**
- BoardView component doesn't implement Enter key handler for `focusedPanel === 'stash'`
- No state for showing stash detail view
- No rendering of stash detail content

**Fix Required:**
1. Add Enter key handler in BoardView.tsx when focusedPanel === 'stash'
2. Add state for selected stash index
3. Create stash detail view mode showing:
   - Stash message
   - Timestamp
   - Changed files list (using git.listFiles)
4. Handle ESC to return to board

#### Failure 2: View file diff with Enter key
**Test:** `should generate diff using git.readBlob (NOT git diff CLI)`
**Location:** `src/tui/__tests__/BoardView-realtime-updates.test.tsx:183`

**Error:**
```
AssertionError: expected '┌────────────────────────────────────…' to match /\+.*-.*lines/i
```

**Expected Behavior:**
- When files panel is focused
- And user presses Enter key
- Then diff view should open
- And display "+5 -2 lines" stats
- And show syntax-highlighted diff content

**Actual Behavior:**
- Enter key not handled for files panel
- No diff view opens
- Still showing main board

**Root Cause:**
- BoardView doesn't implement Enter key handler for `focusedPanel === 'files'`
- No diff generation using git.readBlob
- No diff view rendering

**Fix Required:**
1. Add Enter key handler when focusedPanel === 'files'
2. Add state for selected file index
3. Generate diff using git.readBlob (HEAD vs working tree)
4. Create diff view mode showing:
   - File path
   - "+X -Y lines" summary
   - Syntax-highlighted diff
5. Handle ESC to return to board

---

### 2. UnifiedBoardLayout-work-unit-details.test.tsx (1 failure)

**Feature:** Fix work unit details panel to be static 4 lines high

#### Failure: Work unit with description displays all 4 content lines
**Test:** `should show exactly 4 content lines with ID, description, metadata, and empty line`
**Location:** `src/tui/__tests__/UnifiedBoardLayout-work-unit-details.test.tsx`

**Error:**
Panel is not showing exactly 4 lines as required

**Expected Behavior:**
- Work unit details panel should be EXACTLY 4 lines high
- Line 1: Work unit ID and title
- Line 2: Description (truncated if needed)
- Line 3: Metadata (status, dates)
- Line 4: Empty line (spacing)

**Actual Behavior:**
- Panel height varies based on content
- Not consistently 4 lines

**Root Cause:**
- UnifiedBoardLayout doesn't enforce static 4-line height for details panel
- Layout calculation doesn't reserve exactly 4 lines

**Fix Required:**
1. Update UnifiedBoardLayout to enforce static 4-line work unit details panel
2. Truncate description to fit within line 2
3. Ensure metadata fits on line 3
4. Always render empty line 4 for spacing

---

### 3. ChangedFilesViewer-diff-loading.test.tsx (3 failures)

**Feature:** Diff loading system has rendering bugs and performance issues

#### Failure 1: Infinite re-render flickering
**Test:** `should NOT call getFileDiff multiple times for the same file selection`
**Location:** `src/tui/components/__tests__/ChangedFilesViewer-diff-loading.test.tsx`

**Error:**
getFileDiff is being called multiple times for the same file

**Expected Behavior:**
- When file is selected
- getFileDiff should be called ONCE
- Subsequent renders should NOT re-call getFileDiff

**Actual Behavior:**
- getFileDiff called on every render
- Causes infinite re-render loop
- Flickering display

**Root Cause:**
- Object reference instability in useEffect dependencies
- New object created on every render triggers effect
- Missing memoization

**Fix Required:**
1. Memoize diff loading result
2. Stabilize object references in useEffect deps
3. Use useCallback for getFileDiff
4. Prevent re-calling for same file selection

#### Failure 2: Race condition causes wrong file diff
**Test:** `should cancel previous diff load when selection changes quickly`

**Error:**
Previous diff load not cancelled when selection changes

**Expected Behavior:**
- When selection changes rapidly
- Previous diff load should be cancelled
- Only latest selection's diff should display

**Actual Behavior:**
- Multiple diff loads execute concurrently
- Race condition causes wrong diff to display

**Root Cause:**
- No cancellation mechanism in diff loading
- useEffect doesn't cleanup previous async operations

**Fix Required:**
1. Add AbortController to diff loading
2. Cancel previous load in useEffect cleanup
3. Ignore stale results if selection changed

#### Failure 3: Fixed dependency management
**Test:** `should only trigger diff load when selectedFileIndex changes`

**Error:**
Diff load triggered by other dependency changes

**Expected Behavior:**
- Diff should ONLY load when selectedFileIndex changes
- Other state changes should NOT trigger reload

**Actual Behavior:**
- Diff reloads on unrelated state changes
- Excessive re-rendering

**Root Cause:**
- useEffect dependencies include unstable references
- Over-broad dependency array

**Fix Required:**
1. Narrow useEffect dependencies to ONLY selectedFileIndex
2. Extract stable refs for other values
3. Use useRef for values that shouldn't trigger reload

---

### 4. CheckpointViewer.test.tsx (1 failure)

**Feature:** Arrow Key Navigation for Component Selection

#### Failure: Up/down arrow keys navigate within panes
**Test:** `should navigate items within focused pane without changing pane focus`
**Location:** `src/tui/components/__tests__/CheckpointViewer.test.tsx`

**Error:**
Arrow keys change pane focus instead of navigating within pane

**Expected Behavior:**
- When pane is focused
- Up/down arrows navigate items WITHIN that pane
- Pane focus should NOT change

**Actual Behavior:**
- Arrow keys change which pane is focused
- Items within pane don't navigate

**Root Cause:**
- CheckpointViewer doesn't distinguish between:
  - Tab/Shift+Tab (change pane focus)
  - Up/Down (navigate within pane)
- Missing item navigation logic

**Fix Required:**
1. Add separate handlers for:
   - Tab/Shift+Tab: Change pane focus
   - Up/Down: Navigate items within focused pane
2. Track selected item index per pane
3. Implement item highlighting
4. Ensure arrow keys DON'T change pane focus

---

## Implementation Plan

### Phase 1: BoardView Enter Key Handlers (2 tests)
**Files to modify:**
- `src/tui/components/BoardView.tsx`

**Tasks:**
1. Add state for stash detail view mode
2. Add state for file diff view mode
3. Implement Enter key handlers for stash/files panels
4. Create stash detail rendering (message, files list)
5. Create file diff rendering (using git.readBlob)
6. Add ESC handlers to return to board

**Estimated complexity:** Medium (requires git integration)

---

### Phase 2: UnifiedBoardLayout Static Panel (1 test)
**Files to modify:**
- `src/tui/components/UnifiedBoardLayout.tsx`

**Tasks:**
1. Calculate static 4-line height for details panel
2. Implement line truncation for description
3. Ensure metadata fits on single line
4. Always render 4 lines regardless of content

**Estimated complexity:** Low (pure layout fix)

---

### Phase 3: ChangedFilesViewer Optimizations (3 tests)
**Files to modify:**
- `src/tui/components/ChangedFilesViewer.tsx`

**Tasks:**
1. Add useMemo for diff result
2. Add useCallback for getFileDiff
3. Implement AbortController for cancellation
4. Cleanup previous async operations
5. Stabilize dependency array (only selectedFileIndex)
6. Extract stable refs for non-triggering values

**Estimated complexity:** Medium (requires React optimization patterns)

---

### Phase 4: CheckpointViewer Navigation (1 test)
**Files to modify:**
- `src/tui/components/CheckpointViewer.tsx`

**Tasks:**
1. Add state for selected item index per pane
2. Separate Tab/Shift+Tab from Up/Down handlers
3. Implement item navigation within focused pane
4. Add item highlighting
5. Prevent arrow keys from changing pane focus

**Estimated complexity:** Low (input handling)

---

## Testing Strategy

After each phase:
1. Run affected test file: `npm run test -- <test-file>`
2. Verify failures decrease
3. Run full suite: `npm run test`
4. Ensure no regressions

**Success criteria:** All 1885 tests passing (100%)

---

## Related Work Units

- **BOARD-003:** Real-time board updates with git stash and file inspection
- **TUI-010:** Mouse tracking for board view
- **LOCK-002:** File locking for concurrent access safety

---

## Notes

These tests are "red phase" ACDD tests written BEFORE implementation. They are intentionally failing until features are implemented. All tests are well-written with clear acceptance criteria - implementation should be straightforward following test expectations.

**Key insight:** Tests clearly document expected behavior. Read tests carefully before implementing to avoid misunderstanding requirements.
