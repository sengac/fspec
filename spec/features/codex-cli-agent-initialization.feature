@init
@agent-compatibility
@initialization
@AGENT-017
Feature: Test Codex CLI agent compatibility
  """
  Exercise the common core init dispatcher; use isolated HOME and filesystem state, without invoking a real external agent.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Codex CLI initialization follows AGENT-018: install the shared home prompt and spec/AGENTS.md with Codex CLI activation instructions.
  #
  # EXAMPLES:
  #   1. Reinitializing codex-cli preserves work units and leaves unrelated home prompts unchanged.
  #
  # ========================================
  Background: User Story
    As a Codex CLI user I want to initialize and reinitialize fspec safely So that my project work is preserved

  Scenario: Reinitialize Codex CLI without losing project work
    Given an initialized Codex CLI project with work units and unrelated home prompts
    When I initialize fspec for codex-cli again
    Then the home prompt and project ACDD instructions remain usable
    And existing work units configuration and unrelated home prompts are preserved
    And the Codex CLI activation command and persisted selection are correct
