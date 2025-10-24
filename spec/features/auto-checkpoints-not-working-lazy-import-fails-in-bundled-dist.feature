@critical
@cli
@workflow-automation
@checkpoint
@git
@bug-fix
@BUG-043
Feature: Auto-checkpoints not working - lazy import fails in bundled dist

  """
  Solution: Change from lazy import to static import at top of file. Import statement: import * as gitCheckpoint from '../utils/git-checkpoint'. This will be bundled correctly by Vite into the single index.js file.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Auto-checkpoints must be created before EVERY state transition (except from backlog)
  #   2. Lazy imports must work after Vite bundling into single dist/index.js file
  #
  # EXAMPLES:
  #   1. User transitions from backlog to specifying with dirty working directory, sees '🤖 Auto-checkpoint created' message
  #   2. User transitions from specifying to testing with uncommitted changes, checkpoint saved before transition
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec
    I want to have automatic checkpoints created during work unit transitions
    So that I can safely experiment and recover from mistakes

  Scenario: Auto-checkpoint created on state transition with uncommitted changes
    Given a work unit in 'backlog' status with uncommitted file changes in the working directory
    When the user runs 'fspec update-work-unit-status <id> specifying'
    Then the command should display '🤖 Auto-checkpoint: "<id>-auto-backlog" created before transition'
    And the checkpoint should be retrievable with 'fspec list-checkpoints <id>'

