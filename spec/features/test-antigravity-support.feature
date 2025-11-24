@cli
@agent-support
@done
@AGENT-020
Feature: Test Antigravity Support
  """
  Follow standard fspec agent integration patterns
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. fspec init should detect or support manual selection of Antigravity agent
  #   2. Standard fspec commands should function correctly in the Antigravity environment
  #
  # EXAMPLES:
  #   1. User runs 'fspec init --agent=antigravity' and sees success message
  #   2. User runs 'fspec status' and sees correct output in Antigravity terminal
  #
  # ========================================
  Background: User Story
    As a Antigravity User
    I want to initialize and use fspec with the Antigravity agent
    So that I can leverage fspec's project management capabilities within my Antigravity workflow

  Scenario: Initialize fspec with Antigravity agent
    Given I am in a new project directory
    When I run "fspec init --agent=antigravity"
    Then the exit code should be 0
    And the output should contain "Initialized fspec for Antigravity"
    And a ".fspec" directory should exist

  Scenario: Verify fspec status in Antigravity environment
    Given I have initialized fspec with the "antigravity" agent
    When I run "fspec status"
    Then the exit code should be 0
    And the output should contain "Agent: Antigravity"
