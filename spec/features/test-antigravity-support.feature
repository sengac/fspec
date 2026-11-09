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
    When I dispatch init against that project directory with agent list ['antigravity']
    Then the dispatcher returns success=true
    And the filesInstalled array is 'spec/ANTIGRAVITY.md' and '.antigravity/commands/fspec.md'
    And spec/ANTIGRAVITY.md and .antigravity/commands/fspec.md exist in the project directory
    And the activation message is 'Add .antigravity/commands/ to your Antigravity configuration to activate'

  Scenario: The project config records the Antigravity agent
    Given I have initialized fspec with the "antigravity" agent
    When I read spec/fspec-config.json
    Then its agent field is 'antigravity'
