@high
@tui
@interactive-cli
@ink
@typescript
@cli
@feature-management
@git
@BOARD-003
Feature: Real-time board updates with git stash and file inspection
  """
  CRITICAL CONSTRAINTS:
  1. NEVER use git CLI commands - ALL git operations MUST use isomorphic-git library
  2. Reuse existing utilities: getStagedFiles/getUnstagedFiles from src/git/status.ts
  3. Stash listing via git.log() with ref='refs/stash'
  4. Diff generation via git.readBlob() + manual diff algorithm (NOT git diff CLI)
  5. File watching via Node.js fs.watch (built-in, zero external dependencies)
  6. DRY principle: Reuse existing fspec store loadData() function for refreshing

  Architecture:
  - BoardView component uses React useEffect to setup fs.watch on spec/work-units.json
  - On file change event, call existing fspecStore.loadData() to reload from disk
  - loadData() already exists in src/tui/store/fspecStore.ts (DRY - don't duplicate)
  - Zustand store update triggers automatic React re-render (no manual rerender needed)
  - Cleanup: useEffect returns cleanup function to close watcher on unmount
  - Error handling: Ignore ENOENT errors if file doesn't exist yet
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Board must display git stashes with timestamps and message previews
  #   2. Board must show changed files with staged/unstaged indicators (green/yellow)
  #   3. User can inspect stash details by selecting with Enter key
  #   4. User can view file diffs for changed files by selecting with Enter key
  #   5. Board refreshes automatically when work unit status changes (via fs.watch + loadData)
  #   6. Keyboard navigation: tab/shift-tab to switch between board/stash/files panels
  #   7. ESC key returns from detail view to board view
  #   8. CRITICAL: NEVER use git CLI commands (git status, git diff, git stash) - ONLY use isomorphic-git library
  #   9. Use existing utilities: getStagedFiles(), getUnstagedFiles() from src/git/status.ts
  #   10. Stash operations use git.log() with ref 'refs/stash' to list stashes via isomorphic-git
  #   11. File diffs generated using git.readBlob() and manual diff logic (NOT git diff CLI)
  #   12. CRITICAL: File watching uses Node.js fs.watch (built-in) - NO external libraries (no chokidar, nodemon, etc)
  #   13. DRY: MUST reuse existing fspecStore.loadData() function - DO NOT duplicate file reading logic
  #   14. Tests verify file watching by writing to spec/work-units.json and checking store updates
  #   15. Tests MUST clean up by restoring original file content to avoid side effects
  #
  # EXAMPLES:
  #   1. User opens board with 2 stashes: shows 'GIT-001-auto-testing (2 hours ago)' and 'baseline (3 days ago)'
  #   2. Board shows 3 staged files (green) and 2 unstaged files (yellow) in files panel
  #   3. User presses Enter on stash: detail view shows stash message, timestamp, and changed files
  #   4. User presses Enter on src/auth.ts: diff view shows +5 -2 lines with syntax highlighting
  #   5. File watcher detects change: test writes to spec/work-units.json → fs.watch fires → loadData() called → board refreshes
  #   6. User presses tab: focus switches from board to stash panel, border changes to cyan
  #   7. User presses ESC in diff view: returns to board view with previous focus restored
  #   8. Get staged files: import { getStagedFiles } from 'src/git/status'; const staged = await getStagedFiles(cwd); // ['src/auth.ts', 'README.md']
  #   9. Get unstaged files: import { getUnstagedFiles } from 'src/git/status'; const unstaged = await getUnstagedFiles(cwd); // ['src/utils.ts']
  #   10. List stashes: const logs = await git.log({ fs, dir: cwd, ref: 'refs/stash', depth: 10 }); // Returns stash commits
  #   11. Read file from stash: const { blob } = await git.readBlob({ fs, dir: cwd, oid: stashOid, filepath: 'src/auth.ts' }); // Get file content
  #   12. Generate diff: const headBlob = await git.readBlob({ fs, dir: cwd, oid: 'HEAD', filepath }); const workdirContent = await fs.readFile(filepath); const diff = computeDiff(headBlob, workdirContent);
  #   13. Setup file watcher: const watcher = fs.watch('spec/work-units.json', () => { void loadData(); }); // Watch for changes
  #   14. Cleanup watcher: useEffect(() => { const watcher = fs.watch(...); return () => watcher.close(); }, []); // Cleanup on unmount
  #   15. Error handling: watcher.on('error', (err) => { if (err.code !== 'ENOENT') console.error(err); }); // Ignore missing file
  #   16. DRY reload: loadData() is ALREADY defined in fspecStore - just call it, don't reimplement file reading
  #   17. Test pattern: Read file → Modify work unit → Write file → Wait for fs.watch → Verify store updated
  #   18. Test writes real file: await fs.writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2)); // Trigger fs.watch
  #   19. Test cleanup: Save original content → Do test → Restore original content (prevent side effects)
  #   20. Test verification: Store state should reflect file changes after fs.watch triggers loadData()
  #
  # ========================================
  Background: User Story
    As a developer managing work units
    I want to see real-time status of work in progress
    So that I can quickly inspect changes and stash state without leaving the terminal

  Scenario: Display git stashes with timestamps
    Given the board is displaying
    And there are 2 git stashes: "GIT-001-auto-testing" and "baseline"
    When viewing the stash panel
    Then it should display "GIT-001-auto-testing (2 hours ago)"
    And it should display "baseline (3 days ago)"

  Scenario: Display changed files with staged/unstaged indicators
    Given the board is displaying
    And there are 3 staged files
    And there are 2 unstaged files
    When viewing the files panel
    Then staged files should be displayed with green indicator
    And unstaged files should be displayed with yellow indicator
    And the panel should show "3 staged, 2 unstaged"

  Scenario: Inspect stash details with Enter key
    Given the board is displaying with stash panel focused
    And "GIT-001-auto-testing" stash is selected
    When the user presses the Enter key
    Then the stash detail view should open
    And it should display the stash message
    And it should display the stash timestamp
    And it should list all changed files in the stash

  Scenario: View file diff with Enter key
    Given the board is displaying with files panel focused
    And "src/auth.ts" file is selected
    When the user presses the Enter key
    Then the diff view should open
    And it should display "+5 -2 lines"
    And it should show syntax-highlighted diff content

  Scenario: Auto-refresh board on status change
    Given the board is displaying
    And work unit BOARD-TEST-001 is in "implementing" state in spec/work-units.json
    When spec/work-units.json is updated to change status to "validating"
    Then fs.watch should trigger the change event
    And the board should automatically call loadData() to refresh
    And BOARD-TEST-001 should move to the validating column

  Scenario: Switch focus between panels with tab key
    Given the board is displaying with board panel focused
    When the user presses the tab key
    Then focus should switch to stash panel
    And the stash panel border should change to cyan
    And the board panel border should change to gray

  Scenario: Return from detail view with ESC key
    Given the diff view is open for "src/auth.ts"
    And the board view was previously focused on files panel
    When the user presses the ESC key
    Then the diff view should close
    And the board view should be restored
    And focus should return to files panel
