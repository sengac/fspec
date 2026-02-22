@feature-management
@cli
@done
@status-display
@tui
@GIT-032
Feature: SessionHeader missing isIsolated prop - ISOLATED badge never shows
  """
  AgentView must import useIsIsolated from sessionStore and pass isIsolated prop to SessionHeader. SessionHeader already has rendering logic for [ISOLATED] badge (lines 170-172). Integration point: AgentView.tsx lines 7731-7747.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AgentView must read isIsolated from sessionStore and pass it to SessionHeader
  #   2. SessionHeader must display green [ISOLATED] badge when isIsolated=true
  #   3. SessionHeader must NOT display [ISOLATED] badge when isIsolated=false
  #
  # EXAMPLES:
  #   1. User creates isolated session, header shows [ISOLATED] badge in green next to model name
  #   2. User creates normal session, header does NOT show [ISOLATED] badge
  #
  # ========================================
  Background: User Story
    As a user
    I want to see my isolated session status in the header
    So that know that my changes are isolated in a git worktree

  Scenario: Display ISOLATED badge for isolated session
    Given I have created an isolated session
    When I view the session header
    Then I should see the [ISOLATED] badge in green next to the model name

  Scenario: Do not display ISOLATED badge for normal session
    Given I have created a normal (non-isolated) session
    When I view the session header
    Then I should NOT see the [ISOLATED] badge
