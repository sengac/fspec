@done
@GIT-034
Feature: AI System Reminder Includes Isolation State and Worktree Path

  """
  buildEnvironmentReminder in src/utils/system-reminder.ts needs IsolationContext parameter, IsolationStateChange chunk data must be passed through to reminder generation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When session is isolated, environment system-reminder MUST include Isolation: ACTIVE, Worktree path, and Base commit
  #   2. When session is non-isolated, environment system-reminder MUST NOT include isolation fields (current behavior)
  #   3. Worktree path MUST be displayed as relative path from project root (e.g., .fspec/worktrees/abc123/)
  #   4. Base commit MUST be displayed as short SHA (first 8 characters)
  #
  # EXAMPLES:
  #   1. AI in isolated session sees environment reminder with 'Isolation: ACTIVE', 'Worktree: .fspec/worktrees/abc-123/', 'Base commit: 7a8b9c0d'
  #   2. AI in non-isolated session sees environment reminder without any isolation fields (same as current)
  #   3. User asks 'where are my changes?', AI can respond 'Your changes are in the isolated worktree at .fspec/worktrees/abc-123/. You'll need to merge them to apply to main project.'
  #   4. AI completes task and can advise user: 'I've made all the changes. Since this is an isolated session, use /merge to apply changes to main project or /discard to abandon them.'
  #
  # ========================================

  Background: User Story
    As a AI agent in an isolated session
    I want to see isolation state in my environment context
    So that explain to users that changes are in a worktree and require merging

  @isolated @system-reminder
  Scenario: Isolated session environment reminder includes isolation fields
    Given a session is created with isolated mode enabled
    And the worktree is at ".fspec/worktrees/abc-123/"
    And the base commit SHA is "7a8b9c0def123456"
    When the environment system-reminder is generated
    Then the reminder should contain "Isolation: ACTIVE"
    And the reminder should contain "Worktree: .fspec/worktrees/abc-123/"
    And the reminder should contain "Base commit: 7a8b9c0d"

  @non-isolated @system-reminder
  Scenario: Non-isolated session environment reminder excludes isolation fields
    Given a session is created with isolated mode disabled
    When the environment system-reminder is generated
    Then the reminder should NOT contain "Isolation:"
    And the reminder should NOT contain "Worktree:"
    And the reminder should NOT contain "Base commit:"
    And the reminder should contain "Working directory:"

  @isolated @user-interaction
  Scenario: AI can explain worktree location to user
    Given a session is created with isolated mode enabled
    And the worktree is at ".fspec/worktrees/abc-123/"
    And the environment reminder has been injected into context
    When the user asks "where are my changes?"
    Then the AI has context to respond with the worktree path
    And the AI has context to explain that merging is required

  @isolated @user-interaction
  Scenario: AI can advise user about merge/discard options
    Given a session is created with isolated mode enabled
    And the AI has completed making changes
    And the environment reminder includes isolation state
    When the AI responds to task completion
    Then the AI has context to suggest "/merge" to apply changes
    And the AI has context to suggest "/discard" to abandon changes
