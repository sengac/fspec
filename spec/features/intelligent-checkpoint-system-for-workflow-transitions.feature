@done
@cli
@workflow-automation
@checkpoint
@git
@phase2
@GIT-002
Feature: Intelligent checkpoint system for workflow transitions
  """
  Git stash message format: fspec-checkpoint:{work-unit-id}:{checkpoint-name}:{timestamp}
  Use 'git stash push -u -m message' to include untracked files respecting .gitignore
  Use 'git stash apply stash@{N}' for restoration (preserves stash for re-restoration)
  Conflict detection: parse git output for CONFLICT markers and identify affected files
  AI conflict resolution: emit system-reminder with conflicted files, AI uses Read/Edit to resolve
  Test validation: automatically run 'npm test' after conflict resolution, block completion if tests fail
  Leverage existing isomorphic-git integration for git operations
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Checkpoints are automatically created when work unit status changes (before leaving current state)
  #   2. Users can create named checkpoints manually at any time for experiments
  #   3. Checkpoints capture all file changes including untracked files (respecting .gitignore)
  #   4. Automatic checkpoints use pattern '{work-unit-id}-auto-{state}', manual use user-provided names
  #   5. Checkpoints persist until explicitly deleted (no automatic expiration)
  #   6. Checkpoints stored as git stashes with special message format for filtering
  #   7. When restoration causes conflicts, AI receives system-reminder to resolve using Read/Edit tools
  #   8. All tests must run and pass after checkpoint restoration with conflicts
  #   9. Show all checkpoints by default with clear visual indicators (emoji like 🤖 for auto, 📌 for manual) so users feel confident in the safety net
  #   10. Use 'fspec checkpoint' (short form) for AI efficiency - less verbose, faster to type in conversations
  #   11. Ask user interactively when dirty working directory detected - present options (commit first, stash and restore, force restore with merge) and explain risks of each approach
  #
  # EXAMPLES:
  #   1. User runs 'fspec update-work-unit-status GIT-002 implementing', system creates checkpoint 'GIT-002-auto-testing' before transition
  #   2. User runs 'fspec create-checkpoint GIT-002 before-refactor', system creates named checkpoint for experimentation
  #   3. User creates checkpoint 'baseline', tries approach A (fails), restores 'baseline', tries approach B (succeeds)
  #   4. User restores checkpoint causing conflicts, AI receives system-reminder, reads conflicted files, resolves with Edit tool, tests run automatically, restoration completes when tests pass
  #   5. User runs 'fspec list-checkpoints GIT-002', sees both automatic and manual checkpoints with timestamps
  #   6. User runs 'fspec cleanup-checkpoints GIT-002 --keep-last 5', system deletes old checkpoints keeping 5 most recent
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should automatic checkpoints be hidden by default in 'fspec list-checkpoints GIT-002' (require --show-auto flag to display)?
  #   A: true
  #
  #   Q: Command naming preference: 'fspec checkpoint GIT-002 name' (short) or 'fspec create-checkpoint GIT-002 name' (explicit)?
  #   A: true
  #
  #   Q: Should restoration fail if working directory is dirty (uncommitted changes) even without conflicts?
  #   A: true
  #
  #   Q: Should test command be configurable (package.json 'test' script vs hardcoded 'npm test')?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. fspec does NOT run tests - it emits system-reminder to AI agent that tests must be run. AI chooses test command (npm test, vitest, etc). This is about the system-reminder message content only.
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to automatically save my work at each workflow transition
    So that I can safely experiment without losing progress and recover from mistakes

  Scenario: Automatic checkpoint created on workflow state transition
    Given I have a work unit "GIT-002" in "testing" status
    And I have uncommitted changes in my working directory
    When I run "fspec update-work-unit-status GIT-002 implementing"
    Then a checkpoint "GIT-002-auto-testing" should be created automatically
    And the checkpoint should capture all file changes including untracked files
    And the checkpoint should be stored as a git stash
    And the work unit status should change to "implementing"

  Scenario: Create manual checkpoint for experimentation
    Given I have a work unit "GIT-002" in "implementing" status
    And I have uncommitted changes in my working directory
    When I run "fspec checkpoint GIT-002 before-refactor"
    Then a checkpoint "before-refactor" should be created
    And the checkpoint should be stored as a git stash with message format "fspec-checkpoint:GIT-002:before-refactor:{timestamp}"
    And all file changes should be captured including untracked files

  Scenario: Multiple experiments from same baseline checkpoint
    Given I have created a checkpoint "baseline" for work unit "GIT-002"
    When I restore checkpoint "baseline"
    And I implement approach A which fails
    And I restore checkpoint "baseline" again
    And I implement approach B which succeeds
    Then I should be able to compare both approaches
    And the "baseline" checkpoint should still exist for future experiments

  Scenario: AI-assisted conflict resolution during checkpoint restoration
    Given I have a checkpoint "previous-state" for work unit "GIT-002"
    And I have made conflicting changes in my working directory
    When I run "fspec restore-checkpoint GIT-002 previous-state"
    Then git merge conflicts should be detected
    And a system-reminder should be emitted to the AI with conflicted file paths
    And the AI should use Read and Edit tools to resolve conflicts
    And the system should prompt "Run tests to validate conflict resolution"
    And restoration should complete only after tests pass

  Scenario: List all checkpoints with visual indicators
    Given I have automatic checkpoints "GIT-002-auto-testing" and "GIT-002-auto-implementing"
    And I have manual checkpoints "baseline" and "before-refactor"
    When I run "fspec list-checkpoints GIT-002"
    Then I should see all checkpoints with clear visual indicators
    And automatic checkpoints should show 🤖 emoji
    And manual checkpoints should show 📌 emoji
    And each checkpoint should display its timestamp

  Scenario: Cleanup old checkpoints keeping most recent
    Given I have 10 checkpoints for work unit "GIT-002"
    When I run "fspec cleanup-checkpoints GIT-002 --keep-last 5"
    Then the 5 oldest checkpoints should be deleted
    And the 5 most recent checkpoints should be preserved
    And I should see a summary of deleted and preserved checkpoints

  Scenario: Interactive prompt when restoring with dirty working directory
    Given I have a checkpoint "safe-state" for work unit "GIT-002"
    And I have uncommitted changes in my working directory
    When I run "fspec restore-checkpoint GIT-002 safe-state"
    Then I should be prompted with options:
      | Option                    | Risk Level |
      | Commit changes first      | Low        |
      | Stash changes and restore | Medium     |
      | Force restore with merge  | High       |
    And each option should explain the risks
    And restoration should proceed based on user choice
