@init
@agent-compatibility
@initialization
@AGENT-014
Feature: Test Codex agent compatibility
  """
  Exercise codelet-fspec-core dispatcher init with an isolated HOME; production init is shared with the CLI and already implements AGENT-018.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Codex initialization follows AGENT-018: install the home prompt and spec/AGENTS.md, not obsolete project commands or CODEX.md.
  #
  # EXAMPLES:
  #   1. An isolated HOME receives .codex/prompts/fspec.md and the project receives ACDD instructions and a persisted codex selection.
  #
  # ========================================
  Background: User Story
    As a Codex user I want to initialize fspec safely So that I can activate its ACDD workflow

  Scenario: Initialize Codex with usable ACDD instructions
    Given an empty project and an isolated HOME
    When I initialize fspec for codex
    Then the home prompt and project ACDD instructions are installed
    And the codex selection and activation command are reported correctly
    And obsolete project-local Codex instructions are not created
