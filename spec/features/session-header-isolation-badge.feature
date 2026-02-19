@session-header
@tui-component
@GIT-029
Feature: Session Header Isolation Badge

  """
  SessionHeader displays [ISOLATED] badge when session is isolated.
  Badge is green to match existing badge patterns: [R], [V], [DEBUG], [T:Med].
  """

  Background: User Story
    As a developer
    I want to see an [ISOLATED] badge in the SessionHeader
    So that I know when I'm working in an isolated git worktree

  @tui
  Scenario: SessionHeader displays ISOLATED badge for isolated session
    Given I have created an isolated session
    When the SessionHeader renders
    Then I should see an "[ISOLATED]" badge in green

  @tui
  Scenario: SessionHeader does not display ISOLATED badge for normal session
    Given I have created a normal (non-isolated) session
    When the SessionHeader renders
    Then I should not see an "[ISOLATED]" badge
