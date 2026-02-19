@TUI-067
Feature: CreateSessionDialog shows wrong text when pressing Enter on work unit

  """
  CreateSessionDialog accepts optional workUnit prop. When provided, displays work-unit-aware title and description. When absent, displays generic 'not linked to any task' text.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When triggered from Enter on a work unit, dialog must show work-unit-aware text
  #   2. When triggered from Shift+Right navigation (no work unit), dialog shows generic unattached text
  #
  # EXAMPLES:
  #   1. User presses Enter on story AUTH-001, dialog shows 'Work on AUTH-001?' and 'Start an AI session for this task'
  #   2. User presses Shift+Right past last session, dialog shows 'Start New Agent?' and 'Begin a fresh AI conversation, not linked to any task'
  #
  # ========================================

  Background: User Story
    As a user working on a task
    I want to see context-appropriate dialog text when starting an AI session
    So that I understand whether the session will be linked to my selected task

  Scenario: Show work-unit-aware text when pressing Enter on a story
    Given I am viewing the board with work unit "AUTH-001" titled "User Login"
    When I press Enter on the story card
    Then the dialog title should be "Work on AUTH-001?"
    And the dialog description should be "Start an AI session for this task"

  Scenario: Show generic unattached text when using Shift+Right navigation
    Given I am in the agent view with no work unit selected
    When I press Shift+Right past the last session
    Then the dialog title should be "Start New Agent?"
    And the dialog description should be "Begin a fresh AI conversation, not linked to any task."
