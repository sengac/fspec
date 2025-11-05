@done
@diff-viewer
@tui
@git-integration
@bug-fix
@high
@TUI-028
Feature: Changed files view missing unstaged files
  """
  This bug fix modifies the getUnstagedFilesWithChangeType() function in src/git/status.ts to include untracked files (new files not yet added to git). Currently, the function explicitly filters out untracked files with the condition 'hasUnstagedChanges && \!isUntracked'. The fix will remove this exclusion so that both unstaged modifications and untracked files appear in the changed files view. The ChangedFilesViewer component in src/tui/components/ChangedFilesViewer.tsx will automatically display these files once the data model includes them. Implementation involves modifying the filter logic in getUnstagedFilesWithChangeType() to include files where isUntracked is true, while maintaining proper change type indicators (A for added/untracked, M for modified, D for deleted).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Changed files view must show staged files (files added with git add)
  #   2. Changed files view must show unstaged modifications to tracked files
  #   3. Changed files view must show untracked files (new files not yet added to git)
  #   4. Changed files view must show deleted files with proper status indicators
  #
  # EXAMPLES:
  #   1. User creates new file newfile.txt (untracked), opens changed files view with F key, sees newfile.txt listed with 'A' indicator in green
  #   2. User modifies existing file src/index.ts (unstaged), opens changed files view, sees src/index.ts with 'M' indicator in yellow
  #   3. User stages file with git add README.md, opens changed files view, sees README.md under staged section with appropriate indicator
  #   4. User deletes file oldfile.ts (unstaged deletion), opens changed files view, sees oldfile.ts with 'D' indicator in red
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should untracked files appear in a separate 'Untracked' section, or should they be mixed with unstaged files?
  #   A: true
  #
  #   Q: The current code excludes untracked files from the unstaged list. Should we combine them or keep them separate in the data model?
  #   A: true
  #
  #   Q: Should the files be sorted in any particular order (alphabetical, by status, by type)?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to see all changed files including unstaged and untracked files
    So that I have a complete view of my working directory changes

  Scenario: View untracked file in changed files view
    Given I have created a new file "newfile.txt" that is untracked
    When I open the changed files view with the F key
    Then I should see "newfile.txt" listed in the unstaged section
    And the status indicator should be "A" in green color

  Scenario: View unstaged modification in changed files view
    Given I have modified an existing tracked file "src/index.ts"
    When I open the changed files view with the F key
    Then I should see "src/index.ts" listed in the unstaged section
    And the file has not been staged
    And the status indicator should be "M" in yellow color

  Scenario: View staged file in changed files view
    Given I have staged a file "README.md" using "git add README.md"
    When I open the changed files view with the F key
    Then I should see "README.md" listed in the staged section
    And the file should have an appropriate status indicator based on its change type

  Scenario: View deleted file in changed files view
    Given I have deleted an existing tracked file "oldfile.ts"
    When I open the changed files view with the F key
    Then I should see "oldfile.ts" listed in the unstaged section
    And the deletion has not been staged
    And the status indicator should be "D" in red color
