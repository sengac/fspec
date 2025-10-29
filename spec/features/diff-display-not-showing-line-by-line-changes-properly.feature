@tui
@visualization
@critical
@git
@diff-viewer
@bug-fix
@BUG-050
Feature: Diff display not showing line-by-line changes properly

  """
  Architecture notes:
  - Root cause: generateUnifiedDiff() in src/git/diff.ts uses naive line-by-line comparison (lines 87-120)
  - Fix: Replace naive algorithm with Myers algorithm from 'diff' library (already imported in diff-parser.ts)
  - The 'diff' library's diffLines() function properly aligns lines and detects insertions, deletions, and unchanged blocks
  - This will fix incorrect diffs when lines are reordered or inserted/deleted in the middle of files
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Diff viewer must show only the lines that actually changed, not entire blocks
  #   2. When comparing package.json with structural changes, show line-by-line differences not block-level differences
  #   3. Context lines (unchanged) should be clearly separated from changed lines
  #
  # EXAMPLES:
  #   1. User changes 'typescript': '^5.9.2' to 'vite': '^7.1.5' in package.json devDependencies. Diff should show ONLY the 2 changed lines (old removed, new added), not the entire devDependencies block
  #   2. Currently showing: many lines with '-' prefix (prettier, typescript, vite, vitest, lint-staged) followed by many lines with '+' prefix (execa, immer, ink, etc.). This makes it look like all lines changed when only 1 line did
  #   3. Expected: Show only the actual line that changed with proper context (e.g., 3 lines before and after)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we use a proper diff algorithm (like Myers algorithm from 'diff' library) to replace the naive line-by-line comparison in generateUnifiedDiff()?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer reviewing code changes
    I want to see only the lines that actually changed in diffs
    So that I can quickly understand what was modified without being confused by unrelated lines

  Scenario: Diff correctly shows single line change with context
    Given I have a file with one line changed in the middle
    When I view the unified diff
    Then I should see the removed line with '-' prefix
    And I should see the added line with '+' prefix
    And I should see context lines (3 before and after)
    And I should NOT see unrelated lines from other parts of the file

  Scenario: Diff does not treat reordered lines as complete block replacement
    Given I have package.json with dependencies in different order
    When I view the unified diff
    Then I should see only the lines that were actually added or removed
    And I should NOT see all dependencies marked as removed then all marked as added
    And unchanged dependencies should appear as context lines

  Scenario: Myers algorithm properly aligns lines for accurate diff
    Given I have a file where a line was replaced in the middle
    When the diff is generated using Myers algorithm
    Then the algorithm should identify the minimum set of changes
    And should show only the replaced lines, not entire blocks
    And should preserve correct line numbers in hunk headers
